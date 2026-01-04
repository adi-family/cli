//! ADI Embed Plugin
//!
//! Provides text embedding services using fastembed/ONNX for local ML inference.
//! Other plugins can use the adi.embed service for generating embeddings.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::sync::Mutex;

/// Service ID for embedding operations
const SERVICE_EMBED: &str = "adi.embed";

/// Global embedder instance
static EMBEDDER: OnceCell<Mutex<TextEmbedding>> = OnceCell::new();

/// Embedding model configuration
const MODEL_NAME: &str = "jinaai/jina-embeddings-v2-base-code";
const DIMENSIONS: u32 = 768;

// === Request/Response Types ===

#[derive(Deserialize)]
struct EmbedRequest {
    texts: Vec<String>,
}

#[derive(Serialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize)]
struct DimensionsResponse {
    dimensions: u32,
}

#[derive(Serialize)]
struct ModelInfoResponse {
    model_name: String,
    dimensions: u32,
    provider: String,
}

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new("adi.embed", "ADI Embed", env!("CARGO_PKG_VERSION"), "core")
        .with_author("ADI Team")
        .with_description("Text embedding service using fastembed/ONNX")
        .with_min_host_version("0.8.0")
}

fn get_cache_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "adi", "adi")
        .map(|dirs| dirs.cache_dir().join("models"))
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    // Initialize the embedder
    let cache_dir = get_cache_dir();

    let mut init_options = InitOptions::new(EmbeddingModel::JinaEmbeddingsV2BaseCode);

    if let Some(cache) = cache_dir {
        let _ = std::fs::create_dir_all(&cache);
        init_options = init_options.with_cache_dir(cache);
    }

    match TextEmbedding::try_new(init_options) {
        Ok(model) => {
            let _ = EMBEDDER.set(Mutex::new(model));
        }
        Err(e) => {
            unsafe {
                let host = (*ctx).host();
                host.error(&format!("Failed to initialize embedding model: {}", e));
            }
            return -1;
        }
    }

    unsafe {
        let host = (*ctx).host();

        // Register embedding service
        let embed_descriptor =
            ServiceDescriptor::new(SERVICE_EMBED, ServiceVersion::new(1, 0, 0), "adi.embed")
                .with_description("Text embedding service");

        let embed_handle = ServiceHandle::new(
            SERVICE_EMBED,
            ctx as *const c_void,
            &EMBED_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(embed_descriptor, embed_handle) {
            host.error(&format!("Failed to register embed service: {}", code));
            return code;
        }

        host.info("ADI Embed plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

extern "C" fn handle_message(
    _ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    _msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    match msg_type.as_str() {
        "ping" => RResult::ROk(RString::from("pong")),
        _ => RResult::RErr(PluginError::new(
            -1,
            format!("Unknown message type: {}", msg_type.as_str()),
        )),
    }
}

// === Plugin Entry Point ===

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RSome(handle_message),
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}

// === Embed Service Implementation ===

extern "C" fn embed_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        "embed" => handle_embed(args.as_str()),
        "dimensions" => handle_dimensions(),
        "model_info" => handle_model_info(),
        _ => RResult::RErr(ServiceError::new(
            -1,
            format!("Unknown method: {}", method.as_str()),
        )),
    }
}

fn handle_embed(args: &str) -> RResult<RString, ServiceError> {
    let request: EmbedRequest = match serde_json::from_str(args) {
        Ok(r) => r,
        Err(e) => {
            return RResult::RErr(ServiceError::new(-1, format!("Invalid request: {}", e)));
        }
    };

    if request.texts.is_empty() {
        let response = EmbedResponse { embeddings: vec![] };
        return RResult::ROk(RString::from(serde_json::to_string(&response).unwrap()));
    }

    let embedder = match EMBEDDER.get() {
        Some(e) => e,
        None => {
            return RResult::RErr(ServiceError::new(-1, "Embedder not initialized".to_string()));
        }
    };

    let model = match embedder.lock() {
        Ok(m) => m,
        Err(e) => {
            return RResult::RErr(ServiceError::new(-1, format!("Lock error: {}", e)));
        }
    };

    match model.embed(request.texts, None) {
        Ok(embeddings) => {
            let response = EmbedResponse { embeddings };
            RResult::ROk(RString::from(serde_json::to_string(&response).unwrap()))
        }
        Err(e) => RResult::RErr(ServiceError::new(-1, format!("Embedding error: {}", e))),
    }
}

fn handle_dimensions() -> RResult<RString, ServiceError> {
    let response = DimensionsResponse {
        dimensions: DIMENSIONS,
    };
    RResult::ROk(RString::from(serde_json::to_string(&response).unwrap()))
}

fn handle_model_info() -> RResult<RString, ServiceError> {
    let response = ModelInfoResponse {
        model_name: MODEL_NAME.to_string(),
        dimensions: DIMENSIONS,
        provider: "fastembed".to_string(),
    };
    RResult::ROk(RString::from(serde_json::to_string(&response).unwrap()))
}

extern "C" fn embed_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("embed")
            .with_description("Generate embeddings for texts")
            .with_parameters_schema(r#"{"texts": ["string"]}"#)
            .with_returns_schema(r#"{"embeddings": [[f32]]}"#),
        ServiceMethod::new("dimensions")
            .with_description("Get embedding dimensions")
            .with_returns_schema(r#"{"dimensions": u32}"#),
        ServiceMethod::new("model_info")
            .with_description("Get model information")
            .with_returns_schema(r#"{"model_name": "string", "dimensions": u32, "provider": "string"}"#),
    ]
    .into_iter()
    .collect()
}

static EMBED_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: embed_invoke,
    list_methods: embed_list_methods,
};
