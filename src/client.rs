use serde::{Deserialize, Serialize};

use crate::result::ProofreadResult;

#[derive(Debug, Clone)]
pub struct OpenAiCompatClient {
    http: reqwest::blocking::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiCompatClient {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            api_key: api_key.into(),
        }
    }

    pub fn request_proofread(&self, prompt: &str) -> Result<ProofreadResult, ClientError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatCompletionsRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: Some(0.2),
            response_format: Some(ResponseFormat {
                response_type: "json_object".to_string(),
            }),
        };

        let response = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()?
            .error_for_status()?;
        let parsed: ChatCompletionsResponse = response.json()?;
        let content = parsed
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or(ClientError::EmptyChoices)?;
        let result = ProofreadResult::from_json(&content).map_err(ClientError::ParseResult)?;
        Ok(result)
    }
}

#[derive(Debug)]
pub enum ClientError {
    Http(reqwest::Error),
    EmptyChoices,
    ParseResult(serde_json::Error),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(err) => write!(f, "http error: {err}"),
            Self::EmptyChoices => write!(f, "response choices is empty"),
            Self::ParseResult(err) => write!(f, "failed to parse proofreading result: {err}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<reqwest::Error> for ClientError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

#[derive(Debug, Clone, Serialize)]
struct ChatCompletionsRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    response_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatCompletionsResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
struct Choice {
    message: Message,
}
