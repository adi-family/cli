//! Auto-generated enums from TypeSpec.
//! DO NOT EDIT.

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    #[serde(rename = "decision")]
    Decision,
    #[serde(rename = "fact")]
    Fact,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "guide")]
    Guide,
    #[serde(rename = "glossary")]
    Glossary,
    #[serde(rename = "context")]
    Context,
    #[serde(rename = "assumption")]
    Assumption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    #[serde(rename = "supersedes")]
    Supersedes,
    #[serde(rename = "contradicts")]
    Contradicts,
    #[serde(rename = "requires")]
    Requires,
    #[serde(rename = "related_to")]
    RelatedTo,
    #[serde(rename = "derived_from")]
    DerivedFrom,
    #[serde(rename = "answers")]
    Answers,
}
