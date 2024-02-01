use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use util_redis::derive::{from_redis, to_redis};
use utoipa::ToSchema;

#[derive(ToSchema, sqlx::Type, Debug, Serialize, Deserialize, Clone)]
// #[sqlx(rename_all = "snake_case")]
// #[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum UserType {
    Normal,
    Admin,
    SuperAdmin,
}

#[derive(ToSchema, sqlx::Type, Debug, Serialize, Deserialize, Clone)]
// #[sqlx(rename_all = "snake_case")]
// #[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum UserStatus {
    Available,
    Disabled,
}

#[derive(ToSchema, Deserialize, Debug)]
pub enum SendEmailCodeFrom {
    Register,
    ChangePwd,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: i64,
    pub r#type: UserType,
    pub email: String,
    pub status: UserStatus,
    pub name: Option<String>,
    pub mobile: Option<String>,
    pub laston: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct UserFormatter {
    pub r#type: String,
    pub email: String,
    pub status: String,
    pub name: String,
    pub mobile: String,
    pub laston: String,
    pub created_at: String,
    pub updated_at: String,
}

pub trait GetValFromMap {
    fn get_val(&self, name: &str) -> String;
}

impl GetValFromMap for Map<String, Value> {
    fn get_val(&self, name: &str) -> String {
        self.get(name)
            .map(|x| {
                if x.is_null() {
                    return String::from("");
                }
                x.to_string()
                    .trim_matches('"')
                    .replace("\\", "")
                    .to_string()
                // x.to_string()
            })
            .unwrap_or("".to_string())
    }
}

impl From<Map<String, Value>> for UserFormatter {
    fn from(map: Map<String, Value>) -> Self {
        Self {
            r#type: map.get_val("type"),
            email: map.get_val("email"),
            status: map.get_val("status"),
            name: map.get_val("name"),
            mobile: map.get_val("mobile"),
            laston: map.get_val("laston"),
            created_at: map.get_val("created_at"),
            updated_at: map.get_val("updated_at"),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct SearchedUser {
    pub user: User,
    pub formatter: UserFormatter,
    // formatter: Map<String, Value>,
}

// #[derive(Debug, Clone, Serialize, Deserialize, ToRedisArgs, FromRedisValue, ToSchema)]
#[derive(ToSchema)]
#[to_redis]
#[from_redis]
pub struct CurrentUser {
    pub id: i64,
    pub r#type: UserType,
    pub email: String,
    pub status: UserStatus,
    pub name: Option<String>,
    pub mobile: Option<String>,
    pub laston: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub expire_at: String,
}

#[derive(ToSchema, Deserialize)]
pub struct LoginReq {
    #[schema(example = "yu.exclusive@icloud.com")]
    pub email: String,
    #[schema(example = "a111111")]
    pub pwd: String,
}

#[derive(ToSchema, Deserialize)]
pub struct SendEmailCodeReq {
    pub email: String,
    pub from: SendEmailCodeFrom,
}

#[derive(ToSchema, Deserialize)]
pub struct ChangePasswordReq {
    /// email
    pub email: String,
    /// validate code
    pub code: String,
    /// password
    pub pwd: String,
}

#[derive(ToSchema, Deserialize)]
pub struct LoginDataResponse {
    pub data: String,
}

#[derive(ToSchema, Deserialize)]
pub struct CurrentUserResponse {
    pub data: CurrentUser,
}

#[derive(ToSchema, Deserialize)]
pub struct UserGetResponse {
    pub data: User,
}

#[derive(ToSchema, Deserialize)]
pub struct UserSearchResponse {
    pub data: Vec<SearchedUser>,
    pub total: usize,
}

#[derive(ToSchema, Deserialize)]
pub struct SendEmailResponse {
    pub data: usize,
}

#[derive(ToSchema, Deserialize)]
pub struct UserUpdateReq {
    pub id: i64,
    pub name: Option<String>,
    pub mobile: Option<String>,
}

#[derive(ToSchema, Deserialize)]
pub struct UserUpdateResponse {
    pub data: User,
}

#[derive(ToSchema, Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub pwd: String,
    pub code: String,
    pub name: Option<String>,
    pub mobile: Option<String>,
}

#[derive(ToSchema, Deserialize)]
pub struct UserDeleteReq {
    pub ids: Vec<i64>,
}
