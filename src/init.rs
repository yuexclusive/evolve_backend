use crate::config;
use crate::service::user as user_service;
use dotenv::dotenv;
use std::error::Error;

pub async fn init() -> Result<(), Box<dyn Error>> {
    // env
    dotenv().ok();

    // init log
    log4rs::init_file("log4rs.yml", Default::default())?;

    // get config
    let cfg = config::cfg();

    // init pg
    util_postgres::init();

    // init redis
    util_redis::init(
        &cfg.redis.host,
        cfg.redis.port,
        cfg.redis.username.clone(),
        cfg.redis.password.clone(),
    )
    .await;

    // init meilisearch
    util_meilisearch::init(&cfg.meilisearch.address, &cfg.meilisearch.api_key).await;

    // init email
    util_email::init(
        &cfg.email.username,
        &cfg.email.password,
        &cfg.email.relay,
        cfg.email.port,
    )
    .await;

    // load user search data
    user_service::load_search().await?;

    Ok(())
}
