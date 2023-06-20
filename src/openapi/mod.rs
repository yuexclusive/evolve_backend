#![cfg(feature = "openapi")]
pub mod role;
pub mod security;
pub mod user;

use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

#[macro_export]
macro_rules! serve_openapi {
    ($app: expr) => {
        $app = $app.service(crate::openapi::swaggerui());
    };
}

pub fn swaggerui() -> SwaggerUi {
    // app = app.service(SwaggerUi::new("/swagger/user/{_:.*}").url(
    //     "/api-doc/user.json",
    //     openapi::user::ApiDoc::openapi().clone(),
    // ))
    let urls = vec![
        (
            Url::new("user", "/api-doc/user.json"),
            crate::openapi::user::ApiDoc::openapi().clone(),
        ),
        (
            Url::new("role", "/api-doc/role.json"),
            crate::openapi::role::ApiDoc::openapi().clone(),
        ),
    ];
    SwaggerUi::new("/swagger/{_:.*}").urls(urls)
}
