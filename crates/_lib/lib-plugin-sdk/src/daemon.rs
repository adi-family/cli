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
    pub has_reload: bool,
}

/// Parse daemon service from an impl block
pub fn parse_daemon_service_impl(input: &ItemImpl) -> Result<DaemonServiceInfo> {
    let mut info = DaemonServiceInfo {
        has_start: false,
        has_stop: false,
        has_status: false,
        has_reload: false,
    };

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let name = method.sig.ident.to_string();
            match name.as_str() {
                "start" => info.has_start = true,
                "stop" => info.has_stop = true,
                "status" => info.has_status = true,
                "reload" => info.has_reload = true,
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

    // Generate default methods for missing optional implementations
    let default_stop = if !info.has_stop {
        quote! {
            impl #self_ty {
                #[doc(hidden)]
                pub async fn __sdk_default_stop(&self) -> ::lib_plugin_abi_v3::Result<()> {
                    Ok(())
                }
            }
        }
    } else {
        quote! {}
    };

    let default_status = if !info.has_status {
        quote! {
            impl #self_ty {
                #[doc(hidden)]
                pub async fn __sdk_default_status(&self) -> ::lib_plugin_abi_v3::daemon::ServiceStatus {
                    ::lib_plugin_abi_v3::daemon::ServiceStatus::Unknown
                }
            }
        }
    } else {
        quote! {}
    };

    let default_reload = if !info.has_reload {
        quote! {
            impl #self_ty {
                #[doc(hidden)]
                pub async fn __sdk_default_reload(&self) -> ::lib_plugin_abi_v3::Result<()> {
                    Ok(())
                }
            }
        }
    } else {
        quote! {}
    };

    // Delegate to user method if present, otherwise to the generated default
    let stop_delegation = if info.has_stop {
        quote! { self.stop().await }
    } else {
        quote! { self.__sdk_default_stop().await }
    };

    let status_delegation = if info.has_status {
        quote! { self.status().await }
    } else {
        quote! { self.__sdk_default_status().await }
    };

    let reload_delegation = if info.has_reload {
        quote! { self.reload().await }
    } else {
        quote! { self.__sdk_default_reload().await }
    };

    // DaemonService trait impl delegating to user methods
    let trait_impl = quote! {
        #[::async_trait::async_trait]
        impl ::lib_plugin_abi_v3::daemon::DaemonService for #self_ty {
            async fn start(&self, ctx: ::lib_plugin_abi_v3::daemon::DaemonContext) -> ::lib_plugin_abi_v3::Result<()> {
                self.start(ctx).await
            }

            async fn stop(&self) -> ::lib_plugin_abi_v3::Result<()> {
                #stop_delegation
            }

            async fn status(&self) -> ::lib_plugin_abi_v3::daemon::ServiceStatus {
                #status_delegation
            }

            async fn reload(&self) -> ::lib_plugin_abi_v3::Result<()> {
                #reload_delegation
            }
        }
    };

    // Entry point for dynamic loading
    let entry_point = quote! {
        #[no_mangle]
        pub fn plugin_create_daemon_service() -> Box<dyn ::lib_plugin_abi_v3::daemon::DaemonService> {
            Box::new(#self_ty::new())
        }
    };

    Ok(quote! {
        #input

        #marker
        #default_stop
        #default_status
        #default_reload
        #trait_impl
        #entry_point
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
