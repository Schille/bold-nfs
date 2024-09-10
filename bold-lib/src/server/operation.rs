use async_trait::async_trait;

use super::{request::NfsRequest, response::NfsOpResponse};

#[async_trait]
pub trait NfsOperation: Sync {
    async fn execute(&self, mut request: NfsRequest) -> NfsOpResponse;
}
