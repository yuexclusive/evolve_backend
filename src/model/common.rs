use serde::Deserialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[derive(Deserialize)]
pub struct MsgResponse<'a> {
    pub msg: &'a str,
}

#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[derive(Deserialize)]
pub struct ErrorResponse<'a> {
    pub msg: &'a str,
}

#[cfg_attr(feature = "openapi", derive(utoipa::IntoParams))]
#[derive(Deserialize)]
pub struct Pagination {
    pub index: i64,
    pub size: i64,
}

impl Pagination {
    pub fn skip(&self) -> i64 {
        self.index.checked_sub(1).unwrap_or(0) * self.size
    }

    pub fn take(&self) -> i64 {
        self.size.max(0)
    }
}
