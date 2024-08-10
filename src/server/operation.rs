use async_trait::async_trait;

use super::{request::NfsRequest, response::NfsResponse};

#[async_trait]
pub trait NfsOperation: Sync {
    async fn execute(&self, mut request: NfsRequest) -> NfsResponse;
}
