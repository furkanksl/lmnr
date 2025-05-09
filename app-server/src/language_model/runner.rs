use anyhow::{Context, Result};
use enum_dispatch::enum_dispatch;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::{cache::Cache, db::DB, pipeline::nodes::StreamChunk, traces::spans::InputTokens};

use super::{
    chat_message::ChatCompletion,
    providers::{
        anthropic_bedrock::{AWS_ACCESS_KEY_ID, AWS_REGION, AWS_SECRET_ACCESS_KEY},
        openai_azure::{OPENAI_AZURE_DEPLOYMENT_NAME, OPENAI_AZURE_RESOURCE_ID},
        utils::get_provider,
    },
    Anthropic, AnthropicBedrock, ChatMessage, Gemini, Groq, Mistral, OpenAI, OpenAIAzure,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LanguageModelProviderName {
    Anthropic,
    Mistral,
    OpenAI,
    OpenAIAzure,
    Gemini,
    Groq,
    Bedrock,
}

#[derive(Clone, Debug)]
#[enum_dispatch]
pub enum LanguageModelProvider {
    Anthropic(Anthropic),
    Gemini(Gemini),
    Groq(Groq),
    Mistral(Mistral),
    OpenAI(OpenAI),
    OpenAIAzure(OpenAIAzure),
    Bedrock(AnthropicBedrock),
}

#[enum_dispatch(LanguageModelProvider)]
pub trait ExecuteChatCompletion {
    async fn chat_completion(
        &self,
        model: &str,
        provider_name: LanguageModelProviderName,
        messages: &Vec<ChatMessage>,
        params: &Value,
        env: &HashMap<String, String>,
        tx: Option<Sender<StreamChunk>>,
        node_info: &NodeInfo,
        db: Arc<DB>,
        cache: Arc<Cache>,
    ) -> Result<ChatCompletion>;
}

#[enum_dispatch(LanguageModelProvider)]
pub trait EstimateCost {
    fn db_provider_name(&self) -> &str;

    async fn estimate_input_cost(
        &self,
        db: Arc<DB>,
        cache: Arc<Cache>,
        model: &str,
        input_tokens: u32,
    ) -> Option<f64> {
        super::costs::estimate_input_cost(
            db.clone(),
            cache.clone(),
            self.db_provider_name(),
            model,
            InputTokens {
                regular_input_tokens: input_tokens as i64,
                cache_write_tokens: 0,
                cache_read_tokens: 0,
            },
        )
        .await
    }

    async fn estimate_output_cost(
        &self,
        db: Arc<DB>,
        cache: Arc<Cache>,
        model: &str,
        output_tokens: u32,
    ) -> Option<f64> {
        super::costs::estimate_output_cost(
            db.clone(),
            cache.clone(),
            self.db_provider_name(),
            model,
            output_tokens as i64,
        )
        .await
    }

    async fn estimate_cost(
        &self,
        db: Arc<DB>,
        cache: Arc<Cache>,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Option<f64> {
        let input_cost = self
            .estimate_input_cost(db.clone(), cache.clone(), model, input_tokens)
            .await
            .or_else(|| {
                log::warn!(
                    "No stored price found for provider: {}, model: {}",
                    self.db_provider_name(),
                    model,
                );
                None
            })?;
        let output_cost = self
            .estimate_output_cost(db.clone(), cache.clone(), model, output_tokens)
            .await?;

        Some(input_cost + output_cost)
    }
}

impl LanguageModelProviderName {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "anthropic" => Ok(Self::Anthropic),
            "mistral" => Ok(Self::Mistral),
            "openai" => Ok(Self::OpenAI),
            "openai-azure" => Ok(Self::OpenAIAzure),
            "gemini" => Ok(Self::Gemini),
            "groq" => Ok(Self::Groq),
            "bedrock" => Ok(Self::Bedrock),
            _ => Err(anyhow::anyhow!("Invalid language model provider: {}", s)),
        }
    }

    pub fn api_key(&self, env: &HashMap<String, String>) -> Result<String> {
        let name = self.api_key_name();
        env.get(name)
            .cloned()
            .ok_or(anyhow::anyhow!("Env variables don't contain: {}", name))
    }

    fn api_key_name(&self) -> &str {
        match self {
            LanguageModelProviderName::Anthropic => "ANTHROPIC_API_KEY",
            LanguageModelProviderName::Mistral => "MISTRAL_API_KEY",
            LanguageModelProviderName::OpenAI => "OPENAI_API_KEY",
            LanguageModelProviderName::OpenAIAzure => "AZURE_API_KEY",
            LanguageModelProviderName::Gemini => "GEMINI_API_KEY",
            LanguageModelProviderName::Groq => "GROQ_API_KEY",
            LanguageModelProviderName::Bedrock => AWS_SECRET_ACCESS_KEY,
        }
    }

    pub fn required_env_vars(&self) -> HashSet<String> {
        let mut env_vars = HashSet::new();
        env_vars.insert(self.api_key_name().to_string());

        if matches!(self, Self::Bedrock) {
            env_vars.insert(AWS_REGION.to_string());
            env_vars.insert(AWS_ACCESS_KEY_ID.to_string());
        } else if matches!(self, Self::OpenAIAzure) {
            env_vars.insert(OPENAI_AZURE_RESOURCE_ID.to_string());
            env_vars.insert(OPENAI_AZURE_DEPLOYMENT_NAME.to_string());
        }

        env_vars
    }
}

#[derive(Debug)]
pub struct LanguageModelRunner {
    pub models: HashMap<LanguageModelProviderName, LanguageModelProvider>,
}

impl LanguageModelRunner {
    pub fn new(models: HashMap<LanguageModelProviderName, LanguageModelProvider>) -> Self {
        Self { models }
    }

    /// Completes the chat by calling model's executor
    ///
    /// # Arguments
    ///
    /// * model - model name in the format of "provider:model_name".
    ///     e.g. "openai:gp-3.5-turbo-16k"
    ///     This is done to future proof the system for dynamic model names when we will serve custom fine-tuned models
    ///
    /// * messages - list of messages in the chat.
    ///     If system message is passed, then it must be put as first message!
    ///     Next, alternating user and assistant messages are passed starting from user message.
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: &Vec<ChatMessage>,
        params: &Value,
        env: &HashMap<String, String>,
        tx: Option<Sender<StreamChunk>>,
        node_info: &NodeInfo,
        db: Arc<DB>,
        cache: Arc<Cache>,
    ) -> Result<ChatCompletion> {
        let provider = get_provider(model).context("Invalid model format")?;
        let model_name = model.split(":").skip(1).join(":");
        if model_name.is_empty() {
            return Err(anyhow::anyhow!("Invalid model format"));
        }
        let provider_name = LanguageModelProviderName::from_str(provider)?;

        let executor = self.models.get(&provider_name).unwrap();
        executor
            .chat_completion(
                model_name.as_str(),
                provider_name,
                messages,
                params,
                env,
                tx,
                node_info,
                db,
                cache,
            )
            .await
    }
}

/// Information on the node to send along the streaming
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: Uuid,
    pub node_id: Uuid,
    pub node_name: String,
    pub node_type: String,
}
