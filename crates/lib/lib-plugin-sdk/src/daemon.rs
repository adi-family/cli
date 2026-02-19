//! Daemon service and command macros implementation

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Error, ImplItem, ItemImpl, LitStr, Result};

/// Information about daemon service
#[allow(dead_code)]
pub struct DaemonServiceInfo {
    pub has_start: bool,
    pub has_stop: bool,
    pub has_status: bool,
}

/// Parse daemon service from an impl block
pub fn parse_daemon_service_impl(input: &ItemImpl) -> Result<DaemonServiceInfo> {
    let mut info = DaemonServiceInfo {
        has_start: false,
        has_stop: false,
        has_status: false,
    };

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let name = method.sig.ident.to_string();
            match name.as_str() {
                "start" => info.has_start = true,
                "stop" => info.has_stop = true,
                "status" => info.has_status = true,
                _ => {}
            }
        }
    }

    // Validate required methods
    if !info.has_start {
        return Err(Error::new_spanned(
            input,
            "Daemon service must implement `start` method",
        ));
    }

    Ok(info)
}

/// Expand the #[daemon_service] attribute on an impl block
pub fn expand_daemon_service(input: ItemImpl) -> Result<TokenStream2> {
    let info = parse_daemon_service_impl(&input)?;

    // Extract the self type
    let self_ty = &input.self_ty;

    // Generate marker constant
    let marker = quote! {
        impl #self_ty {
            /// Marker indicating this plugin provides a daemon service
            #[doc(hidden)]
            pub const __SDK_HAS_DAEMON_SERVICE: bool = true;
        }
    };

    // Generate default stop implementation if not provided
    let default_stop = if !info.has_stop {
        quote! {
            impl #self_ty {
                /// Default stop implementation
                #[doc(hidden)]
                pub async fn __sdk_default_stop(&self) -> ::lib_plugin_abi_v3::Result<()> {
                    Ok(())
                }
            }
        }
    } else {
        quote! {}
    };

    // Generate default status implementation if not provided
    let default_status = if !info.has_status {
        quote! {
            impl #self_ty {
                /// Default status implementation
                #[doc(hidden)]
                pub async fn __sdk_default_status(&self) -> ::lib_plugin_abi_v3::daemon::ServiceStatus {
                    ::lib_plugin_abi_v3::daemon::ServiceStatus::Unknown
                }
            }
        }
    } else {
        quote! {}
    };

    // Keep original impl and add markers
    Ok(quote! {
        #input

        #marker
        #default_stop
        #default_status
    })
}

/// Expand daemon_cmd! or daemon_sudo! macro
pub fn expand_daemon_cmd(input: TokenStream, is_sudo: bool) -> TokenStream {
    let cmd: LitStr = match syn::parse(input) {
        Ok(lit) => lit,
        Err(e) => return e.to_compile_error().into(),
    };

    let cmd_value = cmd.value();

    // Generate a DaemonCommand struct
    let output = if is_sudo {
        quote! {
            ::lib_plugin_abi_v3::daemon::DaemonCommand::sudo(#cmd_value)
        }
    } else {
        quote! {
            ::lib_plugin_abi_v3::daemon::DaemonCommand::regular(#cmd_value)
        }
    };

    output.into()
}
