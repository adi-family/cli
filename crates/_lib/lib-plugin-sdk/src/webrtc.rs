//! WebRTC handlers macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl, Result};

/// Information about WebRTC handlers
#[allow(dead_code)]
pub struct WebRtcHandlersInfo {
    pub has_on_connect: bool,
    pub has_on_message: bool,
    pub has_on_disconnect: bool,
}

/// Parse WebRTC handlers from an impl block
pub fn parse_webrtc_handlers_impl(input: &ItemImpl) -> Result<WebRtcHandlersInfo> {
    let mut info = WebRtcHandlersInfo {
        has_on_connect: false,
        has_on_message: false,
        has_on_disconnect: false,
    };

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let name = method.sig.ident.to_string();
            match name.as_str() {
                "on_connect" => info.has_on_connect = true,
                "on_message" => info.has_on_message = true,
                "on_disconnect" => info.has_on_disconnect = true,
                _ => {}
            }
        }
    }

    Ok(info)
}

/// Expand the #[webrtc_handlers] attribute on an impl block
pub fn expand_webrtc_handlers(input: ItemImpl) -> Result<TokenStream> {
    let _info = parse_webrtc_handlers_impl(&input)?;

    // Extract the self type
    let self_ty = &input.self_ty;

    // Generate marker constant
    let marker = quote! {
        impl #self_ty {
            /// Marker indicating this plugin provides WebRTC handlers
            #[doc(hidden)]
            pub const __SDK_HAS_WEBRTC: bool = true;
        }
    };

    // Keep original impl and add marker
    Ok(quote! {
        #input

        #marker
    })
}
