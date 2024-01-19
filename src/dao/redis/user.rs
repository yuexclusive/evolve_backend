use crate::model::user as user_model;
use util_error::BasicResult;
use util_redis as redis_util;

fn email_code_key(email: &str, from: &user_model::SendEmailCodeFrom) -> String {
    format!("{email}_mail_{:?}", from)
}

fn user_agent_key(email: &str) -> String {
    format!("{email}_session")
}

pub async fn get_email_code(
    email: &str,
    from: &user_model::SendEmailCodeFrom,
) -> BasicResult<Option<String>> {
    let res = redis_util::get::<_, Option<String>>(email_code_key(email, from)).await?;

    Ok(res)
}

pub async fn set_email_code(
    email: &str,
    from: &user_model::SendEmailCodeFrom,
    code: impl Into<String>,
    expired_seconds: u64,
) -> BasicResult<()> {
    redis_util::set_ex(email_code_key(email, from), code.into(), expired_seconds).await?;
    Ok(())
}

pub async fn exist_email_code(
    email: &str,
    from: &user_model::SendEmailCodeFrom,
) -> BasicResult<bool> {
    let res = redis_util::exists(email_code_key(email, from)).await?;
    Ok(res)
}

pub async fn exist_current_user(email: &str) -> BasicResult<bool> {
    let res = redis_util::exists(user_agent_key(&email)).await?;
    Ok(res)
}

pub async fn set_current_user(
    current_user: user_model::CurrentUser,
    expired_seconds: u64,
) -> BasicResult<()> {
    redis_util::set_ex(
        user_agent_key(&current_user.email),
        current_user,
        expired_seconds,
    )
    .await?;
    Ok(())
}
pub async fn get_current_user(email: &str) -> BasicResult<user_model::CurrentUser> {
    let current_user =
        redis_util::get::<_, user_model::CurrentUser>(user_agent_key(&email)).await?;
    Ok(current_user)
}
