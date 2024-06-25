use std::path::Path;

use base64::{engine::general_purpose, Engine as _};

use crate::provider::{AIProvider, Anthropic, OpenAI};

pub struct Client {
    provider: Box<dyn AIProvider>,
}

impl Client {
    pub fn new(model: &str) -> anyhow::Result<Self> {
        if model.starts_with("gpt") {
            return Ok(Self {
                provider: Box::new(OpenAI::default().with_model(model)),
            });
        }

        if model.starts_with("claude") {
            return Ok(Self {
                provider: Box::new(Anthropic::default().with_model(model)),
            });
        }

        Err(anyhow::anyhow!("unsupported model: {model}"))
    }

    pub fn message(self) -> MessageBuilder {
        MessageBuilder::new(self)
    }
}

pub struct MessageBuilder {
    client: Client,
    text: Option<String>,
    images: Vec<Image>,
}

impl MessageBuilder {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            text: None,
            images: Vec::new(),
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> anyhow::Result<Self> {
        let prompt_path = std::env::var("PROMPT_PATH")?;
        let prompt_file = Path::new(&prompt_path).join(format!("{}.txt", prompt.into()));
        let prompt = std::fs::read_to_string(prompt_file)?;

        self.text = Some(prompt);
        Ok(self)
    }

    pub fn image(mut self, data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        let data = general_purpose::STANDARD.encode(data);
        self.images.push(Image {
            data,
            mime_type: mime_type.into(),
        });
        self
    }

    // We currently support the base64 source type for images, and the image/jpeg, image/png,
    // image/gif, and image/webp media types.
    pub fn image_file(self, file: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let mime_type = match file.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => return Err(anyhow::anyhow!("unsupported image format")),
        };
        let data = std::fs::read(file)?;
        Ok(self.image(data, mime_type))
    }

    pub async fn send(self) -> anyhow::Result<Response> {
        let msg = Message {
            text: self.text.expect("text is required"),
            images: self.images,
        };

        self.client.provider.send_message(msg).await
    }
}

#[derive(Debug)]
pub struct Message {
    pub text: String,
    pub images: Vec<Image>,
}

#[derive(Debug)]
pub struct Image {
    pub data: String,
    pub mime_type: String,
}

#[derive(Debug)]
pub struct Response {
    pub text: String,
}

impl Response {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}
