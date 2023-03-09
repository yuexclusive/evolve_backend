#![cfg(feature = "ws")]
use crate::ws::hub::{
    self, ClientMessageForHub, MessageForHub, RetrieveRroomsReqType, RoomChangeForHub,
    RoomChangeType, SystemMessageForHub,
};
use futures_util::future::{select, Either};
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::Arc,
};
use tokio::{
    pin,
    sync::{
        mpsc::{self, UnboundedReceiver},
        oneshot, Mutex,
    },
};

use super::hub::{RetrieveRroomsReq, UpdateRooms};
use utilities::{business_error, error::BasicResult};

// use crate::{SessionID, Msg, RoomId};

/// Connection ID.
pub type SessionID = String;

/// Room ID.
pub type RoomID = String;

/// Message sent to a room/client.
pub type Msg = String;

/// A command received by the [`ChatServer`].
#[derive(Debug)]
enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<Msg>,
        res_tx: oneshot::Sender<SessionID>,
        id: String,
        name: String,
    },

    Disconnect {
        session_id: SessionID,
        res_tx: oneshot::Sender<Vec<RoomID>>,
    },

    GetRoomsBySessionID {
        session_id: SessionID,
        res_tx: oneshot::Sender<UpdateRooms>,
    },

    GetRoomsByRoomID {
        room_id: String,
        res_tx: oneshot::Sender<UpdateRooms>,
    },

    Join {
        conn: SessionID,
        room: RoomID,
        res_tx: oneshot::Sender<()>,
    },

    Quit {
        conn: SessionID,
        room: RoomID,
        res_tx: oneshot::Sender<()>,
    },

    Name {
        name: String,
        conn: SessionID,
        res_tx: oneshot::Sender<()>,
    },

    Message {
        room: String,
        id: SessionID,
        msg: Msg,
        res_tx: oneshot::Sender<()>,
    },

    SystemMessage {
        room: String,
        to_id: SessionID,
        msg: Msg,
        res_tx: oneshot::Sender<()>,
    },
}

/// A multi-room chat server.
///
/// Contains the logic of how connections chat with each other plus room management.
///
/// Call and spawn [`run`](Self::run) to start processing commands.
#[derive(Debug)]
pub struct ChatServer<H>
where
    H: hub::Hub,
{
    /// Map of connection IDs to their message receivers.
    sessions: HashMap<SessionID, mpsc::UnboundedSender<Msg>>,

    /// Map of room name to participant IDs in that room.
    rooms: Arc<Mutex<HashMap<RoomID, HashSet<SessionID>>>>,

    // /// Tracks total number of historical connections established.
    // visitor_count: Arc<AtomicUsize>,
    /// Command receiver.
    cmd_rx: Arc<Mutex<UnboundedReceiver<Command>>>,

    hub: Arc<H>,
}

pub const DEFAULT_ROOM: &str = "main";

impl<H> ChatServer<H>
where
    H: hub::Hub,
{
    pub fn new(
        hub: Arc<H>,
    ) -> (
        Self,
        ChatServerHandle,
        Arc<Mutex<HashMap<RoomID, HashSet<SessionID>>>>,
    ) {
        let rooms = Arc::new(Mutex::new(HashMap::<RoomID, HashSet<SessionID>>::new()));
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        (
            Self {
                sessions: HashMap::new(),
                rooms: rooms.clone(),
                hub: hub,
                // visitor_count: Arc::new(AtomicUsize::new(0)),
                cmd_rx: Arc::new(Mutex::new(cmd_rx)),
            },
            ChatServerHandle { cmd_tx },
            rooms,
        )
    }

    /// Send message to users in a room.
    ///
    /// `skip` is used to prevent messages triggered by a connection also being received by it.
    async fn send_message(&self, room: String, skip: Option<SessionID>, msg: impl Into<String>) {
        let msg = msg.into();
        if let Some(sessions) = self.rooms.lock().await.get(&room) {
            for conn_id in sessions {
                if let Some(skip) = &skip {
                    if conn_id != skip {
                        if let Some(tx) = self.sessions.get(conn_id) {
                            tx.send(msg.clone()).unwrap()
                        }
                    }
                }
            }
        }
    }

    /// send_system_message
    async fn send_system_message(&self, to_id: String, msg: impl Into<String>) {
        let msg = msg.into();
        if let Some(conn) = self.sessions.get(&to_id) {
            conn.send(msg).unwrap()
        }
    }

    /// Register new session and assign unique ID to this session
    async fn connect(
        &mut self,
        tx: mpsc::UnboundedSender<Msg>,
        id: String,
        name: String,
    ) -> BasicResult<SessionID> {
        self.hub.open_channel(DEFAULT_ROOM).await?;

        // register session with random connection IDF
        self.sessions.insert(id.clone(), tx);

        // auto join session to main room
        self.rooms
            .lock()
            .await
            .entry(DEFAULT_ROOM.to_owned())
            .or_insert_with(HashSet::new)
            .insert(id.clone());

        self.hub
            .change_rooms(RoomChangeForHub {
                id: id.clone(),
                name: Some(name),
                room: DEFAULT_ROOM.to_string(),
                r#type: RoomChangeType::Add,
            })
            .await?;
        Ok(id)
    }

    /// Unregister connection from room map and broadcast disconnection message.
    async fn disconnect(&mut self, conn_id: SessionID) -> BasicResult<Vec<RoomID>> {
        let mut res = Vec::new();
        // remove sender
        if self.sessions.remove(&conn_id).is_some() {
            let mut rooms = self.rooms.lock().await;
            // remove session from all rooms
            for (room, sessions) in rooms.iter_mut() {
                if sessions.remove(&conn_id) {
                    self.hub
                        .change_rooms(RoomChangeForHub {
                            id: conn_id.clone(),
                            name: None,
                            room: room.clone(),
                            r#type: RoomChangeType::Del,
                        })
                        .await?;

                    res.push(room.clone())
                }
            }

            for room in res.iter() {
                if rooms
                    .get(room)
                    .ok_or(business_error!("can not find room"))?
                    .len()
                    == 0
                {
                    rooms.remove(room);
                    self.hub.close_channel(room).await?;
                }
            }
        }
        Ok(res)
    }

    // /// Returns list of created room names.
    // async fn get_rooms(&mut self) -> BasicResult<ChatServerForHub> {
    //     self.hub.get_rooms().await
    // }

    /// Returns list of created room names.
    async fn get_rooms_by_session_id(&mut self, session_id: SessionID) -> BasicResult<UpdateRooms> {
        self.hub
            .retrieve_rooms(RetrieveRroomsReq::new(
                RetrieveRroomsReqType::SessionID,
                session_id,
            ))
            .await
    }

    /// Returns list of created room names.
    async fn get_rooms_by_room_id(&mut self, room_id: String) -> BasicResult<UpdateRooms> {
        self.hub
            .retrieve_rooms(RetrieveRroomsReq::new(
                RetrieveRroomsReqType::RoomID,
                room_id,
            ))
            .await
    }

    /// Join room, send disconnect message to old room send join message to new room.
    async fn join_room(&mut self, session_id: SessionID, room: String) -> BasicResult<()> {
        self.hub.open_channel(&room).await?;

        self.rooms
            .lock()
            .await
            .entry(room.clone())
            .or_insert_with(HashSet::new)
            .insert(session_id.clone());

        self.hub
            .change_rooms(RoomChangeForHub {
                id: session_id.clone(),
                name: None,
                room: room.clone(),
                r#type: RoomChangeType::Add,
            })
            .await?;
        Ok(())
    }

    async fn quit_room(&mut self, session_id: SessionID, room: String) -> BasicResult<()> {
        if let Some(v) = self.rooms.lock().await.get_mut(&room) {
            if v.remove(&session_id) {
                if v.len() == 0 {
                    self.hub.close_channel(&room).await?;
                }
            }
        }

        self.hub
            .change_rooms(RoomChangeForHub {
                id: session_id.clone(),
                name: None,
                room: room.clone(),
                r#type: RoomChangeType::Del,
            })
            .await?;
        
        Ok(())
    }

    async fn change_name(&mut self, session_id: SessionID, name: String) {
        self.hub
            .change_rooms(RoomChangeForHub {
                id: session_id.clone(),
                name: Some(name),
                room: "".to_string(),
                r#type: RoomChangeType::NameChange,
            })
            .await
            .unwrap();
    }

    pub async fn run(mut self) -> io::Result<()> {
        'outer: loop {
            let cmd_rx = self.cmd_rx.clone();
            let mut cmd_rx = cmd_rx.lock().await;
            let cmd_rx = cmd_rx.recv();
            pin!(cmd_rx);

            let hub_rx = self.hub.get_msssage_rx().await.clone();
            let mut hub_rx = hub_rx.lock().await;
            let hub_rx = hub_rx.recv();
            pin!(hub_rx);

            match select(cmd_rx, hub_rx).await {
                Either::Left((Some(cmd), _)) => match cmd {
                    Command::Connect {
                        conn_tx,
                        res_tx,
                        id,
                        name,
                    } => {
                        let _ = res_tx.send(self.connect(conn_tx, id, name).await.unwrap());
                    }

                    Command::Disconnect { session_id, res_tx } => {
                        let res = self.disconnect(session_id).await;

                        let _ = res_tx.send(res.unwrap());
                    }

                    Command::GetRoomsBySessionID { session_id, res_tx } => {
                        let _ =
                            res_tx.send(self.get_rooms_by_session_id(session_id).await.unwrap());
                    }

                    Command::GetRoomsByRoomID { room_id, res_tx } => {
                        let _ = res_tx.send(self.get_rooms_by_room_id(room_id).await.unwrap());
                    }

                    Command::Join { conn, room, res_tx } => {
                        let _ = self.join_room(conn, room).await;
                        let _ = res_tx.send(());
                    }

                    Command::Quit { conn, room, res_tx } => {
                        let _ = self.quit_room(conn, room).await;
                        let _ = res_tx.send(());
                    }

                    Command::Name { conn, name, res_tx } => {
                        let _ = self.change_name(conn, name).await;
                        let _ = res_tx.send(());
                    }

                    Command::Message {
                        room,
                        id,
                        msg,
                        res_tx,
                    } => {
                        // self.send_message(room, skip, msg).await;
                        // let _ = res_tx.send(());
                        self.hub
                            .publish_client_msg(ClientMessageForHub {
                                room,
                                id,
                                content: msg,
                            })
                            .await
                            .unwrap();

                        let _ = res_tx.send(());
                    }

                    Command::SystemMessage {
                        room,
                        to_id,
                        msg,
                        res_tx,
                    } => {
                        // self.send_message(room, skip, msg).await;
                        // let _ = res_tx.send(());
                        self.hub
                            .publish_system_msg(SystemMessageForHub {
                                room,
                                to_id,
                                content: msg,
                            })
                            .await
                            .unwrap();

                        let _ = res_tx.send(());
                    }
                },
                Either::Right((Some(msg), _)) => match msg {
                    MessageForHub::Client(ClientMessageForHub { room, id, content }) => {
                        self.send_message(room, Some(id), content).await
                    }
                    MessageForHub::System(SystemMessageForHub {
                        room: _,
                        to_id,
                        content,
                    }) => self.send_system_message(to_id, content).await,
                },
                _ => {
                    log::error!("server closed");

                    break 'outer;
                }
            }
        }

        Ok(())
    }
}

/// Handle and command sender for chat server.
///
/// Reduces boilerplate of setting up response channels in WebSocket handlers.
#[derive(Debug, Clone)]
pub struct ChatServerHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}

impl ChatServerHandle {
    /// Register client message sender and obtain connection ID.
    pub async fn connect(
        &self,
        conn_tx: mpsc::UnboundedSender<String>,
        id: String,
        name: String,
    ) -> SessionID {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::Connect {
                conn_tx,
                res_tx,
                id,
                name,
            })
            .map_err(|err| log::error!("{}", err))
            .unwrap();

        // unwrap: chat server does not drop out response channel
        res_rx.await.unwrap()
    }

    /// List all created rooms.
    pub async fn get_rooms_by_session_id(&self, session_id: &str) -> UpdateRooms {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::GetRoomsBySessionID {
                session_id: session_id.to_string(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap()
    }

    pub async fn get_rooms_by_room_id(&self, room_id: &str) -> UpdateRooms {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::GetRoomsByRoomID {
                room_id: room_id.to_string(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap()
    }

    /// Join `room`, creating it if it does not exist.
    pub async fn join_room(&self, conn: SessionID, room: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::Join {
                conn,
                room: room.into(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap();
    }

    pub async fn quit_room(&self, conn: SessionID, room: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::Quit {
                conn,
                room: room.into(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap();
    }

    pub async fn change_name(&self, conn: SessionID, name: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::Name {
                conn,
                name: name.into(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap();
    }

    /// Broadcast message to current room.
    pub async fn send_message(&self, room: String, conn: SessionID, msg: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::Message {
                id: conn,
                room: room,
                msg: msg.into(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap();
    }

    pub async fn send_system_message(&self, room: &str, to_id: &str, msg: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::SystemMessage {
                room: room.to_string(),
                to_id: to_id.to_string(),
                msg: msg.into(),
                res_tx,
            })
            .unwrap();

        // unwrap: chat server does not drop our response channel
        res_rx.await.unwrap();
    }

    /// Unregister message sender and broadcast disconnection message to current room.
    pub async fn disconnect(&self, conn: SessionID) -> Vec<RoomID> {
        let (res_tx, res_rx) = oneshot::channel();
        // unwrap: chat server should not have been dropped
        self.cmd_tx
            .send(Command::Disconnect {
                session_id: conn,
                res_tx,
            })
            .unwrap();

        res_rx.await.unwrap()
    }
}
