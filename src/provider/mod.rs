use async_trait::async_trait;

mod anthropic;
mod openai;

pub use anthropic::Anthropic;
pub use openai::OpenAI;

use crate::client::{Message, Response};

#[async_trait]
pub trait AIProvider {
    async fn send_message(&self, message: Message) -> anyhow::Result<Response>;
}
