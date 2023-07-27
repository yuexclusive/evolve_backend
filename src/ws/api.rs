#![cfg(feature = "ws")]

use crate::session as user_session;
use crate::ws::server::ChatServerHandle;
use crate::ws::session as ws_session;
use actix_files::NamedFile;
use actix_web::{
    get,
    web::{self, Path},
};
use actix_web::{HttpRequest, Responder, Result};
use tokio::task::spawn_local;

#[get("/index")]
async fn index() -> Result<impl Responder> {
    Ok(NamedFile::open_async("static/ws/index.html").await.unwrap())
}

#[get("/ws/{token}")]
async fn connect(
    token: Path<String>,
    req: HttpRequest,
    stream: web::Payload,
    chat_server: web::Data<ChatServerHandle>,
) -> Result<impl Responder> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let token = token.to_string();
    let user = user_session::get_current_user_by_token(&token).await?;
    let id = user.email;
    let name = user.name.unwrap_or(id.clone());
    
    spawn_local(ws_session::chat_ws(
        id,
        name,
        (**chat_server).clone(),
        session,
        msg_stream,
    ));

    Ok(res)
}
