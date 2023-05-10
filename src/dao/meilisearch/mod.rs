pub mod user;

use serde::Serialize;
use std::fmt::Display;
use utilities::error::BasicResult;
use utilities::meilisearch as meilisearch_util;
use utilities::meilisearch::Settings;

pub const USER_LIST_INDEX: &str = "user_list";

pub async fn reload<D>(index: &str, documents: &[D], primary_key: Option<&str>) -> BasicResult<()>
where
    D: Serialize,
{
    meilisearch_util::client()
        .index(index)
        .delete_all_documents()
        .await?;

    meilisearch_util::client()
        .index(index)
        .add_documents(documents, primary_key)
        .await?
        .wait_for_completion(meilisearch_util::client(), None, None)
        .await?;

    meilisearch_util::client()
        .index(index)
        .set_settings(&Settings::new().with_sortable_attributes(["created_at", "updated_at"]))
        .await?;

    Ok(())
}

pub async fn update<D>(index: &str, documents: &[D], primary_key: Option<&str>) -> BasicResult<()>
where
    D: Serialize,
{
    meilisearch_util::client()
        .index(index)
        .add_or_update(documents, primary_key)
        .await?
        .wait_for_completion(meilisearch_util::client(), None, None)
        .await?;
    Ok(())
}

pub async fn delete<T>(index: &str, ids: &[T]) -> BasicResult<()>
where
    T: Display + Serialize + std::fmt::Debug,
{
    meilisearch_util::client()
        .index(index)
        .delete_documents(ids)
        .await?
        .wait_for_completion(meilisearch_util::client(), None, None)
        .await?;

    Ok(())
}
