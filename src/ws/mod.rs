#![cfg(feature = "ws")]

pub mod handler;
pub mod hub;
pub mod server;
pub mod api;

/// init websocket
///
/// it will start a chat_server and hub
///
/// chat_server is a single session
///
/// hub can manage mutiple sessions through a data center and message queue
#[macro_export]
macro_rules! init_ws {
    () => {{
        use ws::hub;
        use ws::server::ChatServer;
        let hub = hub::RedisHub::new().await; // redis hub for distribution
        let (chat_server, server_tx, rooms) = ChatServer::new(hub.clone());
        tokio::spawn(chat_server.run());
        log::info!("ws inited");
        (hub, server_tx, rooms)
    }};
}

#[macro_export]
macro_rules! serve_ws {
    ($app: expr, $server_tx:expr) => {
        use actix_web::web;
        $app = $app.service(
            scope("/ws")
                .app_data(web::Data::new($server_tx.clone()))
                .service(ws::api::index)
                .service(ws::api::connect),
        );
        log::info!("ws is serving")
    };
}

#[macro_export]
macro_rules! clean_ws {
    ($hub:expr,$rooms:expr) => {
        use ws::hub::Hub;
        $hub.clean($rooms).await?;
        log::info!("ws cleaned")
    };
}
