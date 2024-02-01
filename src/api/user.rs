use crate::model::user as user_model;
use crate::service::user as user_service;
use actix_web::web::{Json, Path, Query};
use actix_web::{delete, get, post, put, HttpRequest, Responder, Result};
use serde::Deserialize;
use util_response::{data, msg, prelude::*};

use crate::session;
// use utilities::response::*;

#[utoipa::path(
    request_body = LoginReq,
    path = "/api/login",
    responses(
        (status = 200, description = "successfully", body = LoginDataResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    )
)]
#[post("/login")]
pub async fn login(req: Json<user_model::LoginReq>) -> Result<impl Responder> {
    let res = user_service::login(&req.email, &req.pwd).await?;
    Ok(Json(data!(res)))
}

#[utoipa::path(
    request_body = ChangePasswordReq,
    path = "/api/change_pwd",
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    )
)]
#[put("/change_pwd")]
pub async fn change_pwd(req: Json<user_model::ChangePasswordReq>) -> Result<impl Responder> {
    let _ = user_service::change_pwd(&req.email, &req.code, &req.pwd).await?;

    Ok(Json(msg!("ok")))
}

#[utoipa::path(
    request_body = SendEmailCodeReq,
    path = "/api/send_email_code",
    responses(
        (status = 200, description = "successfully", body = SendEmailResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    )
)]
#[post("/send_email_code")]
pub async fn send_email_code(req: Json<user_model::SendEmailCodeReq>) -> Result<impl Responder> {
    let res = user_service::send_email_code(&req.email, &req.from).await?;
    Ok(Json(data!(res)))
}

#[utoipa::path(
    request_body = RegisterReq,
    path = "/api/register",
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    )
)]
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
    Ok(Json(msg!("ok")))
}

#[utoipa::path(
    path = "/api/validate_exist_email/{email}",
    params(
        ("email", description = "email")
    ),
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    )
)]
#[get("/validate_exist_email/{email}")]
pub async fn validate_exist_email(email: Path<String>) -> Result<impl Responder> {
    user_service::validate_exist_email(&email.into_inner()).await?;

    Ok(Json(msg!("ok")))
}

#[utoipa::path(
    path = "/api/validate_not_exist_email/{email}",
    params(
        ("email", description = "email")
    ),
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    )
)]
#[get("/validate_not_exist_email/{email}")]
pub async fn validate_not_exist_email(email: Path<String>) -> Result<impl Responder> {
    user_service::validate_not_exist_email(&email.into_inner()).await?;

    Ok(Json(msg!("ok")))
}

#[derive(utoipa::IntoParams, Deserialize)]
pub struct SearchReq {
    key_word: String,
}
#[utoipa::path(
    path = "/api/user/search",
    params(
        SearchReq, Pagination
    ),
    responses(
        (status = 200, description = "successfully", body = UserSearchResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 401, description = "unthorized", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    ),
    security(
        ("token" = [])
    )
)]
#[get("/search")]
pub async fn search(req: Query<SearchReq>, page: Query<Pagination>) -> Result<impl Responder> {
    let (data, total) = user_service::search(&req.key_word, &page).await?;
    Ok(Json(data!(data, total)))
}

#[utoipa::path(
    path = "/api/user/{id}",
    params(
        ("id", description = "user id")
    ),
    responses(
        (status = 200, description = "successfully", body = UserGetResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    ),
    security(
        ("token" = [])
    )
)]
#[get("/{id}")]
pub async fn get(id: Path<i64>) -> Result<impl Responder> {
    let res = user_service::get(id.into_inner()).await?;
    Ok(Json(data!(res)))
}

#[utoipa::path(
    path = "/api/user/get_current_user",
    responses(
        (status = 200, description = "successfully", body = CurrentUserResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 401, description = "unthorized", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    ),
    security(
        ("token" = [])
    )
)]
#[get("/get_current_user")]
pub async fn get_current_user(req: HttpRequest) -> Result<impl Responder> {
    Ok(Json(data!(session::get_current_user(&req).await?)))
}

#[utoipa::path(
    request_body = UserUpdateReq,
    path = "/api/user/update",
    responses(
        (status = 200, description = "successfully", body = UserUpdateResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 401, description = "unthorized", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    ),
    security(
        ("token" = [])
    )
)]
#[put("/update")]
pub async fn update(req: Json<user_model::UserUpdateReq>) -> Result<impl Responder> {
    let res = user_service::update(req.id, req.mobile.as_deref(), req.name.as_deref()).await?;
    Ok(Json(data!(res)))
}

#[utoipa::path(
    request_body = UserDeleteReq,
    path = "/api/user/delete",
    responses(
        (status = 200, description = "successfully", body = MsgResponse),
        (status = 400, description = "bad request", body = MsgResponseWithErrCode),
        (status = 401, description = "unthorized", body = MsgResponseWithErrCode),
        (status = 500, description = "internal server error", body = MsgResponseWithErrCode)
    ),
    security(
        ("token" = [])
    )
)]
#[delete("/delete")]
pub async fn delete(req: Json<user_model::UserDeleteReq>) -> Result<impl Responder> {
    let _ = user_service::delete(&req.ids).await?;
    Ok(Json(msg!("ok")))
}
