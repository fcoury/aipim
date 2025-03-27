#![allow(unused)]
use async_trait::async_trait;
use log::{debug, trace};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::client;

use super::AIProvider;

const MAX_TOKENS: u32 = 32768; // Increased token limit for Gemini 2.0 models
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";
const MODELS: &[&str] = &[
    // Gemini 1.0 models
    "gemini-1.5-pro",
    // Gemini 2.0 models
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    // Gemini 2.5 models
    "gemini-2.5-pro-exp-03-25",
];

/// Represents a Google Gemini client for interacting with the Gemini API.
pub struct Google {
    client: Client,
    api_key: String,
    model: String,
}

impl Google {
    /// Creates a new `Google` instance.
    ///
    /// # Arguments
    ///
    /// * `api_key` - A string slice that holds the API key.
    /// * `model` - A string slice that holds the name of the model.
    ///
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Sets the model for the `Google` instance.
    ///
    /// # Arguments
    ///
    /// * `model` - A string slice that holds the name of the model.
    ///
    pub fn with_model(self, model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..self
        }
    }

    /// Validates if a model name is supported by the Gemini API.
    ///
    /// # Arguments
    ///
    /// * `model_name` - A string slice that holds the name of the model to validate.
    ///
    /// # Returns
    ///
    /// Returns `true` if the model is supported, `false` otherwise.
    ///
    pub fn is_valid_model(model_name: &str) -> bool {
        MODELS.contains(&model_name)
    }
}

impl Default for Google {
    fn default() -> Self {
        Self::new(
            std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY is not set"),
            MODELS[2], // Default to gemini-2.0-pro
        )
    }
}

#[async_trait]
impl AIProvider for Google {
    /// Sends a message to the Gemini API.
    ///
    /// # Arguments
    ///
    /// * `message` - A `client::Message` instance containing the message to be sent.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response contains an error.
    ///
    async fn send_message(&self, message: client::Message) -> anyhow::Result<client::Response> {
        let request = build_request(message, self.model.as_str());
        log::info!(
            "JSON Request: {}",
            serde_json::to_string_pretty(&request).unwrap()
        );

        let url = format!(
            "{}models/{}:generateContent?key={}",
            BASE_URL, self.model, self.api_key
        );
        log::info!("url: {}", url);
        let response = self.client.post(&url).json(&request).send().await?;

        let response: serde_json::Value = response.json().await?;
        log::info!(
            "JSON Response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );

        let response = serde_json::from_value::<Response>(response)?;
        debug!("Google Response: {:#?}", response);

        match response {
            Response::Success(success) => {
                let content = &success.candidates[0].content;
                let text = content.parts[0].as_text().ok_or_else(|| {
                    anyhow::anyhow!("unsupported response content type: {:?}", content)
                })?;

                Ok(client::Response::new(text.to_string()))
            }
            Response::Error { error } => Err(anyhow::anyhow!(
                "{}: {} ({})",
                error.status,
                error.message,
                error.code
            )),
        }
    }
}

fn build_request(message: client::Message, model: &str) -> Request {
    let mut content = Content {
        parts: vec![Part::Text(TextPart { text: message.text })],
        role: "user".to_string(),
    };

    if let Some(images) = message.images {
        for image in images {
            content.parts.insert(
                0,
                Part::InlineData(InlineData {
                    inline_data: Blob {
                        mime_type: image.mime_type,
                        data: image.data,
                    },
                }),
            );
        }
    }

    // Adjust generation config based on model version
    let max_tokens = if model.starts_with("gemini-2") {
        32768 // Higher token limit for Gemini 2.x models
    } else {
        8192 // Original token limit for older models
    };

    Request {
        contents: vec![content],
        safety_settings: vec![],
        generation_config: GenerationConfig {
            temperature: 0.9,
            top_p: 1.0,
            top_k: 1,
            max_output_tokens: max_tokens,
            response_mime_type: Some("text/plain".to_string()), // Explicitly request text response
        },
        system_instruction: if model.starts_with("gemini-2") {
            // System instructions are supported in Gemini 2.x models
            Some(SystemInstruction {
                parts: vec![Part::Text(TextPart {
                    text: "You are a helpful AI assistant.".to_string(),
                })],
            })
        } else {
            None
        },
    }
}

unsafe impl Send for Google {}
unsafe impl Sync for Google {}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents a request to the Gemini API.
struct Request {
    contents: Vec<Content>,
    safety_settings: Vec<SafetySetting>,
    generation_config: GenerationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents system instructions for Gemini 2.x models.
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents the content of a message.
struct Content {
    parts: Vec<Part>,
    role: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
/// Represents different parts of a message.
enum Part {
    Text(TextPart),
    InlineData(InlineData),
}

impl Part {
    fn as_text(&self) -> Option<&str> {
        match self {
            Part::Text(part) => Some(&part.text),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents text in a message.
struct TextPart {
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents inline data (e.g., images) in a message.
struct InlineData {
    inline_data: Blob,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Blob {
    mime_type: String,
    data: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents safety settings for content generation.
struct SafetySetting {
    category: String,
    threshold: String,
}

#[derive(Serialize, Debug)]
/// Represents configuration for content generation.
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    top_p: f32,
    top_k: u32,
    max_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
/// Represents a response from the Gemini API.
enum Response {
    Success(SuccessResponse),
    Error { error: ErrorResponse },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents a successful response from the Gemini API.
struct SuccessResponse {
    candidates: Vec<Candidate>,
    model_version: Option<String>,
    usage_metadata: Option<UsageMetadata>,
    prompt_feedback: Option<PromptFeedback>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents a candidate response from the Gemini API.
struct Candidate {
    content: Content,
    finish_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_ratings: Option<Vec<SafetyRating>>,
    avg_logprobs: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents usage metadata for the response.
struct UsageMetadata {
    candidates_token_count: u32,
    prompt_token_count: u32,
    total_token_count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents feedback on the prompt.
struct PromptFeedback {
    safety_ratings: Vec<SafetyRating>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// Represents a safety rating for content.
struct SafetyRating {
    category: String,
    probability: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// Represents an error response from the Gemini API.
struct ErrorResponse {
    code: u32,
    message: String,
    status: String,
}

#[cfg(test)]
/// Unit tests for the Google module.
mod tests {
    use super::*;

    #[test]
    /// Tests creating a new `Google` instance.
    fn test_gemini_new() {
        let api_key = "test_api_key";
        let model = "gemini-2.0-pro";
        let gemini = Google::new(api_key, model);
        assert_eq!(gemini.api_key, api_key);
        assert_eq!(gemini.model, model);
    }

    #[test]
    /// Tests setting the model for a `Google` instance.
    fn test_gemini_with_model() {
        let api_key = "test_api_key";
        let model = "gemini-2.0-pro";
        let new_model = "gemini-2.5-pro";
        let gemini = Google::new(api_key, model).with_model(new_model);
        assert_eq!(gemini.model, new_model);
    }

    #[test]
    fn test_stringify_part() {
        let part = Part::Text(TextPart {
            text: "Hello, world!".to_string(),
        });
        assert_eq!(part.as_text(), Some("Hello, world!"));
    }

    #[test]
    /// Tests parsing a successful response from the Gemini API.
    fn test_parse_success() {
        let res = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {
                    "text": "Hello! How can I assist you today?"
                  }
                ],
                "role": "model"
              },
              "finishReason": "STOP",
              "index": 0,
              "safetyRatings": [
                {
                  "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                  "probability": "NEGLIGIBLE"
                }
              ]
            }
          ],
          "promptFeedback": {
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        }
        "#;
        let response = serde_json::from_str::<Response>(res).unwrap();
        if let Response::Success(success) = response {
            assert_eq!(
                success.candidates[0].content.parts[0].as_text(),
                Some("Hello! How can I assist you today?")
            );
        } else {
            panic!("Expected successful response");
        }
    }

    #[test]
    /// Tests parsing an error response from the Gemini API.
    fn test_parse_error() {
        let error = r#"
        {
          "error": {
            "code": 400,
            "message": "Invalid argument: 'model'.",
            "status": "INVALID_ARGUMENT"
          }
        }
        "#;
        let response = serde_json::from_str::<Response>(error).unwrap();
        if let Response::Error { error } = response {
            assert_eq!(error.code, 400);
            assert_eq!(error.message, "Invalid argument: 'model'.");
            assert_eq!(error.status, "INVALID_ARGUMENT");
        } else {
            panic!("Expected error response");
        }
    }

    #[test]
    fn test_build_request_gemini_1() {
        let message = client::Message {
            text: "Hello, world!".to_string(),
            images: None,
            model: None,
        };
        let request = build_request(message, "gemini-1.5-pro");
        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.contents[0].parts.len(), 1);
        assert_eq!(
            request.contents[0].parts[0].as_text(),
            Some("Hello, world!")
        );
        assert!(request.system_instruction.is_none());
    }

    #[test]
    fn test_build_request_gemini_2() {
        let message = client::Message {
            text: "Hello, world!".to_string(),
            images: None,
            model: None,
        };
        let request = build_request(message, "gemini-2.0-pro");
        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.contents[0].parts.len(), 1);
        assert_eq!(
            request.contents[0].parts[0].as_text(),
            Some("Hello, world!")
        );
        assert!(request.system_instruction.is_some());
    }

    #[test]
    fn test_build_request_with_images() {
        let message = client::Message {
            text: "Hello, world!".to_string(),
            images: Some(vec![client::Image {
                data: "data".to_string(),
                mime_type: "image/png".to_string(),
            }]),
            model: None,
        };
        let request = build_request(message, "gemini-2.0-pro-vision");
        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.contents[0].parts.len(), 2);
        assert_eq!(request.contents[0].parts[0].as_text(), None);
        assert_eq!(
            request.contents[0].parts[1].as_text(),
            Some("Hello, world!")
        );
    }

    #[test]
    fn test_deserialize_gemini_2_response() {
        let response = r#"
        {
          "candidates": [
            {
              "avgLogprobs": -0.5267344580756294,
              "content": {
                "parts": [
                  {
                    "text": "Based on the content of the fax, a suitable name for the queue could be:\n\n*   **Neurology One - Requisitions** or **Neurology One - Orders**."
                  }
                ],
                "role": "model"
              },
              "finishReason": "STOP"
            }
          ],
          "modelVersion": "gemini-2.0-flash",
          "usageMetadata": {
            "candidatesTokenCount": 36,
            "candidatesTokensDetails": [
              {
                "modality": "TEXT",
                "tokenCount": 36
              }
            ],
            "promptTokenCount": 3636,
            "promptTokensDetails": [
              {
                "modality": "DOCUMENT",
                "tokenCount": 3612
              },
              {
                "modality": "TEXT",
                "tokenCount": 24
              }
            ],
            "totalTokenCount": 3672
          }
        }
        "#;

        let response: Response = serde_json::from_str(response).unwrap();
        println!("response: {response:#?}");
    }
}
