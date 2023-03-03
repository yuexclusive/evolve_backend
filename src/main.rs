pub mod actix_tool;
pub mod common;
pub mod config;
pub mod controller;
pub mod dao;
pub mod model;
pub mod openapi;
pub mod service;
pub mod session;
#[cfg(feature = "ws")]
pub mod ws;

use actix_web::{get, web::scope, App, HttpServer, Result};
use controller::middleware;
use controller::user;
use std::error::Error;

use service::user as user_service;
// use dotenv::dotenv;

#[get("/ping")]
pub async fn ping() -> &'static str {
    "pong"
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // dotenv().ok();
    log4rs::init_file("log4rs.yml", Default::default())?;

    utilities::pg::init();

    log::info!("DATABASE_URL: {}", std::env!("DATABASE_URL"));

    let cfg = &config::CONFIG;

    let email_cfg = &cfg.email;
    utilities::email::init(
        &email_cfg.username,
        &email_cfg.password,
        &email_cfg.relay,
        email_cfg.port,
    );
    let meilisearch_cfg = &cfg.meilisearch;
    utilities::meilisearch::init(&meilisearch_cfg.address, &meilisearch_cfg.api_key);

    let redis_cfg = &cfg.redis;
    utilities::redis::init(
        &redis_cfg.host,
        redis_cfg.port,
        redis_cfg.username.clone(),
        redis_cfg.password.clone(),
    );

    user_service::reload_search().await?;

    #[cfg(feature = "ws")]
    let (hub, server_tx, rooms);
    #[cfg(feature = "ws")]
    {
        use ws::hub;
        use ws::server::ChatServer;
        log::info!(
            "room change script sha: {}",
            dao::redis::lua_script::ROOMS_CHANGE.get().await.as_str()
        );

        hub = hub::RedisHub::new().await; // redis hub for distribution
        let chat_server; // chat server for local machine
        (chat_server, server_tx, rooms) = ChatServer::new(hub.clone());
        tokio::spawn(chat_server.run());
    }

    log::info!("✅success");
    log::info!("📡server listening at {}:{}", cfg.host, cfg.port);

    HttpServer::new(move || {
        let mut app = App::new()
            .wrap(middleware::logger::logger())
            .wrap(middleware::cors::cors());

        // user api
        app = app.service(
            scope("/api")
                .service(ping)
                .service(user::login)
                .service(user::register)
                .service(user::send_email_code)
                .service(user::validate_exist_email)
                .service(user::validate_not_exist_email)
                .service(user::change_pwd)
                .service(
                    scope("/user")
                        .wrap(middleware::auth::Auth)
                        .service(user::search)
                        .service(user::update)
                        .service(user::get_current_user)
                        .service(user::delete)
                        .service(user::get)
                        .service(user::send_email_code)
                        .service(user::validate_exist_email)
                        .service(user::validate_not_exist_email),
                ),
        );
        app = app.service(
            scope("/file")
                .service(controller::upload_file::upload_page)
                .service(controller::upload_file::upload),
        );

        app = app.service(
            scope("/static")
                .service(scope("/single").service(controller::static_file::file))
                .service(controller::static_file::static_files()), // .service(controller::static_file::src_files()),
        );

        // ws api
        #[cfg(feature = "ws")]
        {
            use actix_web::web;
            app = app.service(
                scope("/ws")
                    .app_data(web::Data::new(server_tx.clone()))
                    .service(controller::ws::index)
                    .service(controller::ws::connect),
            )
        }
        // swagger api
        #[cfg(feature = "openapi")]
        {
            use utoipa::OpenApi;
            use utoipa_swagger_ui::{SwaggerUi, Url};
            // app = app.service(SwaggerUi::new("/swagger/user/{_:.*}").url(
            //     "/api-doc/user.json",
            //     openapi::user::ApiDoc::openapi().clone(),
            // ))
            app = app.service(SwaggerUi::new("/swagger/{_:.*}").urls(vec![
                (
                    Url::new("user", "/api-doc/user.json"),
                    openapi::user::ApiDoc::openapi().clone(),
                ),
                (
                    Url::new("role", "/api-doc/role.json"),
                    openapi::role::ApiDoc::openapi().clone(),
                ),
            ]));
        }
        app
    })
    .bind((cfg.host.as_str(), cfg.port))?
    .run()
    .await
    .unwrap();

    #[cfg(feature = "ws")]
    {
        use ws::hub::Hub;
        hub.clean_sessions(rooms).await;
    }

    log::info!("📡server stoped");

    Ok(())
}
