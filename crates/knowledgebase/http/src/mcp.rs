//! MCP (Model Context Protocol) support for ADI Knowledgebase.
//!
//! Exposes knowledgebase operations as MCP tools via SSE transport.

use knowledgebase_core::{EdgeType, Knowledgebase, NodeType};
use axum::Router;
use futures::StreamExt;
use lib_mcp_core::{
    prelude::*,
    server::{McpRouter, McpServerBuilder},
    transport::sse_server::SseServerState,
};
use std::pin::pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Build MCP server with knowledgebase tools.
fn build_mcp_server(kb: Arc<RwLock<Option<Knowledgebase>>>) -> impl McpHandler {
    let kb_query = kb.clone();
    let kb_query_subgraph = kb.clone();
    let kb_add = kb.clone();
    let kb_get = kb.clone();
    let kb_delete = kb.clone();
    let kb_approve = kb.clone();
    let kb_add_edge = kb.clone();
    let kb_conflicts = kb.clone();
    let kb_orphans = kb.clone();

    McpServerBuilder::new("adi-knowledgebase", env!("CARGO_PKG_VERSION"))
        .instructions(
            "ADI Knowledgebase MCP server. Provides tools for querying and managing \
             a semantic knowledge graph with embeddings-based search. Use 'query' for \
             natural language search, 'query_subgraph' for related knowledge graphs, \
             and 'add_node' to store new knowledge.",
        )
        // Query tool - semantic search
        .tool(
            Tool::new(
                "query",
                ToolInputSchema::new()
                    .string_property("question", "Natural language question to search for", true)
                    .integer_property("limit", "Maximum number of results (default: 5)", false),
            )
            .with_description(
                "Search the knowledgebase using semantic similarity. Returns relevant \
                 knowledge nodes ranked by relevance score.",
            ),
            move |args| {
                let kb = kb_query.clone();
                async move {
                    let question = args
                        .get("question")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("question is required".into()))?;

                    let limit = args
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5) as usize;

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    let results = kb_ref
                        .query(question)
                        .await
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let results: Vec<_> = results.into_iter().take(limit).collect();
                    let json = serde_json::to_string_pretty(&results)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(json))
                }
            },
        )
        // Query subgraph tool - get related knowledge graph
        .tool(
            Tool::new(
                "query_subgraph",
                ToolInputSchema::new()
                    .string_property("question", "Natural language question", true),
            )
            .with_description(
                "Get a subgraph of related knowledge nodes and edges. Useful for \
                 understanding context and relationships between concepts.",
            ),
            move |args| {
                let kb = kb_query_subgraph.clone();
                async move {
                    let question = args
                        .get("question")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("question is required".into()))?;

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    let subgraph = kb_ref
                        .query_subgraph(question)
                        .await
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let json = serde_json::to_string_pretty(&subgraph)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(json))
                }
            },
        )
        // Add node tool
        .tool(
            Tool::new(
                "add_node",
                ToolInputSchema::new()
                    .string_property("user_said", "Original user statement or context", true)
                    .string_property(
                        "derived_knowledge",
                        "Extracted knowledge to store",
                        true,
                    )
                    .string_property(
                        "node_type",
                        "Type: decision, fact, error, guide, glossary, context, assumption",
                        false,
                    ),
            )
            .with_description(
                "Add a new knowledge node to the knowledgebase. The system will \
                 automatically detect duplicates and create relationships to related nodes.",
            ),
            move |args| {
                let kb = kb_add.clone();
                async move {
                    let user_said = args
                        .get("user_said")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("user_said is required".into()))?;

                    let derived_knowledge = args
                        .get("derived_knowledge")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            Error::InvalidParams("derived_knowledge is required".into())
                        })?;

                    let node_type = args
                        .get("node_type")
                        .and_then(|v| v.as_str())
                        .map(|s| match s {
                            "decision" => NodeType::Decision,
                            "fact" => NodeType::Fact,
                            "error" => NodeType::Error,
                            "guide" => NodeType::Guide,
                            "glossary" => NodeType::Glossary,
                            "context" => NodeType::Context,
                            "assumption" => NodeType::Assumption,
                            _ => NodeType::Fact,
                        })
                        .unwrap_or(NodeType::Fact);

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    let node = kb_ref
                        .add_from_user(user_said, derived_knowledge, node_type)
                        .await
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let json = serde_json::to_string_pretty(&node)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(json))
                }
            },
        )
        // Get node tool
        .tool(
            Tool::new(
                "get_node",
                ToolInputSchema::new()
                    .string_property("id", "UUID of the node to retrieve", true),
            )
            .with_description("Get a specific knowledge node by its ID."),
            move |args| {
                let kb = kb_get.clone();
                async move {
                    let id_str = args
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("id is required".into()))?;

                    let id = uuid::Uuid::parse_str(id_str)
                        .map_err(|e| Error::InvalidParams(format!("Invalid UUID: {}", e)))?;

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    match kb_ref.get_node(id) {
                        Ok(Some(node)) => {
                            let json = serde_json::to_string_pretty(&node)
                                .map_err(|e| Error::Internal(e.to_string()))?;
                            Ok(CallToolResult::text(json))
                        }
                        Ok(None) => Ok(CallToolResult::error("Node not found")),
                        Err(e) => Ok(CallToolResult::error(e.to_string())),
                    }
                }
            },
        )
        // Delete node tool
        .tool(
            Tool::new(
                "delete_node",
                ToolInputSchema::new()
                    .string_property("id", "UUID of the node to delete", true),
            )
            .with_description("Delete a knowledge node from the graph."),
            move |args| {
                let kb = kb_delete.clone();
                async move {
                    let id_str = args
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("id is required".into()))?;

                    let id = uuid::Uuid::parse_str(id_str)
                        .map_err(|e| Error::InvalidParams(format!("Invalid UUID: {}", e)))?;

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    kb_ref
                        .delete_node(id)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(format!("Deleted node {}", id)))
                }
            },
        )
        // Approve node tool
        .tool(
            Tool::new(
                "approve_node",
                ToolInputSchema::new()
                    .string_property("id", "UUID of the node to approve", true),
            )
            .with_description(
                "Approve a knowledge node, setting its confidence to 1.0. \
                 Use this to confirm that knowledge is accurate.",
            ),
            move |args| {
                let kb = kb_approve.clone();
                async move {
                    let id_str = args
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("id is required".into()))?;

                    let id = uuid::Uuid::parse_str(id_str)
                        .map_err(|e| Error::InvalidParams(format!("Invalid UUID: {}", e)))?;

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    kb_ref
                        .approve(id)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(format!("Approved node {}", id)))
                }
            },
        )
        // Add edge tool
        .tool(
            Tool::new(
                "add_edge",
                ToolInputSchema::new()
                    .string_property("from_id", "UUID of the source node", true)
                    .string_property("to_id", "UUID of the target node", true)
                    .string_property(
                        "edge_type",
                        "Type: supersedes, contradicts, requires, related_to, derived_from, answers",
                        false,
                    )
                    .string_property("weight", "Edge weight 0.0-1.0 (default: 0.5)", false),
            )
            .with_description(
                "Create an edge (relationship) between two knowledge nodes. \
                 Edge types define the semantic relationship.",
            ),
            move |args| {
                let kb = kb_add_edge.clone();
                async move {
                    let from_str = args
                        .get("from_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("from_id is required".into()))?;

                    let to_str = args
                        .get("to_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::InvalidParams("to_id is required".into()))?;

                    let from_id = uuid::Uuid::parse_str(from_str)
                        .map_err(|e| Error::InvalidParams(format!("Invalid from_id UUID: {}", e)))?;

                    let to_id = uuid::Uuid::parse_str(to_str)
                        .map_err(|e| Error::InvalidParams(format!("Invalid to_id UUID: {}", e)))?;

                    let edge_type = args
                        .get("edge_type")
                        .and_then(|v| v.as_str())
                        .map(|s| match s {
                            "supersedes" => EdgeType::Supersedes,
                            "contradicts" => EdgeType::Contradicts,
                            "requires" => EdgeType::Requires,
                            "related_to" => EdgeType::RelatedTo,
                            "derived_from" => EdgeType::DerivedFrom,
                            "answers" => EdgeType::Answers,
                            _ => EdgeType::RelatedTo,
                        })
                        .unwrap_or(EdgeType::RelatedTo);

                    let weight = args
                        .get("weight")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.5);

                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    let edge = kb_ref
                        .add_edge(from_id, to_id, edge_type, weight)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let json = serde_json::to_string_pretty(&edge)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(json))
                }
            },
        )
        // Get conflicts tool
        .tool(
            Tool::new("get_conflicts", ToolInputSchema::new())
                .with_description("Get all conflicting knowledge nodes that need resolution."),
            move |_args| {
                let kb = kb_conflicts.clone();
                async move {
                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    let conflicts = kb_ref
                        .get_conflicts()
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let result: Vec<_> = conflicts
                        .into_iter()
                        .map(|(a, b)| {
                            serde_json::json!({
                                "node_a": a,
                                "node_b": b
                            })
                        })
                        .collect();

                    let json = serde_json::to_string_pretty(&result)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(json))
                }
            },
        )
        // Get orphans tool
        .tool(
            Tool::new("get_orphans", ToolInputSchema::new())
                .with_description("Get knowledge nodes with no connections to other nodes."),
            move |_args| {
                let kb = kb_orphans.clone();
                async move {
                    let kb_guard = kb.read().await;
                    let kb_ref = kb_guard
                        .as_ref()
                        .ok_or_else(|| Error::Internal("Knowledgebase not initialized".into()))?;

                    let orphans = kb_ref
                        .get_orphans()
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let json = serde_json::to_string_pretty(&orphans)
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    Ok(CallToolResult::text(json))
                }
            },
        )
        .build()
}

/// Create MCP router for knowledgebase.
pub fn create_mcp_router(kb: Arc<RwLock<Option<Knowledgebase>>>) -> Router {
    let sse_state = SseServerState::new();
    let kb_clone = kb.clone();

    // Spawn MCP message handler
    let sse_state_clone = sse_state.clone();
    tokio::spawn(async move {
        let server = build_mcp_server(kb_clone);
        let mut router = McpRouter::new(server);

        info!("MCP server started, waiting for connections...");

        // Process incoming messages from SSE transport
        let mut stream = pin!(sse_state_clone.incoming_stream());
        while let Some((session_id, message)) = stream.next().await {
            debug!(session = %session_id, "Processing MCP message");

            if let Some(response) = router.handle_message(message).await {
                if let Err(e) = sse_state_clone.broadcast(response) {
                    error!(error = %e, "Failed to broadcast MCP response");
                }
            }
        }
    });

    // Return SSE router
    sse_state.router()
}
