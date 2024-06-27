use async_trait::async_trait;

mod anthropic;
mod google;
mod openai;

pub use anthropic::Anthropic;
pub use google::Google;
pub use openai::OpenAI;

use crate::client::{Message, Response};

#[async_trait]
pub trait AIProvider {
    async fn send_message(&self, message: Message) -> anyhow::Result<Response>;
}
