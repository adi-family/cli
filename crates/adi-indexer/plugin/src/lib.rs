//! ADI Indexer Plugin
//!
//! Provides MCP tools and resources for code indexing and semantic search.
//!
//! This plugin requires the adi.embed plugin to be installed for embeddings.
//! Install with: `adi plugin install adi.embed`

mod mcp;

use abi_stable::std_types::{ROption, RResult, RStr, RString};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceHandle,
    ServiceVTable, ServiceVersion, SERVICE_MCP_RESOURCES, SERVICE_MCP_TOOLS,
};
use lib_plugin_host::current_service_registry;
use once_cell::sync::OnceCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Global tokio runtime for async operations.
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

/// Plugin state stored in context user_data.
struct PluginState {
    project_path: PathBuf,
    indexer: Option<Arc<adi_indexer_core::Adi>>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            project_path: PathBuf::from("."),
            indexer: None,
        }
    }
}

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.indexer",
        "ADI Indexer",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Code indexer with semantic search and symbol analysis")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    // Initialize tokio runtime
    let _ = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    });

    // Create plugin state
    let state = Box::new(PluginState::default());

    // Store state in context
    unsafe {
        (*ctx).set_user_data(state);

        // Get host vtable
        let host = (*ctx).host();

        // Register MCP tools service
        let tools_descriptor = ServiceDescriptor::new(
            SERVICE_MCP_TOOLS,
            ServiceVersion::new(1, 0, 0),
            "adi.indexer",
        )
        .with_description("MCP tools for code search and analysis");

        let tools_handle = ServiceHandle::new(
            SERVICE_MCP_TOOLS,
            ctx as *const c_void,
            &MCP_TOOLS_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(tools_descriptor, tools_handle) {
            host.error(&format!("Failed to register MCP tools service: {}", code));
            return code;
        }

        // Register MCP resources service
        let resources_descriptor = ServiceDescriptor::new(
            SERVICE_MCP_RESOURCES,
            ServiceVersion::new(1, 0, 0),
            "adi.indexer",
        )
        .with_description("MCP resources for accessing indexed data");

        let resources_handle = ServiceHandle::new(
            SERVICE_MCP_RESOURCES,
            ctx as *const c_void,
            &MCP_RESOURCES_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(resources_descriptor, resources_handle) {
            host.error(&format!(
                "Failed to register MCP resources service: {}",
                code
            ));
            return code;
        }

        host.info("ADI Indexer plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(ctx: *mut PluginContext) {
    unsafe {
        // Take and drop the state
        let _state: Option<Box<PluginState>> = (*ctx).take_user_data();
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

// === Message Handler ===

extern "C" fn handle_message(
    ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    match msg_type.as_str() {
        "set_project_path" => {
            unsafe {
                if let Some(state) = (*ctx).user_data_mut::<PluginState>() {
                    state.project_path = PathBuf::from(msg_data.as_str());

                    // Get the service registry from thread-local
                    let service_registry = match current_service_registry() {
                        Some(sr) => sr,
                        None => {
                            return RResult::RErr(PluginError::new(
                                3,
                                "Service registry not available",
                            ));
                        }
                    };

                    // Initialize the indexer for the new path
                    let runtime = RUNTIME.get().unwrap();
                    let path = state.project_path.clone();
                    match runtime.block_on(async {
                        adi_indexer_core::Adi::open_with_plugins(&path, service_registry).await
                    }) {
                        Ok(adi) => {
                            state.indexer = Some(Arc::new(adi));
                            RResult::ROk(RString::from("ok"))
                        }
                        Err(e) => RResult::RErr(PluginError::new(
                            1,
                            format!("Failed to open indexer: {}", e),
                        )),
                    }
                } else {
                    RResult::RErr(PluginError::new(2, "Plugin not initialized"))
                }
            }
        }
        _ => RResult::RErr(PluginError::new(
            -1,
            format!("Unknown message type: {}", msg_type.as_str()),
        )),
    }
}

// === MCP Tools Service ===

static MCP_TOOLS_VTABLE: ServiceVTable = ServiceVTable {
    invoke: mcp_tools_invoke,
    list_methods: mcp_tools_list_methods,
};

extern "C" fn mcp_tools_invoke(
    handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, lib_plugin_abi::ServiceError> {
    let ctx = handle as *mut PluginContext;
    mcp::invoke_tool(ctx, method.as_str(), args.as_str())
}

extern "C" fn mcp_tools_list_methods(
    _handle: *const c_void,
) -> abi_stable::std_types::RVec<lib_plugin_abi::ServiceMethod> {
    mcp::list_tools()
}

// === MCP Resources Service ===

static MCP_RESOURCES_VTABLE: ServiceVTable = ServiceVTable {
    invoke: mcp_resources_invoke,
    list_methods: mcp_resources_list_methods,
};

extern "C" fn mcp_resources_invoke(
    handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, lib_plugin_abi::ServiceError> {
    let ctx = handle as *mut PluginContext;
    mcp::invoke_resource(ctx, method.as_str(), args.as_str())
}

extern "C" fn mcp_resources_list_methods(
    _handle: *const c_void,
) -> abi_stable::std_types::RVec<lib_plugin_abi::ServiceMethod> {
    mcp::list_resources()
}
