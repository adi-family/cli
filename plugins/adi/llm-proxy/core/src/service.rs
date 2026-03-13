include!(concat!(env!("OUT_DIR"), "/llm_proxy_adi_service.rs"));

use crate::crypto::SecretManager;
use crate::db;
use crate::models::*;
use crate::providers;
use sqlx::PgPool;

pub struct LlmProxyService {
    db: PgPool,
    secrets: SecretManager,
}

impl LlmProxyService {
    pub fn new(db: PgPool, secrets: SecretManager) -> Self {
        Self { db, secrets }
    }

    pub async fn from_env() -> Result<Self, String> {
        let config =
            crate::Config::from_env().map_err(|e| format!("Config error: {e}"))?;

        let pool = PgPool::connect(&config.database_url)
            .await
            .map_err(|e| format!("Database connection failed: {e}"))?;

        let secrets =
            SecretManager::from_hex(&config.encryption_key).map_err(|e| format!("{e}"))?;

        Ok(Self::new(pool, secrets))
    }

    fn parse_user_id(ctx: &AdiCallerContext) -> Result<Uuid, AdiServiceError> {
        ctx.require_user_id()?
            .parse()
            .map_err(|_| AdiServiceError::internal("Invalid user_id format"))
    }

    fn map_error(e: crate::ApiError) -> AdiServiceError {
        e.into()
    }
}

#[async_trait::async_trait]
impl LlmProxyServiceHandler for LlmProxyService {
    // -- Upstream key management --

    async fn list_keys(
        &self,
        ctx: &AdiCallerContext,
    ) -> Result<Vec<UpstreamApiKeySummary>, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let keys = db::list_upstream_keys(&self.db, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(keys.into_iter().map(Into::into).collect())
    }

    async fn get_key(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<UpstreamApiKeySummary, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let key = db::get_upstream_key(&self.db, id, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(key.into())
    }

    async fn create_key(
        &self,
        ctx: &AdiCallerContext,
        name: String,
        provider_type: ProviderType,
        api_key: String,
        base_url: Option<String>,
    ) -> Result<UpstreamApiKeySummary, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let encrypted = self
            .secrets
            .encrypt(&api_key)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;
        let key = db::create_upstream_key(
            &self.db,
            user_id,
            &name,
            provider_type,
            &encrypted,
            base_url.as_deref(),
        )
        .await
        .map_err(Self::map_error)?;
        Ok(key.into())
    }

    async fn update_key(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
        name: Option<String>,
        api_key: Option<String>,
        base_url: Option<String>,
        is_active: Option<bool>,
    ) -> Result<UpstreamApiKeySummary, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let encrypted = api_key
            .as_deref()
            .map(|k| self.secrets.encrypt(k))
            .transpose()
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;
        let key = db::update_upstream_key(
            &self.db,
            id,
            user_id,
            name.as_deref(),
            encrypted.as_deref(),
            Some(base_url.as_deref()),
            is_active,
        )
        .await
        .map_err(Self::map_error)?;
        Ok(key.into())
    }

    async fn delete_key(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<DeletedResponse, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        db::delete_upstream_key(&self.db, id, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(DeletedResponse { deleted: true })
    }

    async fn verify_key(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<VerifyKeyResponse, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let key = db::get_upstream_key(&self.db, id, user_id)
            .await
            .map_err(Self::map_error)?;
        let decrypted = self
            .secrets
            .decrypt(&key.api_key_encrypted)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;
        let provider = providers::create_provider(key.provider_type, key.base_url.clone());
        match provider.list_models(&decrypted).await {
            Ok(models) => Ok(VerifyKeyResponse {
                valid: true,
                models: Some(models.into_iter().map(|m| m.id).collect()),
                error: None,
            }),
            Err(e) => Ok(VerifyKeyResponse {
                valid: false,
                models: None,
                error: Some(e.to_string()),
            }),
        }
    }

    // -- Platform key management --

    async fn list_platform_keys(
        &self,
        _ctx: &AdiCallerContext,
    ) -> Result<Vec<PlatformKeySummary>, AdiServiceError> {
        let keys = db::list_platform_keys(&self.db)
            .await
            .map_err(Self::map_error)?;
        Ok(keys
            .into_iter()
            .map(|k| PlatformKeySummary {
                id: k.id,
                provider_type: k.provider_type,
                base_url: k.base_url,
                is_active: k.is_active,
                created_at: k.created_at,
                updated_at: k.updated_at,
            })
            .collect())
    }

    async fn upsert_platform_key(
        &self,
        _ctx: &AdiCallerContext,
        provider_type: ProviderType,
        api_key: String,
        base_url: Option<String>,
    ) -> Result<PlatformKeySummary, AdiServiceError> {
        let encrypted = self
            .secrets
            .encrypt(&api_key)
            .map_err(|e| AdiServiceError::internal(e.to_string()))?;
        let k = db::upsert_platform_key(&self.db, provider_type, &encrypted, base_url.as_deref())
            .await
            .map_err(Self::map_error)?;
        Ok(PlatformKeySummary {
            id: k.id,
            provider_type: k.provider_type,
            base_url: k.base_url,
            is_active: k.is_active,
            created_at: k.created_at,
            updated_at: k.updated_at,
        })
    }

    async fn update_platform_key(
        &self,
        _ctx: &AdiCallerContext,
        id: Uuid,
        is_active: Option<bool>,
    ) -> Result<PlatformKeySummary, AdiServiceError> {
        let k = db::set_platform_key_active(&self.db, id, is_active.unwrap_or(true))
            .await
            .map_err(Self::map_error)?;
        Ok(PlatformKeySummary {
            id: k.id,
            provider_type: k.provider_type,
            base_url: k.base_url,
            is_active: k.is_active,
            created_at: k.created_at,
            updated_at: k.updated_at,
        })
    }

    async fn delete_platform_key(
        &self,
        _ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<DeletedResponse, AdiServiceError> {
        db::delete_platform_key(&self.db, id)
            .await
            .map_err(Self::map_error)?;
        Ok(DeletedResponse { deleted: true })
    }

    // -- Token management --

    async fn list_tokens(
        &self,
        ctx: &AdiCallerContext,
    ) -> Result<Vec<ProxyTokenSummary>, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let tokens = db::list_proxy_tokens(&self.db, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(tokens.into_iter().map(Into::into).collect())
    }

    async fn get_token(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<ProxyTokenSummary, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let token = db::get_proxy_token(&self.db, id, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(token.into())
    }

    async fn create_token(
        &self,
        ctx: &AdiCallerContext,
        name: String,
        key_mode: KeyMode,
        upstream_key_id: Option<Uuid>,
        platform_provider: Option<ProviderType>,
        request_script: Option<String>,
        response_script: Option<String>,
        allowed_models: Option<Vec<String>>,
        blocked_models: Option<Vec<String>>,
        log_requests: Option<bool>,
        log_responses: Option<bool>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<CreateTokenResponse, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;

        let (raw_token, prefix, hash) = db::tokens::generate_token();
        let token = db::create_proxy_token(
            &self.db,
            user_id,
            &name,
            &hash,
            &prefix,
            key_mode,
            upstream_key_id,
            platform_provider,
            request_script.as_deref(),
            response_script.as_deref(),
            allowed_models.as_deref(),
            blocked_models.as_deref(),
            log_requests.unwrap_or(false),
            log_responses.unwrap_or(false),
            expires_at,
        )
        .await
        .map_err(Self::map_error)?;

        Ok(CreateTokenResponse {
            token: token.into(),
            secret: raw_token,
        })
    }

    async fn update_token(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
        name: Option<String>,
        request_script: Option<String>,
        response_script: Option<String>,
        allowed_models: Option<Vec<String>>,
        blocked_models: Option<Vec<String>>,
        log_requests: Option<bool>,
        log_responses: Option<bool>,
        is_active: Option<bool>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ProxyTokenSummary, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;

        let token = db::update_proxy_token(
            &self.db,
            id,
            user_id,
            name.as_deref(),
            request_script.as_ref().map(|s| Some(s.as_str())),
            response_script.as_ref().map(|s| Some(s.as_str())),
            allowed_models.as_ref().map(|v| Some(v.as_slice())),
            blocked_models.as_ref().map(|v| Some(v.as_slice())),
            log_requests,
            log_responses,
            is_active,
            expires_at.map(Some),
        )
        .await
        .map_err(Self::map_error)?;
        Ok(token.into())
    }

    async fn delete_token(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<DeletedResponse, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        db::delete_proxy_token(&self.db, id, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(DeletedResponse { deleted: true })
    }

    async fn rotate_token(
        &self,
        ctx: &AdiCallerContext,
        id: Uuid,
    ) -> Result<RotateTokenResponse, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let (token, secret) = db::rotate_proxy_token(&self.db, id, user_id)
            .await
            .map_err(Self::map_error)?;
        Ok(RotateTokenResponse {
            token: token.into(),
            secret,
        })
    }

    // -- Providers --

    async fn list_providers(
        &self,
        _ctx: &AdiCallerContext,
    ) -> Result<Vec<ProviderSummary>, AdiServiceError> {
        let platform_keys = db::list_platform_keys(&self.db)
            .await
            .map_err(Self::map_error)?;
        let all_models = db::list_all_allowed_models(&self.db)
            .await
            .map_err(Self::map_error)?;

        let provider_types = [
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::OpenRouter,
            ProviderType::Custom,
        ];

        let summaries = provider_types
            .iter()
            .map(|pt| {
                let is_available = platform_keys
                    .iter()
                    .any(|k| k.provider_type == *pt && k.is_active);
                let models: Vec<AllowedModelInfo> = all_models
                    .iter()
                    .filter(|m| m.provider_type == *pt)
                    .map(|m| AllowedModelInfo {
                        model_id: m.model_id.clone(),
                        display_name: m.display_name.clone(),
                    })
                    .collect();
                ProviderSummary {
                    provider_type: *pt,
                    is_available,
                    allowed_models: models,
                }
            })
            .collect();

        Ok(summaries)
    }

    // -- Usage --

    async fn query_usage(
        &self,
        ctx: &AdiCallerContext,
        proxy_token_id: Option<Uuid>,
        from: Option<String>,
        to: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<UsageResponse, AdiServiceError> {
        let user_id = Self::parse_user_id(ctx)?;
        let from_dt = from
            .map(|s| {
                s.parse::<chrono::DateTime<chrono::Utc>>()
                    .map_err(|_| AdiServiceError::invalid_params("Invalid 'from' datetime"))
            })
            .transpose()?;
        let to_dt = to
            .map(|s| {
                s.parse::<chrono::DateTime<chrono::Utc>>()
                    .map_err(|_| AdiServiceError::invalid_params("Invalid 'to' datetime"))
            })
            .transpose()?;

        let lim = limit.unwrap_or(50);
        let off = offset.unwrap_or(0);

        let logs = db::query_usage(&self.db, user_id, proxy_token_id, from_dt, to_dt, lim, off)
            .await
            .map_err(Self::map_error)?;

        let summary = db::get_usage_summary(&self.db, user_id, proxy_token_id, from_dt, to_dt)
            .await
            .map_err(Self::map_error)?;

        Ok(UsageResponse {
            logs: logs
                .into_iter()
                .map(|l| UsageLogEntry {
                    id: l.id,
                    proxy_token_id: l.proxy_token_id,
                    user_id: l.user_id,
                    request_id: l.request_id,
                    upstream_request_id: l.upstream_request_id,
                    requested_model: l.requested_model,
                    actual_model: l.actual_model,
                    provider_type: l.provider_type,
                    key_mode: l.key_mode,
                    input_tokens: l.input_tokens,
                    output_tokens: l.output_tokens,
                    total_tokens: l.total_tokens,
                    reported_cost_usd: l.reported_cost_usd.map(|d| d.to_string()),
                    endpoint: l.endpoint,
                    is_streaming: l.is_streaming,
                    latency_ms: l.latency_ms,
                    ttft_ms: l.ttft_ms,
                    status: l.status,
                    status_code: l.status_code,
                    error_type: l.error_type,
                    error_message: l.error_message,
                    created_at: l.created_at,
                })
                .collect(),
            total: summary.total_requests,
        })
    }
}
