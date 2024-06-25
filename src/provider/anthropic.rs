#![allow(unused)]
use async_trait::async_trait;
use log::{debug, trace};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::AIProvider;
use crate::client;

const MAX_TOKENS: u32 = 1024;
const ANTRHOPIC_VERSION: &str = "2023-06-01";
const MODELS: &[&str] = &[
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
];

const BASE_URL: &str = "https://api.anthropic.com/v1/";

/// Represents the Anthropic AI provider.
///
/// # Examples
///
/// ```
/// use crate::provider::anthropic::Anthropic;
///
/// let api_key = "your_api_key";
/// let model = "claude-3-5-sonnet-20240620";
/// let anthropic = Anthropic::new(api_key, model);
/// ```
pub struct Anthropic {
    client: Client,
    api_key: String,
    model: String,
}

impl Anthropic {
    /// Creates a new `Anthropic` instance.
    ///
    /// # Arguments
    ///
    /// * `api_key` - A string slice that holds the API key.
    /// * `model` - A string slice that holds the name of the model.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::Anthropic;
    ///
    /// let api_key = "your_api_key";
    /// let model = "claude-3-5-sonnet-20240620";
    /// let anthropic = Anthropic::new(api_key, model);
    /// ```
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Sets the model for the `Anthropic` instance.
    ///
    /// # Arguments
    ///
    /// * `model` - A string slice that holds the name of the model.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::Anthropic;
    ///
    /// let api_key = "your_api_key";
    /// let anthropic = Anthropic::new(api_key, "claude-3-5-sonnet-20240620");
    /// let updated_anthropic = anthropic.with_model("claude-3-opus-20240229");
    /// ```
    pub fn with_model(self, model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..self
        }
    }
}

impl Default for Anthropic {
    /// Creates a default `Anthropic` instance using the `ANTHROPIC_API_KEY` environment variable.
    ///
    /// # Panics
    ///
    /// Panics if the `ANTHROPIC_API_KEY` environment variable is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::Anthropic;
    ///
    /// std::env::set_var("ANTHROPIC_API_KEY", "your_api_key");
    /// let anthropic = Anthropic::default();
    /// ```
    fn default() -> Self {
        Self::new(
            std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY is not set"),
            MODELS[0],
        )
    }
}

#[async_trait]
impl AIProvider for Anthropic {
    /// Sends a message to the Anthropic API.
    ///
    /// # Arguments
    ///
    /// * `message` - A `client::Message` instance containing the message to be sent.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response contains an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::Anthropic;
    /// use crate::client::{Client, Message};
    ///
    /// let api_key = "your_api_key";
    /// let model = "claude-3-5-sonnet-20240620";
    /// let anthropic = Anthropic::new(api_key, model);
    ///
    /// let message = Message {
    ///     text: "Hello, world!".to_string(),
    ///     images: vec![],
    /// };
    ///
    /// let response = anthropic.send_message(message).await;
    /// match response {
    ///     Ok(res) => println!("Response: {:?}", res),
    ///     Err(err) => eprintln!("Error: {:?}", err),
    /// }
    /// ```
    async fn send_message(&self, message: client::Message) -> anyhow::Result<client::Response> {
        let mut content = vec![Content::Text(Text {
            typ: "text".to_string(),
            text: message.text,
        })];

        for image in message.images {
            content.push(Content::Image(Image {
                typ: "image".to_string(),
                source: ImageData {
                    typ: "base64".to_string(),
                    media_type: image.mime_type,
                    data: image.data,
                },
            }));
        }

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content,
        }];

        let request = Request {
            model: self.model.clone(),
            max_tokens: MAX_TOKENS as usize,
            messages,
        };

        trace!(
            "JSON Request: {}",
            serde_json::to_string_pretty(&request).unwrap()
        );
        let url = format!("{}messages", BASE_URL);
        trace!("Request URL: {}", url);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("anthropic-version", ANTRHOPIC_VERSION)
            .header("x-api-key", &self.api_key)
            .json(&request)
            .send()
            .await?;

        let response: serde_json::Value = response.json().await?;
        trace!(
            "JSON Response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );

        let response = serde_json::from_value::<Response>(response)?;
        debug!("Anthropic Response: {:#?}", response);

        if response.is_error() {
            return Err(anyhow::anyhow!(response.error().error.message.clone()));
        }

        Ok(client::Response::new(response.text()))
    }
}

#[derive(Serialize, Debug)]
struct Request {
    model: String,
    max_tokens: usize,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize, Debug)]
struct ChatMessage {
    role: String,
    content: Vec<Content>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Content {
    Text(Text),
    Image(Image),
}

impl Content {
    /// Returns the text content of the `Content` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::Content;
    ///
    /// let text_content = Content::Text(Text {
    ///     typ: "text".to_string(),
    ///     text: "Hello, world!".to_string(),
    /// });
    ///
    /// assert_eq!(text_content.as_text(), "Hello, world!".to_string());
    /// ```
    fn as_text(&self) -> String {
        match self {
            Content::Text(c) => c.text.clone(),
            Content::Image(_) => "[image]".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Text {
    #[serde(rename = "type")]
    typ: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Image {
    #[serde(rename = "type")]
    typ: String,
    source: ImageData,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImageData {
    #[serde(rename = "type")]
    typ: String,
    media_type: String,
    data: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Response {
    Message(Message),
    Error(Error),
}

impl Response {
    /// Checks if the response is an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::Response;
    ///
    /// let error_response = Response::Error(Error {
    ///     typ: "error".to_string(),
    ///     error: ErrorDetails {
    ///         typ: "invalid_request_error".to_string(),
    ///         message: "Invalid request".to_string(),
    ///     },
    /// });
    ///
    /// assert!(error_response.is_error());
    /// ```
    pub fn is_error(&self) -> bool {
        matches!(self, Response::Error(_))
    }

    /// Returns the error details of the response.
    ///
    /// # Panics
    ///
    /// Panics if the response is not an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::{Response, Error, ErrorDetails};
    ///
    /// let error_response = Response::Error(Error {
    ///     typ: "error".to_string(),
    ///     error: ErrorDetails {
    ///         typ: "invalid_request_error".to_string(),
    ///         message: "Invalid request".to_string(),
    ///     },
    /// });
    ///
    /// let error = error_response.error();
    /// assert_eq!(error.message, "Invalid request".to_string());
    /// ```
    pub fn error(&self) -> &Error {
        match self {
            Response::Error(error) => error,
            _ => panic!("Response is not an error"),
        }
    }

    /// Returns the text content of the response.
    ///
    /// # Panics
    ///
    /// Panics if the response is not a message.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::provider::anthropic::{Response, Message, Content, Text, Usage};
    ///
    /// let message_response = Response::Message(Message {
    ///     id: "msg_123".to_string(),
    ///     typ: "message".to_string(),
    ///     model: "claude-3-5-sonnet-20240620".to_string(),
    ///     role: "assistant".to_string(),
    ///     stop_reason: "end_turn".to_string(),
    ///     stop_sequence: None,
    ///     usage: Usage {
    ///         input_tokens: 10,
    ///         output_tokens: 25,
    ///     },
    ///     content: vec![Content::Text(Text {
    ///         typ: "text".to_string(),
    ///         text: "Hello, world!".to_string(),
    ///     })],
    /// });
    ///
    /// assert_eq!(message_response.text(), "Hello, world!".to_string());
    /// ```
    pub fn text(&self) -> String {
        match self {
            Response::Message(message) => message.content[0].as_text(),
            _ => panic!("Response is not a message"),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Message {
    id: String,
    #[serde(rename = "type")]
    typ: String,
    model: String,
    role: String,
    stop_reason: String,
    stop_sequence: Option<String>,
    usage: Usage,
    content: Vec<Content>,
}

#[derive(Deserialize, Debug)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct Error {
    #[serde(rename = "type")]
    typ: String,
    error: ErrorDetails,
}

#[derive(Deserialize, Debug)]
struct ErrorDetails {
    #[serde(rename = "type")]
    typ: String,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response() {
        /// Tests parsing a successful response.
        ///
        /// # Examples
        ///
        /// ```
        /// use crate::provider::anthropic::{Response, Message, Content, Text, Usage};
        ///
        /// let response = serde_json::from_str::<Response>(
        ///     r#"
        ///         {
        ///           "content": [
        ///             {
        ///               "text": "Hi! My name is Claude.",
        ///               "type": "text"
        ///             }
        ///           ],
        ///           "id": "msg_013Zva2CMHLNnXjNJJKqJ2EF",
        ///           "model": "claude-3-5-sonnet-20240620",
        ///           "role": "assistant",
        ///           "stop_reason": "end_turn",
        ///           "stop_sequence": null,
        ///           "type": "message",
        ///           "usage": {
        ///             "input_tokens": 10,
        ///             "output_tokens": 25
        ///           }
        ///         }
        ///     "#,
        /// ).unwrap();
        /// assert!(!response.is_error());
        /// ```
        let response = serde_json::from_str::<Response>(
            r#"
                {
                  "content": [
                    {
                      "text": "Hi! My name is Claude.",
                      "type": "text"
                    }
                  ],
                  "id": "msg_013Zva2CMHLNnXjNJJKqJ2EF",
                  "model": "claude-3-5-sonnet-20240620",
                  "role": "assistant",
                  "stop_reason": "end_turn",
                  "stop_sequence": null,
                  "type": "message",
                  "usage": {
                    "input_tokens": 10,
                    "output_tokens": 25
                  }
                }
            "#,
        )
        .unwrap();
        assert!(!response.is_error());
    }

    #[test]
    fn test_parse_error() {
        /// Tests parsing an error response.
        ///
        /// # Examples
        ///
        /// ```
        /// use crate::provider::anthropic::{Response, Error, ErrorDetails};
        ///
        /// let response = serde_json::from_str::<Response>(
        ///     r#"
        ///         {
        ///           "type": "error",
        ///           "error": {
        ///             "type": "invalid_request_error",
        ///             "message": "<string>"
        ///           }
        ///         }
        ///     "#,
        /// ).unwrap();
        /// assert!(response.is_error());
        /// ```
        let response = serde_json::from_str::<Response>(
            r#"
                {
                  "type": "error",
                  "error": {
                    "type": "invalid_request_error",
                    "message": "<string>"
                  }
                }
            "#,
        )
        .unwrap();
        assert!(response.is_error());
    }
}
