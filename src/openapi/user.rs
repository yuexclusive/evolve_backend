#![cfg(feature = "openapi")]
use utoipa::OpenApi;
use utilities::response::{MsgResponse,ErrorResponse};
use crate::controller::user as user_controller;
use crate::model::user as user_model;
use crate::openapi::security::SecurityAddon;

#[derive(OpenApi)]
#[openapi(
    paths(
        user_controller::login,
        user_controller::change_pwd,
        user_controller::send_email_code,
        user_controller::register,
        user_controller::search,
        user_controller::get,
        user_controller::get_current_user,
        user_controller::update,
        user_controller::delete,
        user_controller::validate_exist_email,
        user_controller::validate_not_exist_email,
    ),
    components(
        schemas(
            user_model::LoginReq,
            user_model::RegisterReq,
            user_model::CurrentUser,
            user_model::SearchedUser,
            user_model::UserSearchResponse,
            user_model::User,
            user_model::CurrentUserResponse,
            user_model::UserFormatter,
            user_model::UserStatus,
            user_model::UserType, 
            user_model::SendEmailCodeFrom,
            user_model::ChangePasswordReq,
            user_model::SendEmailCodeReq,
            user_model::LoginDataResponse,
            user_model::UserGetResponse,
            user_model::SendEmailResponse,
            user_model::UserUpdateReq,
            user_model::UserUpdateResponse,
            MsgResponse,
            ErrorResponse,
            user_model::UserDeleteReq,
        )
    ),
    tags(
        (name = "user", description = "user management endpoints.")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;


