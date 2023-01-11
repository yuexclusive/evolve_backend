#![cfg(feature = "ws")]

use std::time::{Duration, Instant};

use actix_ws::Message;
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use tokio::{pin, sync::mpsc};

use serde::Serialize;

use super::server::{ChatServerHandle, SessionID, DEFAULT_ROOM};
use utilities::datetime::FormatDateTime;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

const UPDATE_ROOMS_PRE: &str = "update_rooms:";
const UPDATE_SESSION_PRE: &str = "update_session:";
const MESSAGE_PRE: &str = "message:";

#[derive(Serialize)]
pub struct MessageContent<'a> {
    pub id: u128,
    pub room: &'a str,
    pub from_id: &'a str,
    pub from_name: &'a str,
    pub content: &'a str,
    pub time: String,
}

impl<'a> MessageContent<'a> {
    pub fn format(room: &'a str, from_id: &'a str, from_name: &'a str, content: &'a str) -> String {
        let obj = Self {
            id: uuid::Uuid::new_v4().as_u128(),
            room,
            from_id,
            from_name,
            content,
            time: chrono::Utc::now().to_default(),
        };
        let str = serde_json::to_string(&obj).unwrap();
        format!("{MESSAGE_PRE}{}", str)
    }
}

#[derive(Serialize)]
struct UpdateSession<'a> {
    pub room: &'a str,
    pub name: &'a str,
}

async fn update_session(session: &mut actix_ws::Session, room: &str, name: &str) {
    let obj = UpdateSession { room, name };
    let str = serde_json::to_string(&obj).unwrap();
    let content = format!("{UPDATE_SESSION_PRE}{}", str);
    session.text(content).await.unwrap();
}

async fn notify_update_rooms(chat_server: &ChatServerHandle, room: Option<&str>, session_id: &str) {
    let rooms = chat_server.get_rooms().await;
    let mut target_rooms = vec![];
    match room {
        Some(r) => target_rooms.push(r),
        None => {
            if let Some(rooms) = rooms.session_room_map.get(session_id) {
                for (s, _) in rooms {
                    target_rooms.push(s)
                }
            }
        }
    }

    for r in target_rooms {
        if let Some(sessions) = rooms.rooms.get(r) {
            for (to_id, _) in sessions {
                chat_server
                    .send_system_message(
                        r,
                        to_id,
                        &format!(
                            "{UPDATE_ROOMS_PRE}{}",
                            serde_json::to_string(&rooms.get_by_session(to_id)).unwrap()
                        ),
                    )
                    .await;
            }
        }
    }
}

async fn notify_update_rooms_to_self(
    chat_server: &ChatServerHandle,
    session: &mut actix_ws::Session,
    session_id: &str,
) {
    let rooms = chat_server.get_rooms_by_session_id(session_id).await;
    session
        .text(format!(
            "{UPDATE_ROOMS_PRE}{}",
            serde_json::to_string(&rooms).unwrap()
        ))
        .await
        .unwrap();
}

async fn send_message(
    chat_server: &ChatServerHandle,
    room: &str,
    session_id: &str,
    name: &str,
    msg: &str,
) {
    chat_server
        .send_message(
            room.to_string(),
            session_id.to_string(),
            MessageContent::format(room, session_id, name, msg),
        )
        .await
}

/// Echo text & binary messages received from the client, respond to ping messages, and monitor
/// connection health to detect network issues and free up resources.
pub async fn chat_ws(
    session_id: String,
    session_name: String,
    chat_server: ChatServerHandle,
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
) {
    let mut room = DEFAULT_ROOM.to_string();
    let mut name = session_name.clone();
    let mut last_heartbeat = Instant::now();
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);

    let (conn_tx, mut conn_rx) = mpsc::unbounded_channel();

    // unwrap: chat server is not dropped before the HTTP server
    let conn_id = chat_server
        .connect(conn_tx, session_id.clone(), session_name)
        .await;

    update_session(&mut session, &room, &name).await;
    notify_update_rooms(&chat_server, Some(DEFAULT_ROOM), &session_id).await;

    let close_reason = loop {
        // most of the futures we process need to be stack-pinned to work with select()

        let tick = interval.tick();
        pin!(tick);

        let msg_rx = conn_rx.recv();
        pin!(msg_rx);

        // TODO: nested select is pretty gross for readability on the match
        let messages = select(msg_stream.next(), msg_rx);
        pin!(messages);

        match select(messages, tick).await {
            // commands & messages received from client
            Either::Left((Either::Left((Some(Ok(msg)), _)), _)) => {
                match msg {
                    Message::Ping(bytes) => {
                        last_heartbeat = Instant::now();
                        // unwrap:
                        session.pong(&bytes).await.unwrap();
                    }

                    Message::Pong(_) => {
                        last_heartbeat = Instant::now();
                    }

                    Message::Text(text) => {
                        process_text_msg(
                            &chat_server,
                            &mut session,
                            &text,
                            session_id.clone(),
                            &mut name,
                            &mut room,
                        )
                        .await;
                    }

                    Message::Binary(_bin) => {
                        log::warn!("unexpected binary message");
                    }

                    Message::Close(reason) => break reason,

                    _ => {
                        break None;
                    }
                }
            }

            // client WebSocket stream error
            Either::Left((Either::Left((Some(Err(err)), _)), _)) => {
                log::error!("{}", err);
                break None;
            }

            // client WebSocket stream ended
            Either::Left((Either::Left((None, _)), _)) => break None,

            // chat messages received from other room participants
            Either::Left((Either::Right((Some(chat_msg), _)), _)) => {
                session.text(chat_msg).await.unwrap();
            }

            // all connection's message senders were dropped
            Either::Left((Either::Right((None, _)), _)) => {
                unreachable!(
                    "all connection message senders were dropped; chat server may have panicked"
                )
            }

            // heartbeat internal tick
            Either::Right((_inst, _)) => {
                // if no heartbeat ping/pong received recently, close the connection
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    log::info!(
                        "client has not sent heartbeat in over {CLIENT_TIMEOUT:?}; disconnecting"
                    );
                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"ping").await;
                // session.text(chrono::Utc::now().to_rfc3339()).await.unwrap();
            }
        };
    };

    let rooms = chat_server.disconnect(conn_id).await;

    for room in rooms.iter() {
        notify_update_rooms(&chat_server, Some(room), &session_id).await;
    }

    // attempt to close connection gracefully
    let _ = session.close(close_reason).await;
}

async fn process_text_msg(
    chat_server: &ChatServerHandle,
    session: &mut actix_ws::Session,
    text: &str,
    session_id: SessionID,
    name: &mut String,
    room: &mut String,
) {
    // strip leading and trailing whitespace (spaces, newlines, etc.)
    let msg = text.trim();

    // we check for /<cmd> type of messages
    if msg.starts_with('/') {
        let mut cmd_args = msg.splitn(2, ' ');

        // unwrap: we have guaranteed non-zero string length already
        match cmd_args.next().unwrap() {
            "/list" => {
                notify_update_rooms_to_self(chat_server, session, &session_id).await;
            }

            "/join" => match cmd_args.next() {
                Some(r) => {
                    chat_server.join_room(session_id.clone(), r).await;
                    *room = r.to_string();
                    update_session(session, &room, &name).await;
                    notify_update_rooms(chat_server, Some(r), &session_id).await;
                }

                None => {
                    session.text("!!! room name is required").await.unwrap();
                }
            },

            "/quit" => match cmd_args.next() {
                Some(r) => {
                    if r == DEFAULT_ROOM {
                        session
                            .text(&format!("!!! you can not quit default room: {}", r))
                            .await
                            .unwrap();
                        return;
                    }
                    chat_server.quit_room(session_id.clone(), r).await;
                    *room = DEFAULT_ROOM.to_string();

                    update_session(session, &room, &name).await;
                    notify_update_rooms(chat_server, Some(r), &session_id).await;
                    notify_update_rooms_to_self(chat_server, session, &session_id).await;
                }

                None => {
                    session.text("!!! room name is required").await.unwrap();
                }
            },

            "/name" => match cmd_args.next() {
                Some(new_name) => {
                    *name = new_name.to_string();
                    chat_server
                        .change_name(session_id.clone(), new_name.clone())
                        .await;

                    update_session(session, &room, &name).await;
                    notify_update_rooms(chat_server, None, &session_id).await;
                }
                None => {
                    session.text("!!! name is required").await.unwrap();
                }
            },

            _ => {
                session
                    .text(format!("!!! unknown command: {msg}"))
                    .await
                    .unwrap();
            }
        }
    } else {
        send_message(chat_server, room, &session_id, name, msg).await;
    }
}
