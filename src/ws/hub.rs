#![cfg(feature = "ws")]

use crate::dao::redis::lua_script;
use async_trait::async_trait;
use futures::StreamExt;
use redis::aio::ConnectionLike;
use redis::FromRedisValue;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use utilities::business_error;
use utilities::error::BasicResult;
use utilities::redis::derive::{FromRedisValue, ToRedisArgs};

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize, Debug)]
pub enum RoomChangeType {
    Add,
    Del,
    NameChange,
}

impl Default for RoomChangeType {
    fn default() -> Self {
        RoomChangeType::Add
    }
}

const CLIENT_MESSAGE_CHANNEL: &str = "client_message";
const SYSTEM_MESSAGE_CHANNEL: &str = "system_message";

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize, Debug)]
pub struct UpdateRooms(pub HashMap<String, HashMap<String, String>>);

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize, Debug, Default)]
pub struct RoomChangeForHub {
    pub id: String,
    pub name: Option<String>,
    pub room: String,
    pub r#type: RoomChangeType,
}

#[derive(FromRedisValue, Deserialize, Debug, Default)]
pub struct RoomChangeForHubResponse {
    pub status: i8,
    pub msg: String,
}

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize, Debug)]
pub struct ClientMessageForHub {
    pub room: String,
    pub id: String,
    pub content: String,
}

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize, Debug)]
pub struct SystemMessageForHub {
    pub room: String,
    pub to_id: String,
    pub content: String,
}

#[derive(Debug)]
pub enum MessageForHub {
    Client(ClientMessageForHub),
    System(SystemMessageForHub),
}

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct RetrieveRroomsReq {
    r#type: RetrieveRroomsReqType,
    id: String,
}

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub enum RetrieveRroomsReqType {
    #[serde(rename = "get_by_room_id")]
    RoomID,
    #[serde(rename = "get_by_session_id")]
    SessionID,
}

impl RetrieveRroomsReq {
    pub fn new(r#type: RetrieveRroomsReqType, id: String) -> Self {
        Self { r#type, id }
    }
}

#[derive(FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct HubData {
    pub sessions: HashMap<String, String>,
    pub rooms: HashMap<String, HashMap<String, bool>>, // for json format and lua script
    pub session_room_map: HashMap<String, HashMap<String, bool>>, // for json format and lua script
}

#[async_trait]
pub trait Hub {
    async fn get_msssage_rx(&self) -> Arc<Mutex<UnboundedReceiver<MessageForHub>>>;
    async fn open_channel(&self, room: &str) -> BasicResult<()>;
    async fn close_channel(&self, room: &str) -> BasicResult<()>;
    async fn publish_client_msg(&self, message: ClientMessageForHub) -> BasicResult<()>;
    async fn publish_system_msg(&self, message: SystemMessageForHub) -> BasicResult<()>;
    async fn clean(&self, rooms: Arc<Mutex<HashMap<String, HashSet<String>>>>) -> BasicResult<()>;
    async fn change_rooms(&self, change: RoomChangeForHub) -> BasicResult<()>;
    async fn retrieve_rooms(&self, req: RetrieveRroomsReq) -> BasicResult<UpdateRooms>;
}

pub struct RedisHub {
    message_tx: UnboundedSender<MessageForHub>,
    message_rx: Arc<Mutex<UnboundedReceiver<MessageForHub>>>,
    channels: Arc<Mutex<HashMap<String, (UnboundedSender<bool>, oneshot::Receiver<bool>)>>>,
}
impl RedisHub {
    async fn subscribe(
        room: &str,
        msg_tx: UnboundedSender<MessageForHub>,
    ) -> BasicResult<(UnboundedSender<bool>, oneshot::Receiver<bool>)> {
        let mut system_msg_subscribe =
            utilities::redis::subscribe(&format!("{}_{}", room, SYSTEM_MESSAGE_CHANNEL)).await?;
        let mut client_msg_subscribe =
            utilities::redis::subscribe(&format!("{}_{}", room, CLIENT_MESSAGE_CHANNEL)).await?;
        // let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (close_tx, mut close_rx) = mpsc::unbounded_channel::<bool>();
        let (close_done_tx, close_done_rx) = oneshot::channel();
        tokio::spawn(async move {
            'l: loop {
                tokio::select! {
                   Some(msg) = client_msg_subscribe.next()=>{
                    let payload = msg.get_payload::<ClientMessageForHub>().unwrap();
                    msg_tx.send(MessageForHub::Client(payload)).unwrap();
                   }
                   Some(msg) = system_msg_subscribe.next()=>{
                    let payload = msg.get_payload::<SystemMessageForHub>().unwrap();
                    msg_tx.send(MessageForHub::System(payload)).unwrap();
                   }
                   Some(v)= close_rx.recv()=>{
                     close_done_tx.send(v).unwrap();
                     break 'l
                   }
                }
            }
        });

        Ok((close_tx, close_done_rx))
    }

    pub async fn new() -> Arc<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        Arc::new(Self {
            message_rx: Arc::new(Mutex::new(msg_rx)),
            message_tx: msg_tx,
            channels: Default::default(),
        })
    }
}

#[async_trait]
impl Hub for RedisHub {
    async fn get_msssage_rx(&self) -> Arc<Mutex<UnboundedReceiver<MessageForHub>>> {
        self.message_rx.clone()
    }
    async fn publish_client_msg(&self, message: ClientMessageForHub) -> BasicResult<()> {
        let res = utilities::redis::sync::publish(
            format!("{}_{}", message.room.clone(), CLIENT_MESSAGE_CHANNEL),
            message,
        )
        .unwrap();
        Ok(res)
    }

    async fn publish_system_msg(&self, message: SystemMessageForHub) -> BasicResult<()> {
        let res = utilities::redis::sync::publish(
            format!("{}_{}", message.room.clone(), SYSTEM_MESSAGE_CHANNEL),
            message,
        )
        .unwrap();
        Ok(res)
    }

    async fn change_rooms(&self, change: RoomChangeForHub) -> BasicResult<()> {
        // let input = serde_json::to_string(&change).unwrap();
        let mut cmd = redis::cmd("evalsha");

        cmd.arg(lua_script::ROOMS_CHANGE.get().await.as_str()) //sha
            .arg(0) //keys number
            .arg(&change);

        let value = utilities::redis::conn()
            .await?
            .req_packed_command(&cmd)
            .await?;

        let res = RoomChangeForHubResponse::from_redis_value(&value)?;

        if res.status != 0 {
            return business_error!(res.msg).into();
        }

        Ok(())
    }

    async fn retrieve_rooms(&self, req: RetrieveRroomsReq) -> BasicResult<UpdateRooms> {
        let mut cmd = redis::cmd("evalsha");

        cmd.arg(lua_script::ROOMS_RETRIEVE.get().await.as_str()) //sha
            .arg(0) //keys number
            .arg(&req);

        let value = utilities::redis::conn()
            .await?
            .req_packed_command(&cmd)
            .await?;

        let data = UpdateRooms::from_redis_value(&value)?;
        Ok(data)
    }

    async fn clean(&self, rooms: Arc<Mutex<HashMap<String, HashSet<String>>>>) -> BasicResult<()> {
        for (room, sessions) in rooms.lock().await.iter() {
            for id in sessions {
                self.change_rooms(RoomChangeForHub {
                    id: id.to_string(),
                    room: room.to_string(),
                    name: None,
                    r#type: RoomChangeType::Del,
                })
                .await
                .unwrap();
            }
        }
        Ok(())
    }

    async fn open_channel(&self, room: &str) -> BasicResult<()> {
        let mut channels = self.channels.lock().await;
        if !channels.contains_key(room) {
            let (close, close_done) = Self::subscribe(&room, self.message_tx.clone()).await?;
            channels.insert(room.to_string(), (close, close_done));
        }
        Ok(())
    }

    async fn close_channel(&self, room: &str) -> BasicResult<()> {
        let mut channels = self.channels.lock().await;
        if let Some((close, close_done)) = channels.remove(room) {
            close.send(true)?;
            close_done.await?; //waiting for close done
        }
        Ok(())
    }
}
