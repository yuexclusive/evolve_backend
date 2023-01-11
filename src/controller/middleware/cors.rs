use actix_cors::Cors;

pub fn cors() -> Cors {
    Cors::default()
        .allow_any_header()
        .allow_any_method()
        .allow_any_origin()
        .expose_any_header()
        // .supports_credentials()
        .max_age(3600)
}
