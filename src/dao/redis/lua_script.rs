use async_once::AsyncOnce;
use lazy_static::lazy_static;
use redis::{aio::ConnectionLike, FromRedisValue};
use utilities::error::BasicResult;

lazy_static! {
    pub static ref ROOMS_CHANGE: AsyncOnce<String> =
        AsyncOnce::new(async { load_rooms_change().await.unwrap() });
}

async fn load_rooms_change() -> BasicResult<String> {
    let mut conn = utilities::redis::conn().await?;
    let files = ["json.lua", "rooms.lua"];
    let cmd_str: String = files
        .iter()
        .map(|&name| std::fs::read_to_string(format!("static/lua_scripts/{}", name)).unwrap())
        .collect();
        
    let mut cmd = redis::cmd("script");
    
    cmd.arg("load").arg(cmd_str);

    let value = conn.req_packed_command(&cmd).await?;

    let res = String::from_redis_value(&value)?;
    Ok(res)
}
