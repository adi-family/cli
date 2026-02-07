//! Auto-generated enums from TypeSpec.
//! DO NOT EDIT.

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "enum")]
    Enum,
    #[serde(rename = "interface")]
    Interface,
    #[serde(rename = "trait")]
    Trait,
    #[serde(rename = "module")]
    Module,
    #[serde(rename = "constant")]
    Constant,
    #[serde(rename = "variable")]
    Variable,
    #[serde(rename = "type")]
    Type,
    #[serde(rename = "property")]
    Property,
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "constructor")]
    Constructor,
    #[serde(rename = "destructor")]
    Destructor,
    #[serde(rename = "operator")]
    Operator,
    #[serde(rename = "macro")]
    Macro,
    #[serde(rename = "namespace")]
    Namespace,
    #[serde(rename = "package")]
    Package,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "public_crate")]
    PublicCrate,
    #[serde(rename = "public_super")]
    PublicSuper,
    #[serde(rename = "protected")]
    Protected,
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "internal")]
    Internal,
    #[serde(rename = "unknown")]
    Unknown,
}
