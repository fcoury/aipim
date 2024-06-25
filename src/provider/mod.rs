use async_trait::async_trait;

mod anthropic;

pub use anthropic::Anthropic;

use crate::client::{Message, Response};

#[async_trait]
pub trait AIProvider {
    async fn send_message(&self, message: Message) -> anyhow::Result<Response>;
}
