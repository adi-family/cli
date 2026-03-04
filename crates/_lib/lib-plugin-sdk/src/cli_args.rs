//! CliArgs derive macro implementation

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Expr, Field, Fields, Lit, Result, Token, Type};

/// Parsed #[arg(...)] attribute
struct ArgAttr {
    long: Option<String>,
    #[allow(dead_code)]
    short: Option<char>,
    position: Option<u8>,
    default: Option<Expr>,
}

impl ArgAttr {
    fn parse_from_field(field: &Field) -> Result<Self> {
        let mut long = None;
        let mut short = None;
        let mut position = None;
        let mut default = None;

        for attr in &field.attrs {
            if !attr.path().is_ident("arg") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("long") {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: Lit = meta.input.parse()?;
                        if let Lit::Str(s) = lit {
                            long = Some(s.value());
                        }
                    } else {
                        // #[arg(long)] without value - use field name
                        long = Some(String::new()); // marker to use field name
                    }
                    Ok(())
                } else if meta.path.is_ident("short") {
                    if meta.input.peek(Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let lit: Lit = meta.input.parse()?;
                        if let Lit::Char(c) = lit {
                            short = Some(c.value());
                        }
                    }
                    Ok(())
                } else if meta.path.is_ident("position") {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: Lit = meta.input.parse()?;
                    if let Lit::Int(i) = lit {
                        position = Some(i.base10_parse()?);
                    }
                    Ok(())
                } else if meta.path.is_ident("default") {
                    let _: Token![=] = meta.input.parse()?;
                    let expr: Expr = meta.input.parse()?;
                    default = Some(expr);
                    Ok(())
                } else {
                    Err(meta.error("unknown arg attribute"))
                }
            })?;
        }

        Ok(ArgAttr {
            long,
            short,
            position,
            default,
        })
    }
}

/// Analyze a type to determine if it's optional and get inner type
fn analyze_type(ty: &Type) -> (bool, &Type, &'static str) {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();

            // Check for Option<T>
            if ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let inner_type = get_cli_arg_type(inner);
                        return (true, inner, inner_type);
                    }
                }
            }

            // Check for Vec<T>
            if ident == "Vec" {
                return (true, ty, "String"); // Vecs are always optional (can be empty)
            }

            return (false, ty, get_cli_arg_type(ty));
        }
    }
    (false, ty, "String")
}

/// Get CLI arg type string from Rust type
fn get_cli_arg_type(ty: &Type) -> &'static str {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            return match ident.as_str() {
                "String" | "str" | "PathBuf" => "String",
                "i8" | "i16" | "i32" | "i64" | "isize" => "Int",
                "u8" | "u16" | "u32" | "u64" | "usize" => "Int",
                "f32" | "f64" => "Float",
                "bool" => "Bool",
                _ => "String",
            };
        }
    }
    "String"
}

/// Expand the derive(CliArgs) macro
pub fn expand_cli_args(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new_spanned(input, "CliArgs requires named fields")),
        },
        _ => {
            return Err(Error::new_spanned(
                input,
                "CliArgs can only be derived for structs",
            ))
        }
    };

    let mut schema_items = Vec::new();
    let mut parse_items = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;

        let attr = ArgAttr::parse_from_field(field)?;
        let (is_optional, _inner_type, cli_type_str) = analyze_type(field_type);

        // Determine the argument name
        let arg_name = if attr.position.is_some() {
            // Positional argument
            field_name_str.clone()
        } else {
            // Flag/option - use --name format
            let long_name = attr
                .long
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| s.clone())
                .unwrap_or_else(|| field_name_str.replace('_', "-"));
            format!("--{}", long_name)
        };

        // Required if not Option<T> and no default
        let is_required = !is_optional && attr.default.is_none();

        // CLI type token
        let cli_type = format_ident!("{}", cli_type_str);

        // Build schema entry
        if let Some(pos) = attr.position {
            schema_items.push(quote! {
                CliArg::positional(#pos, #arg_name, CliArgType::#cli_type, #is_required)
            });
        } else if is_required {
            schema_items.push(quote! {
                CliArg::required(#arg_name, CliArgType::#cli_type)
            });
        } else {
            schema_items.push(quote! {
                CliArg::optional(#arg_name, CliArgType::#cli_type)
            });
        }

        // Build parse entry
        let parse_key = field_name_str.replace('_', "-");

        if let Some(pos) = attr.position {
            // Positional argument
            let pos_usize = pos as usize;
            if is_optional {
                parse_items.push(quote! {
                    let #field_name: #field_type = __ctx.arg(#pos_usize).map(|s| s.parse().ok()).flatten();
                });
            } else if let Some(default_expr) = &attr.default {
                parse_items.push(quote! {
                    let #field_name: #field_type = __ctx.arg(#pos_usize)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(|| #default_expr);
                });
            } else {
                parse_items.push(quote! {
                    let #field_name: #field_type = __ctx.arg(#pos_usize)
                        .ok_or_else(|| format!("Missing required argument: {}", #arg_name))?
                        .parse()
                        .map_err(|_| format!("Invalid value for {}", #arg_name))?;
                });
            }
        } else if cli_type_str == "Bool" {
            // Boolean flag
            parse_items.push(quote! {
                let #field_name: #field_type = __ctx.has_flag(#parse_key);
            });
        } else if is_optional {
            // Optional flag/option
            parse_items.push(quote! {
                let #field_name: #field_type = __ctx.option(#parse_key);
            });
        } else if let Some(default_expr) = &attr.default {
            // Required with default
            parse_items.push(quote! {
                let #field_name: #field_type = __ctx.option(#parse_key).unwrap_or_else(|| #default_expr);
            });
        } else {
            // Required, no default
            parse_items.push(quote! {
                let #field_name: #field_type = __ctx.option(#parse_key)
                    .ok_or_else(|| format!("Missing required option: {}", #arg_name))?;
            });
        }
    }

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    Ok(quote! {
        impl CliArgsTrait for #name {
            fn schema() -> Vec<CliArg> {
                vec![
                    #(#schema_items),*
                ]
            }

            fn parse(__ctx: &CliContext) -> std::result::Result<Self, String> {
                #(#parse_items)*

                Ok(Self {
                    #(#field_names),*
                })
            }
        }
    })
}
