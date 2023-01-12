use crate::model::user::CurrentUser;
use crate::service::user as user_service;
use actix_web::HttpRequest;
use utilities::error::BasicResult;
use utilities::unauthorized_error;

pub async fn get_current_user(req: &HttpRequest) -> BasicResult<CurrentUser> {
    let email = req
        .headers()
        .get("email")
        .ok_or_else(|| unauthorized_error!("can not get email from header"))?
        .to_str()
        .map_err(|err| unauthorized_error!(err))?;
    let res = user_service::get_current_user(email).await?;
    Ok(res)
}

pub async fn get_current_user_by_token(token: &str) -> BasicResult<CurrentUser> {
    #[cfg(feature = "test_ws")]
    {
        use crate::model::user::{UserStatus, UserType};
        use rand::Rng;
        use std::ops::Add;
        use utilities::datetime::FormatDateTime;
        let now = chrono::Utc::now();
        if token == "token" {
            return Ok(CurrentUser {
                id: -1,
                r#type: UserType::Normal,
                email: uuid::Uuid::new_v4().to_string(),
                status: UserStatus::Available,
                name: Some(format!(
                    "Guest_{}",
                    rand::thread_rng().gen_range(10000..99999)
                )),
                mobile: None,
                laston: None,
                created_at: now.to_default(),
                updated_at: None,
                expire_at: now.add(chrono::Duration::days(1)).to_default(),
            });
        }
    }
    let email = user_service::check(token)?;
    let res = user_service::get_current_user(&email).await?;
    Ok(res)
}
