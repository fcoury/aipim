use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::client::AIProvider;

const MAX_TOKENS: u32 = 1024;
const ANTRHOPIC_VERSION: &str = "2023-06-01";
const MODELS: &[&str] = &[
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
];

pub struct ClaudeProvider {}

#[async_trait]
impl AIProvider for ClaudeProvider {
    async fn send_message(&self, _message: String) -> anyhow::Result<String> {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: Content,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    ContentBlocks(Vec<ContentBlock>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ContentType {
    Text,
    Image,
}

#[derive(Serialize, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: ContentType,
    text: Option<String>,
    source: Option<Source>,
}

#[derive(Serialize, Deserialize)]
struct Source {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Serialize, Deserialize)]
struct RequestBody {
    model: String,
    max_tokens: i32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    user_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ToolChoice {
    #[serde(rename = "type")]
    tool_choice_type: String,
}

#[derive(Serialize, Deserialize)]
struct Tool {
    name: String,
    description: Option<String>,
    input_schema: ToolInputSchema,
}

#[derive(Serialize, Deserialize)]
struct ToolInputSchema {
    #[serde(rename = "type")]
    schema_type: String,
    properties: Option<Properties>,
}

#[derive(Serialize, Deserialize)]
struct Properties {
    ticker: Option<Ticker>,
}

#[derive(Serialize, Deserialize)]
struct Ticker {
    #[serde(rename = "type")]
    ticker_type: String,
    description: String,
}
