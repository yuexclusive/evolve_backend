use actix_web::middleware;

pub fn logger() -> middleware::Logger {
    middleware::Logger::default()
}
