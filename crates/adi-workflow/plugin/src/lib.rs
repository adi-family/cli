//! ADI Workflow Plugin
//!
//! Run workflows defined in TOML files with interactive prompts and templating.

mod cli;
mod discovery;
mod executor;
mod parser;
mod prelude;
mod prompts;
mod template;

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};
use std::ffi::c_void;

const SERVICE_CLI: &str = "adi.workflow.cli";

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.workflow",
        "ADI Workflow",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Run workflows defined in TOML files")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI commands service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.workflow")
                .with_description("CLI commands for workflow execution");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!("Failed to register CLI service: {}", code));
            return code;
        }

        host.info("ADI Workflow plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

extern "C" fn handle_message(
    _ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    _msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    RResult::RErr(PluginError::new(
        -1,
        format!("Unknown message type: {}", msg_type.as_str()),
    ))
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

// === CLI Service VTable ===

static CLI_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: cli_invoke,
    list_methods: cli_list_methods,
};

extern "C" fn cli_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        "run_command" => {
            let result = cli::run_command(args.as_str());
            match result {
                Ok(output) => RResult::ROk(RString::from(output)),
                Err(e) => RResult::RErr(ServiceError::invocation_error(e)),
            }
        }
        "list_commands" => {
            let commands = cli::list_commands();
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
    ]
    .into_iter()
    .collect()
}
