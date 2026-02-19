//! Plugin struct annotation macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, Ident, ItemStruct, Result, Visibility};

/// Information extracted from the plugin struct
#[allow(dead_code)]
pub struct PluginInfo {
    pub name: Ident,
    pub vis: Visibility,
    pub fields: Fields,
}

/// Parse a plugin struct
pub fn parse_plugin_struct(input: &ItemStruct) -> Result<PluginInfo> {
    Ok(PluginInfo {
        name: input.ident.clone(),
        vis: input.vis.clone(),
        fields: input.fields.clone(),
    })
}

/// Expand the #[plugin] attribute macro
pub fn expand_plugin(input: ItemStruct) -> Result<TokenStream> {
    let info = parse_plugin_struct(&input)?;
    let struct_name = &info.name;
    let vis = &info.vis;

    // Generate the struct with additional fields for SDK tracking
    let sdk_fields = quote! {
        /// Plugin commands registry (populated by #[command] macros)
        #[doc(hidden)]
        pub __sdk_commands: ::std::vec::Vec<::lib_plugin_abi_v3::cli::CliCommand>,
        /// Global commands registry (populated by #[global_command] macros)
        #[doc(hidden)]
        pub __sdk_global_commands: ::std::vec::Vec<::lib_plugin_abi_v3::cli::CliCommand>,
        /// HTTP routes flag (set by #[http_routes] macro)
        #[doc(hidden)]
        pub __sdk_has_http: bool,
        /// WebRTC handlers flag (set by #[webrtc_handlers] macro)
        #[doc(hidden)]
        pub __sdk_has_webrtc: bool,
        /// Daemon service flag (set by #[daemon_service] macro)
        #[doc(hidden)]
        pub __sdk_has_daemon: bool,
    };

    // Reconstruct struct with SDK fields injected
    let new_struct = match &info.fields {
        Fields::Named(fields) => {
            let existing_fields = &fields.named;
            quote! {
                #[derive(Default)]
                #vis struct #struct_name {
                    #existing_fields
                    #sdk_fields
                }
            }
        }
        Fields::Unnamed(_) => {
            // For tuple structs, we can't add named fields
            // Keep the original and add a separate SDK state
            return Err(Error::new_spanned(
                &input,
                "Tuple structs are not supported for plugins. Use named fields.",
            ));
        }
        Fields::Unit => {
            quote! {
                #[derive(Default)]
                #vis struct #struct_name {
                    #sdk_fields
                }
            }
        }
    };

    // Generate helper methods
    let impl_block = quote! {
        impl #struct_name {
            /// Create a new plugin instance
            pub fn new() -> Self {
                Self::default()
            }

            /// Register a command (called by #[command] macro)
            #[doc(hidden)]
            pub fn __sdk_register_command(&mut self, cmd: ::lib_plugin_abi_v3::cli::CliCommand) {
                self.__sdk_commands.push(cmd);
            }

            /// Register a global command (called by #[global_command] macro)
            #[doc(hidden)]
            pub fn __sdk_register_global_command(&mut self, cmd: ::lib_plugin_abi_v3::cli::CliCommand) {
                self.__sdk_global_commands.push(cmd);
            }

            /// Get registered plugin commands
            pub fn commands(&self) -> &[::lib_plugin_abi_v3::cli::CliCommand] {
                &self.__sdk_commands
            }

            /// Get registered global commands
            pub fn global_commands(&self) -> &[::lib_plugin_abi_v3::cli::CliCommand] {
                &self.__sdk_global_commands
            }
        }
    };

    // Generate the entry point
    let entry_point = quote! {
        /// Plugin entry point (required by lib-plugin-abi-v3)
        #[no_mangle]
        pub fn plugin_create() -> Box<dyn ::lib_plugin_abi_v3::Plugin> {
            Box::new(#struct_name::new())
        }
    };

    Ok(quote! {
        #new_struct
        #impl_block
        #entry_point
    })
}
