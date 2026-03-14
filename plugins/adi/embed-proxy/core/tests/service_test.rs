//! Integration tests for EmbedProxyService handler methods.
//!
//! Requires a PostgreSQL instance. Set `DATABASE_URL` env var before running.
//! Each test gets an isolated database via `#[sqlx::test]`.

use embed_proxy_core::crypto::SecretManager;
use embed_proxy_core::models::*;
use embed_proxy_core::service::{EmbedProxyService, EmbedProxyServiceHandler};
use lib_adi_service::AdiCallerContext;
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_secrets() -> SecretManager {
    SecretManager::new([0u8; 32])
}

fn authed_ctx() -> AdiCallerContext {
    AdiCallerContext {
        user_id: Some("550e8400-e29b-41d4-a716-446655440000".into()),
        device_id: Some("test-device".into()),
    }
}

fn other_user_ctx() -> AdiCallerContext {
    AdiCallerContext {
        user_id: Some("660e8400-e29b-41d4-a716-446655440001".into()),
        device_id: None,
    }
}

fn anon_ctx() -> AdiCallerContext {
    AdiCallerContext::anonymous()
}

fn bad_uid_ctx() -> AdiCallerContext {
    AdiCallerContext {
        user_id: Some("not-a-uuid".into()),
        device_id: None,
    }
}

fn svc(pool: PgPool) -> EmbedProxyService {
    EmbedProxyService::new(pool, test_secrets())
}

// ── list_keys ──

#[sqlx::test(migrations = "../migrations")]
async fn list_keys_success_empty(pool: PgPool) {
    let s = svc(pool);
    let keys = s.list_keys(&authed_ctx()).await.unwrap();
    assert!(keys.is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn list_keys_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s.list_keys(&anon_ctx()).await.unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn list_keys_fail_invalid_user_id(pool: PgPool) {
    let s = svc(pool);
    let err = s.list_keys(&bad_uid_ctx()).await.unwrap_err();
    assert_eq!(err.code, "internal");
}

// ── create_key ──

#[sqlx::test(migrations = "../migrations")]
async fn create_key_success(pool: PgPool) {
    let s = svc(pool);
    let key = s
        .create_key(
            &authed_ctx(),
            "my-openai".into(),
            ProviderType::OpenAI,
            "sk-test-123".into(),
            None,
        )
        .await
        .unwrap();

    assert_eq!(key.name, "my-openai");
    assert_eq!(key.provider_type, ProviderType::OpenAI);
    assert!(key.is_active);
}

#[sqlx::test(migrations = "../migrations")]
async fn create_key_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .create_key(
            &anon_ctx(),
            "key".into(),
            ProviderType::OpenAI,
            "sk-x".into(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn create_key_fail_duplicate_name(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    s.create_key(
        &ctx,
        "dup-key".into(),
        ProviderType::OpenAI,
        "sk-1".into(),
        None,
    )
    .await
    .unwrap();

    let err = s
        .create_key(
            &ctx,
            "dup-key".into(),
            ProviderType::Cohere,
            "sk-2".into(),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.code == "internal" || err.code == "invalid_params",
        "expected internal or invalid_params, got: {}",
        err.code
    );
}

// ── get_key ──

#[sqlx::test(migrations = "../migrations")]
async fn get_key_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let created = s
        .create_key(
            &ctx,
            "get-test".into(),
            ProviderType::Cohere,
            "sk-abc".into(),
            Some("https://custom.api".into()),
        )
        .await
        .unwrap();

    let fetched = s.get_key(&ctx, created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "get-test");
    assert_eq!(fetched.base_url, Some("https://custom.api".into()));
}

#[sqlx::test(migrations = "../migrations")]
async fn get_key_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s.get_key(&anon_ctx(), Uuid::new_v4()).await.unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn get_key_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .get_key(&authed_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── update_key ──

#[sqlx::test(migrations = "../migrations")]
async fn update_key_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let key = s
        .create_key(&ctx, "upd".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();

    let updated = s
        .update_key(
            &ctx,
            key.id,
            Some("renamed".into()),
            None,
            None,
            Some(false),
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "renamed");
    assert!(!updated.is_active);
}

#[sqlx::test(migrations = "../migrations")]
async fn update_key_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .update_key(&anon_ctx(), Uuid::new_v4(), None, None, None, None)
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn update_key_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .update_key(
            &authed_ctx(),
            Uuid::new_v4(),
            Some("x".into()),
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── delete_key ──

#[sqlx::test(migrations = "../migrations")]
async fn delete_key_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let key = s
        .create_key(&ctx, "del".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();

    let resp = s.delete_key(&ctx, key.id).await.unwrap();
    assert!(resp.deleted);

    let err = s.get_key(&ctx, key.id).await.unwrap_err();
    assert_eq!(err.code, "not_found");
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_key_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s.delete_key(&anon_ctx(), Uuid::new_v4()).await.unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_key_fail_wrong_user(pool: PgPool) {
    let s = svc(pool);
    let key = s
        .create_key(
            &authed_ctx(),
            "other".into(),
            ProviderType::OpenAI,
            "sk-x".into(),
            None,
        )
        .await
        .unwrap();

    let err = s.delete_key(&other_user_ctx(), key.id).await.unwrap_err();
    assert!(err.code == "not_found" || err.code == "internal");
}

// ── verify_key ──

#[sqlx::test(migrations = "../migrations")]
async fn verify_key_success_valid(pool: PgPool) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "text-embedding-3-small", "object": "model"},
                    {"id": "text-embedding-3-large", "object": "model"}
                ]
            })),
        )
        .mount(&mock_server)
        .await;

    let s = svc(pool);
    let ctx = authed_ctx();
    let key = s
        .create_key(
            &ctx,
            "verify-ok".into(),
            ProviderType::OpenAI,
            "sk-valid".into(),
            Some(mock_server.uri()),
        )
        .await
        .unwrap();

    let resp = s.verify_key(&ctx, key.id).await.unwrap();
    assert!(resp.valid);
    assert!(resp.models.is_some());
    assert!(resp.error.is_none());
}

#[sqlx::test(migrations = "../migrations")]
async fn verify_key_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .verify_key(&anon_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn verify_key_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .verify_key(&authed_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── list_platform_keys ──

#[sqlx::test(migrations = "../migrations")]
async fn list_platform_keys_success_empty(pool: PgPool) {
    let s = svc(pool);
    let keys = s.list_platform_keys(&authed_ctx()).await.unwrap();
    assert!(keys.is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn list_platform_keys_success_with_data(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    s.upsert_platform_key(&ctx, ProviderType::OpenAI, "sk-plat".into(), None)
        .await
        .unwrap();

    let keys = s.list_platform_keys(&ctx).await.unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].provider_type, ProviderType::OpenAI);
}

// ── upsert_platform_key ──

#[sqlx::test(migrations = "../migrations")]
async fn upsert_platform_key_success_create(pool: PgPool) {
    let s = svc(pool);
    let key = s
        .upsert_platform_key(
            &authed_ctx(),
            ProviderType::Cohere,
            "sk-cohere".into(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(key.provider_type, ProviderType::Cohere);
    assert!(key.is_active);
}

#[sqlx::test(migrations = "../migrations")]
async fn upsert_platform_key_success_update(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let k1 = s
        .upsert_platform_key(&ctx, ProviderType::OpenAI, "sk-old".into(), None)
        .await
        .unwrap();
    let k2 = s
        .upsert_platform_key(
            &ctx,
            ProviderType::OpenAI,
            "sk-new".into(),
            Some("https://new.api".into()),
        )
        .await
        .unwrap();

    assert_eq!(k1.id, k2.id);
    assert_eq!(k2.base_url, Some("https://new.api".into()));
}

// ── update_platform_key ──

#[sqlx::test(migrations = "../migrations")]
async fn update_platform_key_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let key = s
        .upsert_platform_key(&ctx, ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();

    let updated = s
        .update_platform_key(&ctx, key.id, Some(false))
        .await
        .unwrap();
    assert!(!updated.is_active);
}

#[sqlx::test(migrations = "../migrations")]
async fn update_platform_key_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .update_platform_key(&authed_ctx(), Uuid::new_v4(), Some(false))
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

#[sqlx::test(migrations = "../migrations")]
async fn update_platform_key_fail_nil_id(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .update_platform_key(&authed_ctx(), Uuid::nil(), Some(true))
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── delete_platform_key ──

#[sqlx::test(migrations = "../migrations")]
async fn delete_platform_key_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let key = s
        .upsert_platform_key(&ctx, ProviderType::Google, "sk-goog".into(), None)
        .await
        .unwrap();

    let resp = s.delete_platform_key(&ctx, key.id).await.unwrap();
    assert!(resp.deleted);

    let keys = s.list_platform_keys(&ctx).await.unwrap();
    assert!(keys.is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_platform_key_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .delete_platform_key(&authed_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_platform_key_fail_nil_id(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .delete_platform_key(&authed_ctx(), Uuid::nil())
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── list_tokens ──

#[sqlx::test(migrations = "../migrations")]
async fn list_tokens_success_empty(pool: PgPool) {
    let s = svc(pool);
    let tokens = s.list_tokens(&authed_ctx()).await.unwrap();
    assert!(tokens.is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn list_tokens_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s.list_tokens(&anon_ctx()).await.unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn list_tokens_fail_invalid_user_id(pool: PgPool) {
    let s = svc(pool);
    let err = s.list_tokens(&bad_uid_ctx()).await.unwrap_err();
    assert_eq!(err.code, "internal");
}

// ── create_token ──

#[sqlx::test(migrations = "../migrations")]
async fn create_token_success_byok(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let key = s
        .create_key(&ctx, "tok-key".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();

    let resp = s
        .create_token(
            &ctx,
            "my-token".into(),
            KeyMode::Byok,
            Some(key.id),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(resp.token.name, "my-token");
    assert_eq!(resp.token.key_mode, KeyMode::Byok);
    assert!(!resp.secret.is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn create_token_success_platform(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let resp = s
        .create_token(
            &ctx,
            "plat-token".into(),
            KeyMode::Platform,
            None,
            Some(ProviderType::OpenAI),
            Some(vec!["text-embedding-3-small".into()]),
            None,
            Some(true),
            Some(false),
            None,
        )
        .await
        .unwrap();

    assert_eq!(resp.token.key_mode, KeyMode::Platform);
    assert_eq!(resp.token.platform_provider, Some(ProviderType::OpenAI));
    assert_eq!(
        resp.token.allowed_models,
        Some(vec!["text-embedding-3-small".into()])
    );
    assert!(resp.token.log_requests);
    assert!(!resp.token.log_responses);
}

#[sqlx::test(migrations = "../migrations")]
async fn create_token_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .create_token(
            &anon_ctx(),
            "x".into(),
            KeyMode::Platform,
            None,
            Some(ProviderType::OpenAI),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn create_token_fail_byok_without_key(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .create_token(
            &authed_ctx(),
            "bad".into(),
            KeyMode::Byok,
            None, // missing upstream_key_id
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        err.code == "internal" || err.code == "invalid_params",
        "expected constraint violation, got: {}",
        err.code
    );
}

// ── get_token ──

#[sqlx::test(migrations = "../migrations")]
async fn get_token_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let key = s
        .create_key(&ctx, "gt-key".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();
    let created = s
        .create_token(
            &ctx,
            "gt-tok".into(),
            KeyMode::Byok,
            Some(key.id),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let fetched = s.get_token(&ctx, created.token.id).await.unwrap();
    assert_eq!(fetched.id, created.token.id);
    assert_eq!(fetched.name, "gt-tok");
}

#[sqlx::test(migrations = "../migrations")]
async fn get_token_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s.get_token(&anon_ctx(), Uuid::new_v4()).await.unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn get_token_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .get_token(&authed_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── update_token ──

#[sqlx::test(migrations = "../migrations")]
async fn update_token_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let key = s
        .create_key(&ctx, "ut-key".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();
    let tok = s
        .create_token(
            &ctx,
            "ut-tok".into(),
            KeyMode::Byok,
            Some(key.id),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let updated = s
        .update_token(
            &ctx,
            tok.token.id,
            Some("renamed-tok".into()),
            Some(vec!["text-embedding-3-small".into()]),
            None,
            Some(true),
            None,
            Some(false),
            None,
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "renamed-tok");
    assert_eq!(
        updated.allowed_models,
        Some(vec!["text-embedding-3-small".into()])
    );
    assert!(updated.log_requests);
    assert!(!updated.is_active);
}

#[sqlx::test(migrations = "../migrations")]
async fn update_token_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .update_token(
            &anon_ctx(),
            Uuid::new_v4(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn update_token_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .update_token(
            &authed_ctx(),
            Uuid::new_v4(),
            Some("x".into()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── delete_token ──

#[sqlx::test(migrations = "../migrations")]
async fn delete_token_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let key = s
        .create_key(&ctx, "dt-key".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();
    let tok = s
        .create_token(
            &ctx,
            "dt-tok".into(),
            KeyMode::Byok,
            Some(key.id),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let resp = s.delete_token(&ctx, tok.token.id).await.unwrap();
    assert!(resp.deleted);

    let err = s.get_token(&ctx, tok.token.id).await.unwrap_err();
    assert_eq!(err.code, "not_found");
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_token_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .delete_token(&anon_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_token_fail_wrong_user(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let key = s
        .create_key(&ctx, "dtwu-key".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();
    let tok = s
        .create_token(
            &ctx,
            "dtwu-tok".into(),
            KeyMode::Byok,
            Some(key.id),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let err = s
        .delete_token(&other_user_ctx(), tok.token.id)
        .await
        .unwrap_err();
    assert!(err.code == "not_found" || err.code == "internal");
}

// ── rotate_token ──

#[sqlx::test(migrations = "../migrations")]
async fn rotate_token_success(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();

    let key = s
        .create_key(&ctx, "rt-key".into(), ProviderType::OpenAI, "sk-x".into(), None)
        .await
        .unwrap();
    let created = s
        .create_token(
            &ctx,
            "rt-tok".into(),
            KeyMode::Byok,
            Some(key.id),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let rotated = s.rotate_token(&ctx, created.token.id).await.unwrap();
    assert_eq!(rotated.token.id, created.token.id);
    assert_ne!(rotated.secret, created.secret);
}

#[sqlx::test(migrations = "../migrations")]
async fn rotate_token_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .rotate_token(&anon_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn rotate_token_fail_not_found(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .rotate_token(&authed_ctx(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.code, "not_found");
}

// ── list_providers ──

#[sqlx::test(migrations = "../migrations")]
async fn list_providers_success_empty(pool: PgPool) {
    let s = svc(pool);
    let providers = s.list_providers(&authed_ctx()).await.unwrap();
    assert_eq!(providers.len(), 4);
    assert!(providers.iter().all(|p| !p.is_available));
}

#[sqlx::test(migrations = "../migrations")]
async fn list_providers_success_with_active_key(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    s.upsert_platform_key(&ctx, ProviderType::OpenAI, "sk-plat".into(), None)
        .await
        .unwrap();

    let providers = s.list_providers(&ctx).await.unwrap();
    let openai = providers
        .iter()
        .find(|p| p.provider_type == ProviderType::OpenAI)
        .unwrap();
    assert!(openai.is_available);

    let cohere = providers
        .iter()
        .find(|p| p.provider_type == ProviderType::Cohere)
        .unwrap();
    assert!(!cohere.is_available);
}

#[sqlx::test(migrations = "../migrations")]
async fn list_providers_success_inactive_key_not_available(pool: PgPool) {
    let s = svc(pool);
    let ctx = authed_ctx();
    let key = s
        .upsert_platform_key(&ctx, ProviderType::OpenAI, "sk-plat".into(), None)
        .await
        .unwrap();
    s.update_platform_key(&ctx, key.id, Some(false))
        .await
        .unwrap();

    let providers = s.list_providers(&ctx).await.unwrap();
    let openai = providers
        .iter()
        .find(|p| p.provider_type == ProviderType::OpenAI)
        .unwrap();
    assert!(!openai.is_available);
}

// ── query_usage ──

#[sqlx::test(migrations = "../migrations")]
async fn query_usage_success_empty(pool: PgPool) {
    let s = svc(pool);
    let resp = s
        .query_usage(&authed_ctx(), None, None, None, None, None)
        .await
        .unwrap();
    assert!(resp.logs.is_empty());
    assert_eq!(resp.total, 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn query_usage_fail_anonymous(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .query_usage(&anon_ctx(), None, None, None, None, None)
        .await
        .unwrap_err();
    assert_eq!(err.code, "unauthorized");
}

#[sqlx::test(migrations = "../migrations")]
async fn query_usage_fail_invalid_from_datetime(pool: PgPool) {
    let s = svc(pool);
    let err = s
        .query_usage(
            &authed_ctx(),
            None,
            Some("not-a-date".into()),
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, "invalid_params");
}
