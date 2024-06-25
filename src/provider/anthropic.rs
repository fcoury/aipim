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

pub struct Anthropic {
    client: Client,
    api_key: String,
    model: String,
}

impl Anthropic {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    pub fn with_model(self, model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..self
        }
    }
}

impl Default for Anthropic {
    fn default() -> Self {
        Self::new(
            std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY is not set"),
            MODELS[0],
        )
    }
}

#[async_trait]
impl AIProvider for Anthropic {
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
    pub fn is_error(&self) -> bool {
        matches!(self, Response::Error(_))
    }

    pub fn error(&self) -> &Error {
        match self {
            Response::Error(error) => error,
            _ => panic!("Response is not an error"),
        }
    }

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
