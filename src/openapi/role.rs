use crate::openapi::security::SecurityAddon;
use utilities::response::MsgResponse;
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
    paths(
    ),
    components(
        schemas(
            MsgResponse,
        )
    ),
    tags(
        (name = "role", description = "role management endpoints.")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
