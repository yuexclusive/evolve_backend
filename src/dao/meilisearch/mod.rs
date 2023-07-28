pub mod user;

use serde::Serialize;
use std::fmt::Display;
use util_error::BasicResult;
use util_meilisearch::Settings;

pub const USER_LIST_INDEX: &str = "user_list";

pub async fn reload<D>(index: &str, documents: &[D], primary_key: Option<&str>) -> BasicResult<()>
where
    D: Serialize,
{
    util_meilisearch::client()
        .index(index)
        .delete_all_documents()
        .await?;

    util_meilisearch::client()
        .index(index)
        .add_documents(documents, primary_key)
        .await?
        .wait_for_completion(util_meilisearch::client(), None, None)
        .await?;

    util_meilisearch::client()
        .index(index)
        .set_settings(&Settings::new().with_sortable_attributes(["created_at", "updated_at"]))
        .await?;

    Ok(())
}

pub async fn update<D>(index: &str, documents: &[D], primary_key: Option<&str>) -> BasicResult<()>
where
    D: Serialize,
{
    util_meilisearch::client()
        .index(index)
        .add_or_update(documents, primary_key)
        .await?
        .wait_for_completion(util_meilisearch::client(), None, None)
        .await?;
    Ok(())
}

pub async fn delete<T>(index: &str, ids: &[T]) -> BasicResult<()>
where
    T: Display + Serialize + std::fmt::Debug,
{
    util_meilisearch::client()
        .index(index)
        .delete_documents(ids)
        .await?
        .wait_for_completion(util_meilisearch::client(), None, None)
        .await?;

    Ok(())
}
