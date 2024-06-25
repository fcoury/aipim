use std::path::Path;

use base64::{engine::general_purpose, Engine as _};

use crate::provider::{AIProvider, Anthropic, OpenAI};

/// The `Client` struct is responsible for interacting with different AI providers.
///
/// # Examples
///
/// ```
/// use your_crate::client::Client;
///
/// let client = Client::new("gpt-3.5-turbo").unwrap();
/// let response = client.message().text("Hello, world!").send().await.unwrap();
/// println!("{}", response.text);
/// ```
pub struct Client {
    provider: Box<dyn AIProvider>,
}

impl Client {
    /// Creates a new `Client` instance based on the provided model.
    ///
    /// # Arguments
    ///
    /// * `model` - A string slice that holds the name of the model.
    ///
    /// # Errors
    ///
    /// Returns an error if the model is unsupported.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::Client;
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// ```
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

    /// Returns a `MessageBuilder` to construct a message.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::Client;
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let builder = client.message();
    /// ```
    pub fn message(self) -> MessageBuilder {
        MessageBuilder::new(self)
    }
}

/// The `MessageBuilder` struct is used to build messages to be sent to the AI provider.
///
/// # Examples
///
/// ```
/// use your_crate::client::{Client, MessageBuilder};
///
/// let client = Client::new("gpt-3.5-turbo").unwrap();
/// let builder = client.message().text("Hello, world!");
/// ```
pub struct MessageBuilder {
    client: Client,
    text: Option<String>,
    images: Vec<Image>,
}

impl MessageBuilder {
    /// Creates a new `MessageBuilder` instance.
    ///
    /// # Arguments
    ///
    /// * `client` - A `Client` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::{Client, MessageBuilder};
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let builder = MessageBuilder::new(client);
    /// ```
    pub fn new(client: Client) -> Self {
        Self {
            client,
            text: None,
            images: Vec::new(),
        }
    }

    /// Sets the text for the message.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::{Client, MessageBuilder};
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let builder = client.message().text("Hello, world!");
    /// ```
    #[allow(unused)]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Sets the text for the message from a prompt file.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The name of the prompt file (without extension).
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt file cannot be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::{Client, MessageBuilder};
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let builder = client.message().prompt("greeting").unwrap();
    /// ```
    pub fn prompt(mut self, prompt: impl Into<String>) -> anyhow::Result<Self> {
        let prompt_path = std::env::var("PROMPT_PATH")?;
        let prompt_file = Path::new(&prompt_path).join(format!("{}.txt", prompt.into()));
        let prompt = std::fs::read_to_string(prompt_file)?;

        self.text = Some(prompt);
        Ok(self)
    }

    /// Adds an image to the message.
    ///
    /// # Arguments
    ///
    /// * `data` - The image data as a byte vector.
    /// * `mime_type` - The MIME type of the image.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::{Client, MessageBuilder};
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let image_data = vec![/* image data */];
    /// let builder = client.message().image(image_data, "image/png");
    /// ```
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
    /// Adds an image to the message from a file.
    ///
    /// # Arguments
    ///
    /// * `file` - The path to the image file.
    ///
    /// # Errors
    ///
    /// Returns an error if the image format is unsupported or the file cannot be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::{Client, MessageBuilder};
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let builder = client.message().image_file("path/to/image.png").unwrap();
    /// ```
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

    /// Sends the message to the AI provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::{Client, MessageBuilder};
    ///
    /// let client = Client::new("gpt-3.5-turbo").unwrap();
    /// let response = client.message().text("Hello, world!").send().await.unwrap();
    /// println!("{}", response.text);
    /// ```
    pub async fn send(self) -> anyhow::Result<Response> {
        let msg = Message {
            text: self.text.expect("text is required"),
            images: self.images,
        };

        self.client.provider.send_message(msg).await
    }
}

#[derive(Debug)]
/// The `Message` struct represents a message to be sent to the AI provider.
pub struct Message {
    pub text: String,
    pub images: Vec<Image>,
}

#[derive(Debug)]
/// The `Image` struct represents an image to be sent to the AI provider.
pub struct Image {
    pub data: String,
    pub mime_type: String,
}

#[derive(Debug)]
/// The `Response` struct represents a response from the AI provider.
pub struct Response {
    pub text: String,
}

impl Response {
    /// Creates a new `Response` instance.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content of the response.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::client::Response;
    ///
    /// let response = Response::new("Hello, world!");
    /// println!("{}", response.text);
    /// ```
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}
