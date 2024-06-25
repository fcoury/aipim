use async_trait::async_trait;
use log::{debug, trace};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::client;

use super::AIProvider;

const MAX_TOKENS: u32 = 4096;
const BASE_URL: &str = "https://api.openai.com/v1/";
const MODELS: &[&str] = &["gpt-4o", "gpt-4-turbo", "gpt-4", "gpt-3.5-turbo"];

pub struct OpenAI {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAI {
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

impl Default for OpenAI {
    fn default() -> Self {
        Self::new(
            std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set"),
            MODELS[0],
        )
    }
}

#[async_trait]
impl AIProvider for OpenAI {
    async fn send_message(&self, message: client::Message) -> anyhow::Result<client::Response> {
        let mut content = Content::Complex(vec![ComplexContent::Text(Text {
            typ: "text".to_string(),
            text: message.text,
        })]);

        for image in message.images {
            content.push(ComplexContent::Image(Image {
                typ: "image_url".to_string(),
                image_url: ImageUrl {
                    url: format!("data:image/jpeg;base64,{}", image.data),
                },
            }));
        }

        let chat_message = ChatMessage {
            role: "user".to_string(),
            content,
        };

        let request = Request {
            model: self.model.clone(),
            messages: vec![chat_message],
            max_tokens: MAX_TOKENS as usize,
        };

        trace!(
            "JSON Request: {}",
            serde_json::to_string_pretty(&request).unwrap()
        );

        let response = self
            .client
            .post(&format!("{}chat/completions", BASE_URL))
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        let response: serde_json::Value = response.json().await?;
        trace!(
            "JSON Response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );

        let response = serde_json::from_value::<Response>(response)?;
        debug!("OpenAI Response: {:#?}", response);

        match response {
            Response::Message(message) => {
                let content = &message.choices[0].message.content;
                let text = content.as_text().ok_or_else(|| {
                    anyhow::anyhow!("unsupported response content type: {:?}", content)
                })?;

                Ok(client::Response::new(text))
            }
            Response::Error { error } => {
                let code = if let Some(code) = error.code {
                    format!("{}: ", code)
                } else {
                    "".to_string()
                };
                Err(anyhow::anyhow!(
                    "{}{} ({})",
                    code,
                    error.message,
                    error.param
                ))
            }
        }
    }
}

#[derive(Serialize, Debug)]
struct Request {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    role: String,
    content: Content,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Content {
    Simple(String),
    Complex(Vec<ComplexContent>),
}

impl Content {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Simple(text) => Some(text),
            _ => None,
        }
    }

    pub fn push(&mut self, content: ComplexContent) {
        if let Content::Complex(vec) = self {
            vec.push(content);
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ComplexContent {
    Text(Text),
    Image(Image),
}

#[derive(Serialize, Deserialize, Debug)]
struct Text {
    text: String,
    #[serde(rename = "type")]
    typ: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Image {
    image_url: ImageUrl,
    #[serde(rename = "type")]
    typ: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImageUrl {
    url: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Response {
    Message(Message),
    Error { error: Error },
}

#[derive(Deserialize, Debug)]
struct Message {
    id: String,
    object: String,
    created: i64,
    model: String,
    system_fingerprint: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Deserialize, Debug)]
struct Error {
    code: Option<String>,
    message: String,
    param: String,
    #[serde(rename = "type")]
    typ: String,
}

#[derive(Deserialize, Debug)]
struct Choice {
    index: usize,
    message: ChatMessage,
    logprobs: Option<bool>,
    finish_reason: String,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let res = r#"
        {
          "choices": [
            {
              "finish_reason": "length",
              "index": 0,
              "logprobs": null,
              "message": {
                "content": "response",
                "role": "assistant"
              }
            }
          ],
          "created": 1719328775,
          "id": "chatcmpl-9e2FDY8pjRfZqufnqa4XSu5f26aUy",
          "model": "gpt-4o-2024-05-13",
          "object": "chat.completion",
          "system_fingerprint": "fp_8c6b918852",
          "usage": {
            "completion_tokens": 1024,
            "prompt_tokens": 1563,
            "total_tokens": 2587
          }
        }
        "#;
        let response = serde_json::from_str::<Response>(res).unwrap();
        println!("{:#?}", response);
    }

    #[test]
    fn test_parse_error() {
        let error = r#"
            {
              "error": {
                "code": null,
                "message": "Invalid content type. image_url is only supported by certain models.",
                "param": "messages.[0].content.[1].type",
                "type": "invalid_request_error"
              }
            }
        "#;
        let response = serde_json::from_str::<Response>(error).unwrap();
        let Response::Error { error } = response else {
            panic!("expected error response, got: {:?}", response);
        };
        assert!(error.code.is_none());
    }

    #[test]
    fn test_as_text() {
        let simple = Content::Simple("text".to_string());
        assert_eq!(simple.as_text(), Some("text"));

        let complex = Content::Complex(vec![ComplexContent::Text(Text {
            typ: "text".to_string(),
            text: "text".to_string(),
        })]);
        assert_eq!(complex.as_text(), None);
    }
}
