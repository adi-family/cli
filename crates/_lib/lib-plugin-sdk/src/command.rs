//! Command macro implementation for #[command] and #[global_command]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Expr, FnArg, ImplItemFn, Lit, Meta, Pat, PatIdent, PatType, Result, Token, Type,
};

/// Command type: plugin subcommand or global CLI command
#[derive(Clone, Copy, PartialEq)]
pub enum CommandType {
    /// Plugin command: `adi <plugin> <cmd>`
    Plugin,
    /// Global command: `adi <cmd>`
    Global,
}

/// Parsed command attribute
pub struct CommandAttr {
    pub name: String,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub aliases: Vec<String>,
}

impl Parse for CommandAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut aliases = Vec::new();

        let pairs: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;

        for meta in pairs {
            match &meta {
                Meta::NameValue(nv) => {
                    let ident = nv
                        .path
                        .get_ident()
                        .ok_or_else(|| Error::new_spanned(&nv.path, "Expected identifier"))?;

                    let value = match &nv.value {
                        Expr::Lit(expr_lit) => match &expr_lit.lit {
                            Lit::Str(s) => s.value(),
                            _ => {
                                return Err(Error::new_spanned(
                                    &nv.value,
                                    "Expected string literal",
                                ))
                            }
                        },
                        _ => return Err(Error::new_spanned(&nv.value, "Expected string literal")),
                    };

                    match ident.to_string().as_str() {
                        "name" => name = Some(value),
                        "description" => description = Some(value),
                        "alias" => aliases.push(value),
                        other => {
                            return Err(Error::new_spanned(
                                ident,
                                format!("Unknown attribute: {}", other),
                            ))
                        }
                    }
                }
                _ => return Err(Error::new_spanned(&meta, "Expected name = \"value\"")),
            }
        }

        let name = name.ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                "Missing required attribute: name",
            )
        })?;

        Ok(CommandAttr {
            name,
            description,
            aliases,
        })
    }
}

/// Information about the args parameter
struct ArgsParam {
    /// The type of the args struct (e.g., ListArgs)
    ty: Type,
    /// The parameter name (e.g., args)
    name: syn::Ident,
}

/// Extract the typed args parameter from function signature
/// Returns None if function has no args (just &self)
fn extract_args_param(sig: &syn::Signature) -> Result<Option<ArgsParam>> {
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let name = match pat.as_ref() {
                    Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                    _ => continue,
                };

                return Ok(Some(ArgsParam {
                    ty: (**ty).clone(),
                    name,
                }));
            }
        }
    }

    Ok(None)
}

/// Expand the #[command] or #[global_command] attribute
pub fn expand_command(
    attr: CommandAttr,
    input: ImplItemFn,
    _cmd_type: CommandType,
) -> Result<TokenStream> {
    let fn_name = &input.sig.ident;
    let cmd_name = &attr.name;

    // Extract args parameter if present
    let args_param = extract_args_param(&input.sig)?;

    // Generate description key for i18n (or use provided)
    let description = attr
        .description
        .unwrap_or_else(|| format!("cmd-{}-help", cmd_name.replace('_', "-")));

    // Generate command metadata function
    let meta_fn_name = format_ident!("__sdk_cmd_meta_{}", fn_name);

    let (schema_expr, handler_body) = if let Some(args_param) = &args_param {
        let args_ty = &args_param.ty;
        let args_name = &args_param.name;

        // Has typed args - use CliArgs trait
        let schema = quote! {
            <#args_ty as CliArgsTrait>::schema()
        };

        let body = quote! {
            let #args_name = <#args_ty as CliArgsTrait>::parse(__ctx)
                .map_err(|e| PluginError::InvalidInput(e))?;
            let result = self.#fn_name(#args_name).await;
            match result {
                Ok(output) => Ok(CliResult::success(output)),
                Err(e) => Ok(CliResult::error(e)),
            }
        };

        (schema, body)
    } else {
        // No args - empty schema
        let schema = quote! { vec![] };
        let body = quote! {
            let result = self.#fn_name().await;
            match result {
                Ok(output) => Ok(CliResult::success(output)),
                Err(e) => Ok(CliResult::error(e)),
            }
        };

        (schema, body)
    };

    let cmd_metadata = quote! {
        #[doc(hidden)]
        pub fn #meta_fn_name() -> CliCommand {
            CliCommand {
                name: #cmd_name.to_string(),
                description: #description.to_string(),
                args: #schema_expr,
                has_subcommands: false,
            }
        }
    };

    // Generate handler function
    let handler_name = format_ident!("__sdk_cmd_handler_{}", fn_name);

    let handler = quote! {
        #[doc(hidden)]
        pub async fn #handler_name(
            &self,
            __ctx: &CliContext
        ) -> Result<CliResult> {
            #handler_body
        }
    };

    // Return original function + metadata + handler
    Ok(quote! {
        #input
        #cmd_metadata
        #handler
    })
}
