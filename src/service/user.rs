use crate::{
    dao::{
        meilisearch::{self as meilisearch_dao, user as meilisearch_user_dao},
        pg::user as pg_user_dao,
        redis::user as redis_user_dao,
    },
    model::{common::Pagination, user as user_model},
};

use futures::TryFutureExt;
use rand::Rng;

use serde::{Deserialize, Serialize};

use utilities::{email as email_util, error::BasicResult, hint_error, unauthorized_error};

mod private {
    use base64::{engine::general_purpose, Engine as _};

    const EMAIL_VALIDATE_REGEX: &str = r#"\w[-\w.+]*@([A-Za-z0-9][-A-Za-z0-9]+\.)+[A-Za-z]{2,14}"#;
    const PWD_VALIDATE_REGEX: &str = r#"^[a-zA-Z]{1}\w{5,17}$"#;
    const MOBILE_VALIDATE_REGEX: &str = r#"0?(13|14|15|17|18|19)[0-9]{9}"#;

    use std::cmp::Ordering;

    use super::{
        meilisearch_dao, pg_user_dao, redis_user_dao, user_model, BasicResult, Deserialize,
        Serialize,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use regex::Regex;
    use sha2::Digest;
    use utilities::{datetime::FormatDateTime, unauthorized_error, validation_error};
    use uuid::Uuid;
    struct Token {
        secret: &'static str,
        algorithm: Algorithm,
        duration: usize,
    }

    const TOKEN: Token = Token {
        secret: "secret",
        algorithm: Algorithm::HS512,
        duration: 60 * 60 * 24,
    };

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        aud: String, // Optional. Audience
        exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
        iat: usize, // Optional. Issued at (as UTC timestamp)
                    // sub: Option<String>, // Optional. Subject (whom token refers to)
                    // iss: String, // Optional. Issuer
                    // nbf: usize, // Optional. Not Before (as UTC timestamp)
    }

    pub(super) fn hash_password(password: impl AsRef<str>, salt: impl AsRef<str>) -> String {
        let mut hasher = sha2::Sha512::new();
        hasher.update(password.as_ref());
        hasher.update(b"$");
        hasher.update(salt.as_ref());
        let encoded: String = general_purpose::STANDARD.encode(&hasher.finalize());
        encoded
    }

    pub(super) fn salt() -> String {
        Uuid::new_v4().to_string()
    }

    pub(super) fn check_pwd(
        pwd: impl AsRef<str>,
        salt: impl AsRef<str>,
        pwd_hashed: Option<impl AsRef<str>>,
    ) -> BasicResult<()> {
        match pwd_hashed {
            Some(v) => {
                let pwd = hash_password(pwd, salt);
                match pwd.as_str().cmp(v.as_ref()) {
                    Ordering::Equal => Ok(()),
                    _ => validation_error!("wrong password").into(),
                }
            }
            None => validation_error!("password has not been initialized").into(),
        }
    }

    pub(super) fn validate_email(email: &str) -> BasicResult<()> {
        match email.is_empty() {
            true => validation_error!("please type in email").into(),
            _ => {
                let reg = Regex::new(EMAIL_VALIDATE_REGEX)?;
                match reg.is_match(email) {
                    false => validation_error!("invalid email").into(),
                    _ => Ok(()),
                }
            }
        }
    }

    pub(super) fn validate_pwd(pwd: &str) -> BasicResult<()> {
        match pwd.is_empty() {
            true => Err(validation_error!("please type in passwd")),
            _ => {
                let reg = Regex::new(PWD_VALIDATE_REGEX)?; //6位字母+数字,字母开头
                match reg.is_match(pwd) {
                    false => {
                        validation_error!("invalid passowrd: length>=6, a-z and 0-9 is demanded")
                            .into()
                    }
                    _ => Ok(()),
                }
            }
        }
    }

    pub(super) fn validate_mobile(mobile: &str) -> BasicResult<()> {
        let reg = Regex::new(MOBILE_VALIDATE_REGEX)?;
        match reg.is_match(mobile) {
            false => validation_error!("invalid mobile").into(),
            _ => Ok(()),
        }
    }

    pub(super) async fn validate_email_code(
        email: &str,
        from: &user_model::SendEmailCodeFrom,
        code: &str,
    ) -> BasicResult<()> {
        let email_code: Option<String> = redis_user_dao::get_email_code(email, from).await?;

        match email_code.is_none() || email_code.unwrap() != code {
            true => validation_error!(format!("invalid code {}", code)).into(),
            _ => Ok(()),
        }
    }

    pub(super) async fn validate_exist_email(email: &str) -> BasicResult<pg_user_dao::User> {
        validate_email(email)?;
        let res = pg_user_dao::get_by_email(email).await?;
        match res.deleted_at {
            Some(_) => validation_error!("email has already been deleted").into(),
            _ => Ok(res),
        }
    }

    pub(super) async fn validate_not_exist_email(email: &str) -> BasicResult<()> {
        validate_email(email)?;
        match pg_user_dao::get_by_email(email).await {
            Ok(_) => validation_error!(format!("email {} already exist", email)).into(),
            _ => Ok(()),
        }
    }

    pub(super) fn check_token(token: &str) -> BasicResult<String> {
        let token = token.trim_start_matches("Bearer ");
        let validation = Validation::new(TOKEN.algorithm);
        let claims = jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(TOKEN.secret.as_ref()),
            &validation,
        )
        .map_err(|err| unauthorized_error!(err))?;
        Ok(claims.claims.aud)
    }

    pub(super) async fn set_current_user(
        user: &pg_user_dao::User,
        now: &DateTime<Utc>,
    ) -> BasicResult<String> {
        let header = Header::new(TOKEN.algorithm);
        let iat = now.timestamp() as usize;
        let exp = iat + TOKEN.duration;

        let claims = Claims {
            aud: user.email.clone(),
            iat: iat,
            exp: exp,
        };
        let token = jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_secret(TOKEN.secret.as_ref()),
        )
        .map_err(|err| {
            log::error!("encode token err: {}", err.to_string());
            err
        })?;

        redis_user_dao::set_current_user(
            user_model::CurrentUser {
                id: user.id,
                r#type: user.r#type.clone(),
                email: user.email.clone(),
                status: user.status.clone(),
                name: user.name.clone(),
                mobile: user.mobile.clone(),
                laston: user.laston.map(|x| x.to_default()),
                created_at: user.created_at.to_default(),
                updated_at: user.updated_at.map(|x| x.to_default()),
                expire_at: chrono::Utc
                    .timestamp_opt(exp as i64, 0)
                    .unwrap()
                    .to_default(),
            },
            TOKEN.duration,
        )
        .await?;
        Ok(token)
    }

    pub(super) async fn update_search<T>(data: T) -> BasicResult<()>
    where
        T: Into<user_model::User>,
    {
        meilisearch_dao::update(meilisearch_dao::USER_LIST_INDEX, &[data.into()], Some("id"))
            .await?;
        Ok(())
    }
}

pub async fn search(
    key_word: &str,
    page: &Pagination,
) -> BasicResult<(Vec<user_model::SearchedUser>, usize)> {
    meilisearch_user_dao::search(key_word, page).await
}

pub async fn get(id: i64) -> BasicResult<user_model::User> {
    let res = pg_user_dao::get(id).await?.into();
    Ok(res)
}

pub async fn get_all() -> BasicResult<Vec<user_model::User>> {
    let res = pg_user_dao::get_all()
        .await?
        .into_iter()
        .map(|x| x.into())
        .collect();
    Ok(res)
}

pub async fn register(
    email: &str,
    code: &str,
    pwd: &str,
    name: Option<&str>,
    mobile: Option<&str>,
) -> BasicResult<user_model::User> {
    private::validate_not_exist_email(email).await?;
    private::validate_pwd(pwd)?;
    private::validate_email_code(email, &user_model::SendEmailCodeFrom::Register, code).await?;
    if let Some(x) = mobile {
        private::validate_mobile(x)?;
    }
    let salt = private::salt();
    let pwd = private::hash_password(pwd, &salt);

    let current_user = pg_user_dao::insert(email, &salt, &pwd, name, mobile).await?;
    private::update_search(current_user.clone()).await?;
    Ok(current_user.into())
}

pub async fn login(email: &str, pwd: &str) -> BasicResult<String> {
    let mut user = private::validate_exist_email(email).await?;

    private::check_pwd(pwd, &user.salt, user.pwd.as_deref())?;
    let now = chrono::Utc::now();
    user.laston = Some(now);
    let user_copy = user.clone();

    let (_, _, set_current_user_result) = tokio::try_join!(
        pg_user_dao::update_laston(user.id, &now).map_err(|err| err.into()),
        private::update_search(user_copy),
        private::set_current_user(&user, &now),
    )?;

    return Ok(set_current_user_result);
}

pub async fn validate_not_exist_email(email: &str) -> BasicResult<()> {
    private::validate_not_exist_email(email).await
}

pub async fn validate_exist_email(email: &str) -> BasicResult<user_model::User> {
    Ok(private::validate_exist_email(email).await?.into())
}

pub async fn delete(ids: &[i64]) -> BasicResult<u64> {
    let (pg_del_res, _) = tokio::try_join!(
        pg_user_dao::delete(ids).map_err(|err| err.into()),
        meilisearch_dao::delete(meilisearch_dao::USER_LIST_INDEX, ids)
    )?;
    Ok(pg_del_res)
}

pub async fn change_pwd(email: &str, code: &str, new_pwd: &str) -> BasicResult<u64> {
    let user = validate_exist_email(email).await?;
    private::validate_email_code(email, &user_model::SendEmailCodeFrom::ChangePwd, code).await?;
    private::validate_pwd(new_pwd)?;

    let salt = private::salt();
    let pwd = private::hash_password(new_pwd, &salt);
    let res = pg_user_dao::update_pwd(user.id, &salt, &pwd).await?;

    Ok(res)
}

pub async fn send_email_code(
    email: &str,
    from: &user_model::SendEmailCodeFrom,
) -> BasicResult<usize> {
    match from {
        user_model::SendEmailCodeFrom::Register => validate_not_exist_email(email).await?,
        user_model::SendEmailCodeFrom::ChangePwd => {
            private::validate_exist_email(email).await?;
            ()
        }
    }

    if redis_user_dao::exist_email_code(email, from).await? {
        return hint_error!("the validation code has already send to your mail box, please check or resend after a few minutes").into();
    }

    let code = rand::thread_rng().gen_range(100000..999999);

    let expired_seconds = 120;

    let body = format!("the validation code is: {}", code);
    let (cache_code_res, send_code_res) = tokio::join!(
        redis_user_dao::set_email_code(email, from, code.to_string(), expired_seconds),
        email_util::send(email, "validation code", &body,)
    );
    let _ = send_code_res?;
    let _ = cache_code_res?;
    Ok(expired_seconds)
}

pub async fn update(
    id: i64,
    mobile: Option<&str>,
    name: Option<&str>,
) -> BasicResult<user_model::User> {
    if let Some(mobile) = mobile {
        private::validate_mobile(mobile)?;
    }
    let res = pg_user_dao::update(id, name, mobile).await?;
    private::update_search(res.clone()).await?;
    Ok(res.into())
}

pub fn check(token: &str) -> BasicResult<String> {
    private::check_token(token)
}

pub async fn get_current_user(email: &str) -> BasicResult<user_model::CurrentUser> {
    let res = redis_user_dao::get_current_user(email)
        .await
        .map_err(|err| unauthorized_error!(err))?;
    Ok(res)
}

pub async fn reload_search() -> BasicResult<()> {
    let data = get_all().await?;
    let documents = data
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<user_model::User>>();

    meilisearch_dao::reload(meilisearch_dao::USER_LIST_INDEX, &documents, Some("id")).await?;
    Ok(())
}
