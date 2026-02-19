//! HTTP routes macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, ItemImpl, Result};

/// Information about HTTP routes
#[allow(dead_code)]
pub struct HttpRoutesInfo {
    pub has_routes: bool,
}

/// Parse HTTP routes from an impl block
#[allow(dead_code)]
pub fn parse_http_routes_impl(_input: &ItemImpl) -> Result<HttpRoutesInfo> {
    Ok(HttpRoutesInfo { has_routes: true })
}

/// Expand the #[http_routes] attribute on a function
pub fn expand_http_routes(input: ImplItemFn) -> Result<TokenStream> {
    // Generate the marker constant to indicate HTTP support
    let marker = quote! {
        /// Marker indicating this plugin provides HTTP routes
        #[doc(hidden)]
        const __SDK_HAS_HTTP_ROUTES: bool = true;
    };

    // Generate the HttpRoutes trait implementation helper
    let http_routes_impl = quote! {
        /// Get HTTP routes for this plugin
        #[doc(hidden)]
        pub fn __sdk_list_http_routes(&self) -> Vec<::lib_plugin_abi_v3::http::HttpRoute> {
            // The actual routes function returns an axum Router
            // We need to introspect it - but that's complex at compile time
            // For now, we mark that HTTP is supported and let runtime handle it
            vec![]
        }
    };

    // Keep original function and add marker
    Ok(quote! {
        #marker

        #input

        #http_routes_impl
    })
}
