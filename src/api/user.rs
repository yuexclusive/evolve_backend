use crate::model::user as user_model;
use crate::service::user as user_service;
use actix_web::web::{Json, Path, Query};
use actix_web::{delete, get, post, put, HttpRequest, Responder, Result};
use serde::Deserialize;
use util_response::{data, msg, prelude::*};

use crate::session;
// use utilities::response::*;

#[cfg_attr(feature = "openapi", utoipa::path(
    request_body = LoginReq,
    path = "/api/login",
    responses(
        (status = 200, description = "successfully", body = LoginDataResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    )
))]
#[post("/login")]
pub async fn login(req: Json<user_model::LoginReq>) -> Result<impl Responder> {
    let res = user_service::login(&req.email, &req.pwd).await?;
    data!(res)
}

#[cfg_attr(feature = "openapi", utoipa::path(
    request_body = ChangePasswordReq,
    path = "/api/change_pwd",
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    )
))]
#[put("/change_pwd")]
pub async fn change_pwd(req: Json<user_model::ChangePasswordReq>) -> Result<impl Responder> {
    let _ = user_service::change_pwd(&req.email, &req.code, &req.pwd).await?;

    msg!("ok")
}

#[cfg_attr(feature = "openapi", utoipa::path(
    request_body = SendEmailCodeReq,
    path = "/api/send_email_code",
    responses(
        (status = 200, description = "successfully", body = SendEmailResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    )
))]
#[post("/send_email_code")]
pub async fn send_email_code(req: Json<user_model::SendEmailCodeReq>) -> Result<impl Responder> {
    let res = user_service::send_email_code(&req.email, &req.from).await?;
    data!(res)
}

#[cfg_attr(feature = "openapi", utoipa::path(
    request_body = RegisterReq,
    path = "/api/register",
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    )
))]
#[post("/register")]
pub async fn register(req: Json<user_model::RegisterReq>) -> Result<impl Responder> {
    user_service::register(
        &req.email,
        &req.code,
        &req.pwd,
        req.name.as_deref(),
        req.mobile.as_deref(),
    )
    .await?;
    msg!("ok")
}

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/api/validate_exist_email/{email}",
    params(
        ("email", description = "email")
    ),
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    )
))]
#[get("/validate_exist_email/{email}")]
pub async fn validate_exist_email(email: Path<String>) -> Result<impl Responder> {
    user_service::validate_exist_email(&email.into_inner()).await?;

    msg!("ok")
}

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/api/validate_not_exist_email/{email}",
    params(
        ("email", description = "email")
    ),
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    )
))]
#[get("/validate_not_exist_email/{email}")]
pub async fn validate_not_exist_email(email: Path<String>) -> Result<impl Responder> {
    user_service::validate_not_exist_email(&email.into_inner()).await?;

    msg!("ok")
}

#[cfg_attr(feature = "openapi", derive(utoipa::IntoParams))]
#[derive(Deserialize)]
pub struct SearchReq {
    key_word: String,
}
#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/api/user/search",
    params(
        SearchReq, Pagination
    ),
    responses(
        (status = 200, description = "successfully", body = UserSearchResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unthorized", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    ),
    security(
        ("token" = [])
    )
))]
#[get("/search")]
pub async fn search(req: Query<SearchReq>, page: Query<Pagination>) -> Result<impl Responder> {
    let (data, total) = user_service::search(&req.key_word, &page).await?;
    data!(data, total)
}

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/api/user/{id}",
    params(
        ("id", description = "user id")
    ),
    responses(
        (status = 200, description = "successfully", body = UserGetResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    ),
    security(
        ("token" = [])
    )
))]
#[get("/{id}")]
pub async fn get(id: Path<i64>) -> Result<impl Responder> {
    let res = user_service::get(id.into_inner()).await?;
    data!(res)
}

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/api/user/get_current_user",
    responses(
        (status = 200, description = "successfully", body = CurrentUserResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unthorized", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    ),
    security(
        ("token" = [])
    )
))]
#[get("/get_current_user")]
pub async fn get_current_user(req: HttpRequest) -> Result<impl Responder> {
    data!(session::get_current_user(&req).await?)
}

#[cfg_attr(feature = "openapi", utoipa::path(
    request_body = UserUpdateReq,
    path = "/api/user/update",
    responses(
        (status = 200, description = "successfully", body = UserUpdateResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unthorized", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    ),
    security(
        ("token" = [])
    )
))]
#[put("/update")]
pub async fn update(req: Json<user_model::UserUpdateReq>) -> Result<impl Responder> {
    let res = user_service::update(req.id, req.mobile.as_deref(), req.name.as_deref()).await?;
    data!(res)
}

#[cfg_attr(feature = "openapi", utoipa::path(
    request_body = UserDeleteReq,
    path = "/api/user/delete",
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = ErrorResponse),
        (status = 401, description = "unthorized", body = ErrorResponse),
        (status = 500, description = "internal server error", body = ErrorResponse)
    ),
    security(
        ("token" = [])
    )
))]
#[delete("/delete")]
pub async fn delete(req: Json<user_model::UserDeleteReq>) -> Result<impl Responder> {
    let _ = user_service::delete(&req.ids).await?;
    msg!("ok")
}
