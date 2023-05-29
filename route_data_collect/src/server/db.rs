use std::borrow::Borrow;

use bson::Document;
use mongodb::results::InsertOneResult;
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct AsyncDb {
    client: mongodb::Client,
}

impl AsyncDb {
    pub async fn try_from(value: &str) -> Result<AsyncDb, mongodb::error::Error> {
        Ok(AsyncDb {
            client: mongodb::Client::with_options(
                mongodb::options::ClientOptions::parse_async(value).await?,
            )?,
        })
    }

    pub async fn add_doc<T>(
        &self,
        db: &str,
        coll: &str,
        doc: T,
    ) -> Result<InsertOneResult, mongodb::error::Error>
    where
        T: Serialize + Borrow<Document>,
    {
        self.client
            .database(db)
            .collection::<Document>(coll)
            .insert_one(doc, None)
            .await
    }
}
