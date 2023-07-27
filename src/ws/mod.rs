#![cfg(feature = "ws")]

pub mod api;
pub mod session;
pub mod hub;
pub mod server;

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
        let (hub, hub_rx) = hub::RedisHub::new(); // redis hub for distribution
        let (chat_server, cmd_tx, cmd_rx) = ChatServer::new(hub);
        tokio::spawn(chat_server.run(hub_rx, cmd_rx));
        log::info!("ws inited");
        cmd_tx
    }};
}

#[macro_export]
macro_rules! serve_ws {
    ($app: expr, $cmd_tx:expr) => {
        use actix_web::web;
        let cmd_tx = $cmd_tx.clone();
        $app = $app.service(
            scope("/ws")
                .app_data(web::Data::new(cmd_tx))
                .service(ws::api::index)
                .service(ws::api::connect),
        );
        log::info!("ws is serving")
    };
}

// #[macro_export]
// macro_rules! clean_ws {
//     ($hub:expr,$rooms:expr) => {
//         use ws::hub::Hub;
//         $hub.clean($rooms).await?;
//         log::info!("ws cleaned")
//     };
// }
