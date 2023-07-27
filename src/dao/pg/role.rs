use utilities::response::Pagination;
use chrono::{DateTime, Utc};
use utilities::postgres::{SqlResult, conn};

#[derive(Debug, Clone)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub async fn query(p: &Pagination) -> SqlResult<Vec<Role>> {
    sqlx::query_as!(
        Role,
        r#"
select 
    id,
    "name",
    description,
    created_at,
    updated_at,
    deleted_at 
from "role"
where deleted_at is null
order by created_at desc
limit $1 offset $2
    "#,
        p.take(),
        p.skip()
    )
    .fetch_all(conn().await)
    .await
}

pub async fn get(id: i64) -> SqlResult<Role> {
    sqlx::query_as!(
        Role,
        r#"
select 
    id,
    "name",
    description,
    created_at,
    updated_at,
    deleted_at
from "role" 
where id = $1
            "#,
        id,
    )
    .fetch_one(conn().await)
    .await
}

pub async fn insert(name: &str, description: Option<&str>) -> SqlResult<Role> {
    let created_at = chrono::Local::now();
    let res = sqlx::query_as!(
        Role,
        r#"
insert into "role" (name,description,created_at) values ($1,$2,$3) 
RETURNING 
id,
"name",
description,
created_at,
updated_at,
deleted_at           
            "#,
        name,
        description,
        created_at
    )
    .fetch_one(conn().await)
    .await?;

    Ok(res)
}

pub async fn update(id: i64, name: &str, description: Option<&str>) -> SqlResult<Role> {
    let updated_at = chrono::Local::now();
    sqlx::query_as!(
        Role,
        r#"
update "role" set name = $1, description = $2, updated_at=$3 where id = $4 RETURNING
id,
"name",
description,
created_at,
updated_at,
deleted_at  
    "#,
        name,
        description,
        updated_at,
        id
    )
    .fetch_one(conn().await)
    .await
}

pub async fn delete(ids: &[i64]) -> SqlResult<u64> {
    let deleted_at = chrono::Local::now();
    let res = sqlx::query!(
        r#"update "role" set deleted_at = $1 where id = ANY($2)"#,
        deleted_at,
        ids,
    )
    .execute(conn().await)
    .await?;

    Ok(res.rows_affected())
}
