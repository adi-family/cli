//! PHP Language Support Plugin

mod analyzer;

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_indexer_lang_abi::{
    ExtractRequest, GrammarPathResponse, LanguageInfoAbi, METHOD_EXTRACT_REFERENCES,
    METHOD_EXTRACT_SYMBOLS, METHOD_GET_GRAMMAR_PATH, METHOD_GET_INFO,
};
use lib_plugin_abi::{
    PluginContext, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError, ServiceHandle,
    ServiceMethod, ServiceVTable, ServiceVersion,
};
use std::ffi::c_void;

const LANGUAGE: &str = "php";
const SERVICE_ID: &str = "adi.indexer.lang.php";

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.lang.php",
        "PHP Language Support",
        env!("CARGO_PKG_VERSION"),
        "language",
    )
    .with_author("ADI Team")
    .with_description("PHP language parsing and analysis for ADI indexer")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();
        let descriptor =
            ServiceDescriptor::new(SERVICE_ID, ServiceVersion::new(1, 0, 0), "adi.lang.php")
                .with_description("PHP language analyzer for code indexing");
        let handle = ServiceHandle::new(
            SERVICE_ID,
            ctx as *const c_void,
            &ANALYZER_SERVICE_VTABLE as *const ServiceVTable,
        );
        if let Err(code) = host.register_svc(descriptor, handle) {
            host.error(&format!(
                "Failed to register PHP analyzer service: {}",
                code
            ));
            return code;
        }
        host.info("PHP language plugin initialized");
    }
    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RNone,
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}

static ANALYZER_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: analyzer_invoke,
    list_methods: analyzer_list_methods,
};

extern "C" fn analyzer_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        METHOD_GET_GRAMMAR_PATH => handle_get_grammar_path(),
        METHOD_EXTRACT_SYMBOLS => handle_extract_symbols(args.as_str()),
        METHOD_EXTRACT_REFERENCES => handle_extract_references(args.as_str()),
        METHOD_GET_INFO => handle_get_info(),
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn analyzer_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new(METHOD_GET_GRAMMAR_PATH)
            .with_description("Get path to tree-sitter grammar shared library"),
        ServiceMethod::new(METHOD_EXTRACT_SYMBOLS)
            .with_description("Extract symbols from PHP source code"),
        ServiceMethod::new(METHOD_EXTRACT_REFERENCES)
            .with_description("Extract references from PHP source code"),
        ServiceMethod::new(METHOD_GET_INFO).with_description("Get language plugin information"),
    ]
    .into_iter()
    .collect()
}

fn handle_get_grammar_path() -> RResult<RString, ServiceError> {
    let response = GrammarPathResponse {
        path: "builtin".to_string(),
    };
    match serde_json::to_string(&response) {
        Ok(json) => RResult::ROk(RString::from(json)),
        Err(e) => RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e))),
    }
}

fn handle_extract_symbols(args: &str) -> RResult<RString, ServiceError> {
    let request: ExtractRequest = match serde_json::from_str(args) {
        Ok(r) => r,
        Err(e) => {
            return RResult::RErr(ServiceError::invocation_error(format!(
                "Invalid request: {}",
                e
            )))
        }
    };
    let symbols = analyzer::extract_symbols(&request.source);
    match serde_json::to_string(&symbols) {
        Ok(json) => RResult::ROk(RString::from(json)),
        Err(e) => RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e))),
    }
}

fn handle_extract_references(args: &str) -> RResult<RString, ServiceError> {
    let request: ExtractRequest = match serde_json::from_str(args) {
        Ok(r) => r,
        Err(e) => {
            return RResult::RErr(ServiceError::invocation_error(format!(
                "Invalid request: {}",
                e
            )))
        }
    };
    let references = analyzer::extract_references(&request.source);
    match serde_json::to_string(&references) {
        Ok(json) => RResult::ROk(RString::from(json)),
        Err(e) => RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e))),
    }
}

fn handle_get_info() -> RResult<RString, ServiceError> {
    let info = LanguageInfoAbi::new(LANGUAGE, env!("CARGO_PKG_VERSION"))
        .with_extensions(["php", "phtml", "php3", "php4", "php5", "phps"])
        .with_display_name("PHP");
    match serde_json::to_string(&info) {
        Ok(json) => RResult::ROk(RString::from(json)),
        Err(e) => RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e))),
    }
}
