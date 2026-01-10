use serde::{Deserialize, Serialize};

/// Role in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Model,
}

/// Content part (text, inline data, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<InlineData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            inline_data: None,
            function_call: None,
            function_response: None,
        }
    }

    pub fn inline_data(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            text: None,
            inline_data: Some(InlineData {
                mime_type: mime_type.into(),
                data: data.into(),
            }),
            function_call: None,
            function_response: None,
        }
    }

    pub fn function_response(name: impl Into<String>, response: serde_json::Value) -> Self {
        Self {
            text: None,
            inline_data: None,
            function_call: None,
            function_response: Some(FunctionResponse {
                name: name.into(),
                response,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: Role,
    pub parts: Vec<Part>,
}

impl Content {
    pub fn user(parts: Vec<Part>) -> Self {
        Self {
            role: Role::User,
            parts,
        }
    }

    pub fn model(parts: Vec<Part>) -> Self {
        Self {
            role: Role::Model,
            parts,
        }
    }

    pub fn user_text(text: impl Into<String>) -> Self {
        Self::user(vec![Part::text(text)])
    }

    pub fn model_text(text: impl Into<String>) -> Self {
        Self::model(vec![Part::text(text)])
    }
}

/// Tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl FunctionDeclaration {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// Generation configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,
}

impl GenerationConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_output_tokens(mut self, tokens: i32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }

    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    pub fn top_k(mut self, k: i32) -> Self {
        self.top_k = Some(k);
        self
    }
}

/// Safety setting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetySetting {
    pub category: HarmCategory,
    pub threshold: HarmBlockThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmCategory {
    HarmCategoryHarassment,
    HarmCategoryHateSpeech,
    HarmCategorySexuallyExplicit,
    HarmCategoryDangerousContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    BlockNone,
    BlockOnlyHigh,
    BlockMediumAndAbove,
    BlockLowAndAbove,
}

/// Generate content request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
}

impl GenerateContentRequest {
    pub fn new(contents: Vec<Content>) -> Self {
        Self {
            contents,
            system_instruction: None,
            tools: None,
            generation_config: None,
            safety_settings: None,
        }
    }

    pub fn with_system(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(Content::user(vec![Part::text(instruction)]));
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }

    pub fn with_safety(mut self, settings: Vec<SafetySetting>) -> Self {
        self.safety_settings = Some(settings);
        self
    }
}

/// Generate content response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    pub candidates: Option<Vec<Candidate>>,
    pub prompt_feedback: Option<PromptFeedback>,
    pub usage_metadata: Option<UsageMetadata>,
}

impl GenerateContentResponse {
    /// Get text from first candidate.
    pub fn text(&self) -> Option<String> {
        self.candidates
            .as_ref()?
            .first()?
            .content
            .parts
            .iter()
            .filter_map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("")
            .into()
    }

    /// Get function calls from first candidate.
    pub fn function_calls(&self) -> Vec<&FunctionCall> {
        self.candidates
            .as_ref()
            .map(|c| {
                c.first()
                    .map(|c| {
                        c.content
                            .parts
                            .iter()
                            .filter_map(|p| p.function_call.as_ref())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Check if response has function calls.
    pub fn has_function_calls(&self) -> bool {
        !self.function_calls().is_empty()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<FinishReason>,
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Other,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    pub block_reason: Option<String>,
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: Option<i32>,
    pub total_token_count: i32,
}

/// Count tokens request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountTokensRequest {
    pub contents: Vec<Content>,
}

/// Count tokens response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountTokensResponse {
    pub total_tokens: i32,
}

/// Model info.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub input_token_limit: i32,
    pub output_token_limit: i32,
    pub supported_generation_methods: Vec<String>,
}

/// List models response.
#[derive(Debug, Clone, Deserialize)]
pub struct ListModelsResponse {
    pub models: Vec<Model>,
}
