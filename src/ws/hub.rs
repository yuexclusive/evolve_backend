use crate::dao::redis::lua_script;
use redis::aio::ConnectionLike;
use redis::FromRedisValue;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{self, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use util_error::business_error;
use util_error::BasicResult;
use util_redis::derive::{from_redis, to_redis};

#[from_redis]
// #[to_redis]
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

const MESSAGE_CHANNEL: &str = "message";

#[to_redis]
#[from_redis]
pub struct UpdateRooms(pub HashMap<String, HashMap<String, String>>);

#[to_redis]
#[from_redis]
pub struct ChangeRoomReq {
    pub id: String,
    pub name: Option<String>,
    pub room: String,
    pub r#type: RoomChangeType,
}

#[from_redis]
pub struct RoomChangeForHubResponse {
    pub status: i8,
    pub msg: String,
}

#[from_redis]
#[to_redis]
#[derive(Debug)]
pub struct MessageForHub {
    pub room: String,
    pub id: String,
    pub content: String,
}

#[from_redis]
#[to_redis]
pub struct RetrieveRroomsReq {
    r#type: RetrieveRroomsReqType,
    id: String,
}

#[from_redis]
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

pub trait Hub {
    async fn subscribe_room(&self, room: &str) -> BasicResult<()>;
    async fn unsubscribe_room(&self, room: &str) -> BasicResult<()>;
    async fn publish(&self, message: MessageForHub) -> BasicResult<()>;
    async fn clean(&self, rooms: &HashMap<String, HashSet<String>>) -> BasicResult<()>;
    async fn change_rooms(&self, req: ChangeRoomReq) -> BasicResult<()>;
    async fn retrieve_rooms(&self, req: RetrieveRroomsReq) -> BasicResult<UpdateRooms>;
}

pub struct RedisHub {
    message_tx: UnboundedSender<MessageForHub>,
    channels: Arc<Mutex<HashMap<String, (Sender<()>, oneshot::Receiver<()>)>>>,
}
impl RedisHub {
    pub fn new() -> (Self, UnboundedReceiver<MessageForHub>) {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        (
            Self {
                message_tx: msg_tx,
                channels: Default::default(),
            },
            msg_rx,
        )
    }
}

impl Hub for RedisHub {
    async fn publish(&self, message: MessageForHub) -> BasicResult<()> {
        let res = util_redis::publish(
            format!("{}_{}", message.room.clone(), MESSAGE_CHANNEL),
            message,
        )
        .await?;
        Ok(res)
    }

    async fn change_rooms(&self, change: ChangeRoomReq) -> BasicResult<()> {
        // let input = serde_json::to_string(&change).unwrap();
        let mut cmd = redis::cmd("evalsha");

        cmd.arg(lua_script::ROOMS_CHANGE.get().await.as_str()) //sha
            .arg(0) //keys number
            .arg(&change);

        let value = util_redis::conn().await?.req_packed_command(&cmd).await?;

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

        let value = util_redis::conn().await?.req_packed_command(&cmd).await?;

        let data = UpdateRooms::from_redis_value(&value)?;
        Ok(data)
    }

    async fn clean(&self, rooms: &HashMap<String, HashSet<String>>) -> BasicResult<()> {
        for (room, sessions) in rooms.iter() {
            for id in sessions {
                self.change_rooms(ChangeRoomReq {
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

    async fn subscribe_room(&self, room: &str) -> BasicResult<()> {
        let mut channels = self.channels.lock().await;
        if !channels.contains_key(room) {
            let message_tx = self.message_tx.clone();
            let (close_tx, close_done_rx) =
                util_redis::subscribe(&format!("{}_{}", room, MESSAGE_CHANNEL), move |msg| {
                    let payload = msg.get_payload::<MessageForHub>().unwrap();
                    message_tx.send(payload).unwrap();
                })
                .await?;
            channels.insert(room.to_string(), (close_tx, close_done_rx));
        }
        Ok(())
    }

    async fn unsubscribe_room(&self, room: &str) -> BasicResult<()> {
        let mut channels = self.channels.lock().await;
        if let Some((close, close_done)) = channels.remove(room) {
            close.send(()).await.unwrap();
            close_done.await.unwrap(); //waiting for close done
        }
        Ok(())
    }
}
