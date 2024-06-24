use async_trait::async_trait;

#[async_trait]
pub trait AIProvider {
    async fn send_message(&self, message: String) -> anyhow::Result<String>;
}

pub enum AIProviderType {
    Claude3Haiku,
    Claude3Opus,
    Claude35Sonnet,
}

pub struct Client {
    provider: Box<dyn AIProvider>,
}

pub struct Message {
    pub text: String,
    pub images: Vec<Image>,
}

pub struct Image {
    data: Vec<u8>,
    mime_type: String,
}

pub struct Response {
    text: String,
}
