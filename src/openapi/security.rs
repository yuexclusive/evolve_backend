use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify,
};

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.

        components.add_security_scheme(
            "token",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("token"))),
        );
    }
}
