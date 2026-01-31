//! C/C++ Language Support Plugin

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

const LANGUAGE: &str = "cpp";
const SERVICE_ID: &str = "adi.indexer.lang.cpp";

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.lang.cpp",
        "C/C++ Language Support",
        env!("CARGO_PKG_VERSION"),
        "language",
    )
    .with_author("ADI Team")
    .with_description("C and C++ language parsing and analysis")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register C++ service
        let cpp_descriptor =
            ServiceDescriptor::new(SERVICE_ID, ServiceVersion::new(1, 0, 0), "adi.lang.cpp")
                .with_description("C++ language analyzer");
        let cpp_handle = ServiceHandle::new(
            SERVICE_ID,
            ctx as *const c_void,
            &CPP_ANALYZER_VTABLE as *const ServiceVTable,
        );
        if let Err(code) = host.register_svc(cpp_descriptor, cpp_handle) {
            host.error(&format!("Failed to register C++ analyzer: {}", code));
            return code;
        }

        // Register C service
        let c_descriptor = ServiceDescriptor::new(
            "adi.indexer.lang.c",
            ServiceVersion::new(1, 0, 0),
            "adi.lang.cpp",
        )
        .with_description("C language analyzer");
        let c_handle = ServiceHandle::new(
            "adi.indexer.lang.c",
            ctx as *const c_void,
            &C_ANALYZER_VTABLE as *const ServiceVTable,
        );
        if let Err(code) = host.register_svc(c_descriptor, c_handle) {
            host.error(&format!("Failed to register C analyzer: {}", code));
            return code;
        }

        host.info("C/C++ language plugin initialized");
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

static CPP_ANALYZER_VTABLE: ServiceVTable = ServiceVTable {
    invoke: cpp_analyzer_invoke,
    list_methods: analyzer_list_methods,
};

static C_ANALYZER_VTABLE: ServiceVTable = ServiceVTable {
    invoke: c_analyzer_invoke,
    list_methods: analyzer_list_methods,
};

extern "C" fn cpp_analyzer_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    analyzer_invoke_impl(method.as_str(), args.as_str(), true)
}

extern "C" fn c_analyzer_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    analyzer_invoke_impl(method.as_str(), args.as_str(), false)
}

fn analyzer_invoke_impl(method: &str, args: &str, is_cpp: bool) -> RResult<RString, ServiceError> {
    match method {
        METHOD_GET_GRAMMAR_PATH => {
            let response = GrammarPathResponse {
                path: "builtin".to_string(),
            };
            match serde_json::to_string(&response) {
                Ok(json) => RResult::ROk(RString::from(json)),
                Err(e) => {
                    RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e)))
                }
            }
        }
        METHOD_EXTRACT_SYMBOLS => {
            let request: ExtractRequest = match serde_json::from_str(args) {
                Ok(r) => r,
                Err(e) => {
                    return RResult::RErr(ServiceError::invocation_error(format!(
                        "Invalid request: {}",
                        e
                    )))
                }
            };
            let symbols = analyzer::extract_symbols(&request.source, is_cpp);
            match serde_json::to_string(&symbols) {
                Ok(json) => RResult::ROk(RString::from(json)),
                Err(e) => {
                    RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e)))
                }
            }
        }
        METHOD_EXTRACT_REFERENCES => {
            let request: ExtractRequest = match serde_json::from_str(args) {
                Ok(r) => r,
                Err(e) => {
                    return RResult::RErr(ServiceError::invocation_error(format!(
                        "Invalid request: {}",
                        e
                    )))
                }
            };
            let references = analyzer::extract_references(&request.source, is_cpp);
            match serde_json::to_string(&references) {
                Ok(json) => RResult::ROk(RString::from(json)),
                Err(e) => {
                    RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e)))
                }
            }
        }
        METHOD_GET_INFO => {
            let (lang, exts, display) = if is_cpp {
                ("cpp", vec!["cpp", "cc", "cxx", "hpp", "hh", "hxx"], "C++")
            } else {
                ("c", vec!["c", "h"], "C")
            };
            let info = LanguageInfoAbi::new(lang, env!("CARGO_PKG_VERSION"))
                .with_extensions(exts)
                .with_display_name(display);
            match serde_json::to_string(&info) {
                Ok(json) => RResult::ROk(RString::from(json)),
                Err(e) => {
                    RResult::RErr(ServiceError::invocation_error(format!("JSON error: {}", e)))
                }
            }
        }
        _ => RResult::RErr(ServiceError::method_not_found(method)),
    }
}

extern "C" fn analyzer_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new(METHOD_GET_GRAMMAR_PATH).with_description("Get grammar path"),
        ServiceMethod::new(METHOD_EXTRACT_SYMBOLS).with_description("Extract symbols"),
        ServiceMethod::new(METHOD_EXTRACT_REFERENCES).with_description("Extract references"),
        ServiceMethod::new(METHOD_GET_INFO).with_description("Get language info"),
    ]
    .into_iter()
    .collect()
}
