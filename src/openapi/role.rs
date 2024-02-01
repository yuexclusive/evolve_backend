use crate::openapi::security::SecurityAddon;
use util_response::{ MsgResponse, MsgResponseWithErrCode };
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
    paths(
    ),
    components(
        schemas(
            MsgResponse,
            MsgResponseWithErrCode,
        )
    ),
    tags(
        (name = "role", description = "role management endpoints.")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
