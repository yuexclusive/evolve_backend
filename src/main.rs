#![allow(dead_code)]

mod api;
mod config;
mod dao;
mod init;
mod middleware;
mod model;
mod openapi;
mod service;
mod session;
mod static_file;
mod upload_file;
mod ws;

use actix_web::{get, web::scope, App, HttpServer, Result};
use std::error::Error;

#[get("/ping")]
pub async fn ping() -> &'static str {
    "pong"
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init::init().await?;

    #[cfg(feature = "ws")]
    let (hub, server_tx, rooms) = init_ws!();

    HttpServer::new(move || {
        let mut app = App::new()
            .wrap(middleware::logger::logger())
            .wrap(middleware::cors::cors());

        serve_api!(app);
        #[cfg(feature = "openapi")]
        serve_openapi!(app);
        #[cfg(feature = "ws")]
        serve_ws!(app, server_tx);
        #[cfg(feature = "static_file")]
        serve_static_file!(app);
        #[cfg(feature = "upload_file")]
        serve_upload_file!(app);
        app
    })
    .bind((config::cfg().host.as_str(), config::cfg().port))?
    .run()
    .await
    .unwrap();

    #[cfg(feature = "ws")]
    clean_ws!(hub, rooms);
    log::info!("server stoped");
    Ok(())
}
