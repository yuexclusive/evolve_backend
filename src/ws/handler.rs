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

pub enum NotifyType<'a> {
    UpdateSession {
        session: &'a mut actix_ws::Session,
        name: &'a str,
        room: &'a str,
    },

    List {
        chat_server: &'a ChatServerHandle,
        session: &'a mut actix_ws::Session,
        session_id: &'a str,
    },

    JoinRoom {
        chat_server: &'a ChatServerHandle,
        session_id: &'a str,
        name: &'a str,
        room: &'a str,
    },
    QuitRoom {
        chat_server: &'a ChatServerHandle,
        session_id: &'a str,
        name: &'a str,
        room: &'a str,
    },
    UpdateName {
        chat_server: &'a ChatServerHandle,
        session_id: &'a str,
        name: &'a str,
        old_name: &'a str,
    },
    Message {
        chat_server: &'a ChatServerHandle,
        session_id: &'a str,
        name: &'a str,
        room: &'a str,
        msg: &'a str,
    },
}

const UPDATE_SESSION_PRE: &str = "update_session:";
const LIST_PRE: &str = "list:";
const JOIN_ROOM_PRE: &str = "join_room:";
const QUIT_ROOM_PRE: &str = "quit_room:";
const UPDATE_NAME_PRE: &str = "update_name:";
const MESSAGE_PRE: &str = "message:";

#[derive(Serialize)]
struct UpdateSession<'a> {
    pub room: &'a str,
    pub name: &'a str,
}

#[derive(Serialize)]
struct RoomChange<'a> {
    pub session_id: &'a str,
    pub name: &'a str,
    pub room: &'a str,
}

#[derive(Serialize)]
struct UpdateName<'a> {
    pub session_id: &'a str,
    pub name: &'a str,
    pub old_name: &'a str,
}

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

async fn notify(ty: NotifyType<'_>) {
    match ty {
        NotifyType::UpdateSession {
            session,
            name,
            room,
        } => {
            session
                .text(format!(
                    "{UPDATE_SESSION_PRE}{}",
                    serde_json::to_string(&UpdateSession { room, name }).unwrap()
                ))
                .await
                .unwrap();
        }

        NotifyType::List {
            chat_server,
            session,
            session_id,
        } => {
            let rooms = chat_server.get_rooms_by_session_id(session_id).await;
            session
                .text(format!(
                    "{LIST_PRE}{}",
                    serde_json::to_string(&rooms).unwrap()
                ))
                .await
                .unwrap();
        }

        NotifyType::JoinRoom {
            chat_server,
            name,
            session_id,
            room,
        } => {
            let rooms = chat_server.get_rooms_by_room_id(room).await;
            for (r, sessions) in rooms.0.iter() {
                for (to_id, _) in sessions {
                    chat_server
                        .send_system_message(
                            r,
                            to_id,
                            &format!(
                                "{JOIN_ROOM_PRE}{}",
                                serde_json::to_string(&RoomChange {
                                    session_id,
                                    room,
                                    name
                                })
                                .unwrap()
                            ),
                        )
                        .await;
                }
            }
        }
        NotifyType::QuitRoom {
            chat_server,
            session_id,
            name,
            room,
        } => {
            let rooms = chat_server.get_rooms_by_room_id(room).await;
            for (r, sessions) in rooms.0.iter() {
                for (to_id, _) in sessions {
                    chat_server
                        .send_system_message(
                            r,
                            to_id,
                            &format!(
                                "{QUIT_ROOM_PRE}{}",
                                serde_json::to_string(&RoomChange {
                                    session_id,
                                    room,
                                    name
                                })
                                .unwrap()
                            ),
                        )
                        .await;
                }
            }
        }
        NotifyType::UpdateName {
            chat_server,
            session_id,
            name,
            old_name,
        } => {
            let rooms = chat_server.get_rooms_by_session_id(session_id).await;
            for (r, sessions) in rooms.0.iter() {
                for (to_id, _) in sessions {
                    chat_server
                        .send_system_message(
                            r,
                            to_id,
                            &format!(
                                "{UPDATE_NAME_PRE}{}",
                                serde_json::to_string(&UpdateName {
                                    session_id,
                                    name,
                                    old_name
                                })
                                .unwrap()
                            ),
                        )
                        .await;
                }
            }
        }
        NotifyType::Message {
            chat_server,
            session_id,
            name,
            room,
            msg,
        } => {
            chat_server
                .send_message(
                    room.to_string(),
                    session_id.to_string(),
                    MessageContent::format(room, session_id, name, msg),
                )
                .await
        }
    }
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

    notify(NotifyType::UpdateSession {
        session: &mut session,
        name: &name,
        room: &room,
    })
    .await;

    notify(NotifyType::JoinRoom {
        chat_server: &chat_server,
        session_id: &session_id,
        name: &name,
        room: &DEFAULT_ROOM,
    })
    .await;

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

    for room in rooms {
        notify(NotifyType::QuitRoom {
            chat_server: &chat_server,
            session_id: &session_id,
            name: &name,
            room: &room,
        })
        .await;
    }

    // attempt to close connection gracefully
    let _ = session.close(close_reason).await;
}

async fn process_text_msg(
    chat_server: &ChatServerHandle,
    mut session: &mut actix_ws::Session,
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
                notify(NotifyType::List {
                    chat_server: &chat_server,
                    session_id: &session_id,
                    session,
                })
                .await;
            }

            "/join" => match cmd_args.next() {
                Some(r) => {
                    chat_server.join_room(session_id.clone(), r).await;
                    *room = r.to_string();

                    notify(NotifyType::UpdateSession {
                        session: &mut session,
                        name: &name,
                        room: &room,
                    })
                    .await;

                    notify(NotifyType::JoinRoom {
                        chat_server: &chat_server,
                        session_id: &session_id,
                        name: &name,
                        room: &room,
                    })
                    .await;
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

                    notify(NotifyType::QuitRoom {
                        chat_server: &chat_server,
                        session_id: &session_id,
                        name: &name,
                        room: &room,
                    })
                    .await;

                    chat_server.quit_room(session_id.clone(), r).await;
                    *room = DEFAULT_ROOM.to_string();

                    notify(NotifyType::UpdateSession {
                        session: &mut session,
                        name: &name,
                        room: &room,
                    })
                    .await;
                }

                None => {
                    session.text("!!! room name is required").await.unwrap();
                }
            },

            "/name" => match cmd_args.next() {
                Some(new_name) => {
                    let old_name = name.clone();
                    *name = new_name.to_string();
                    chat_server
                        .change_name(session_id.clone(), new_name.clone())
                        .await;

                    notify(NotifyType::UpdateSession {
                        session: &mut session,
                        name: &name,
                        room: &room,
                    })
                    .await;

                    notify(NotifyType::UpdateName {
                        chat_server: &chat_server,
                        session_id: &session_id,
                        name: &name,
                        old_name: &old_name,
                    })
                    .await;
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
        notify(NotifyType::Message {
            chat_server: &chat_server,
            session_id: &session_id,
            name: &name,
            room: &room,
            msg: &msg,
        })
        .await;
    }
}
