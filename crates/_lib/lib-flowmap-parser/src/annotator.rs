//! LLM-based flow annotation for human-readable descriptions.

use lib_flowmap_core::{FlowGraph, FlowIndex, NodeId};
use lib_client_anthropic::{ApiKeyAuth, Client, CreateMessageRequest, Message};
use std::collections::HashMap;

const HAIKU_MODEL: &str = "claude-3-5-haiku-20241022";

/// Annotator that uses Claude Haiku to generate descriptions.
pub struct FlowAnnotator {
    client: Client,
}

impl FlowAnnotator {
    /// Create a new annotator with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = Client::builder()
            .auth(ApiKeyAuth::new(api_key))
            .build();
        Self { client }
    }

    /// Annotate all nodes in a flow graph with human-readable descriptions.
    pub async fn annotate(&self, graph: &mut FlowGraph) -> crate::Result<()> {
        // Collect nodes that need annotation
        let nodes_to_annotate: Vec<_> = graph
            .nodes
            .values()
            .filter(|n| n.description.is_none())
            .map(|n| (n.id, n.code_label.clone(), format!("{:?}", n.kind)))
            .collect();

        if nodes_to_annotate.is_empty() {
            return Ok(());
        }

        // Build prompt
        let prompt = self.build_prompt(&nodes_to_annotate);

        // Call Haiku
        let request = CreateMessageRequest::new(
            HAIKU_MODEL,
            vec![Message::user(&prompt)],
            2048,
        )
        .with_system(SYSTEM_PROMPT)
        .with_temperature(0.0);

        let response = self.client.create_message(request).await
            .map_err(|e| crate::ParseError::Annotation(e.to_string()))?;

        // Parse response and update nodes
        let descriptions = self.parse_response(response.text().unwrap_or_default());

        for node in graph.nodes.values_mut() {
            if let Some(desc) = descriptions.get(&node.id.0) {
                node.description = Some(desc.clone());
            }
        }

        Ok(())
    }

    /// Annotate all flows in a FlowIndex.
    pub async fn annotate_index(&self, index: &mut FlowIndex) -> crate::Result<()> {
        for flow in &mut index.flows {
            self.annotate(flow).await?;
        }
        Ok(())
    }

    fn build_prompt(&self, nodes: &[(NodeId, String, String)]) -> String {
        let mut prompt = String::from(
            "Describe each code node in 5-10 words. Be specific about WHAT it does, not HOW.\n\n"
        );

        for (id, code, kind) in nodes {
            prompt.push_str(&format!("NODE_{}: [{}] `{}`\n", id.0, kind, code));
        }

        prompt.push_str("\nRespond with:\nNODE_<id>: <description>\n");
        prompt
    }

    fn parse_response(&self, text: String) -> HashMap<u64, String> {
        let mut map = HashMap::new();

        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("NODE_") {
                if let Some((id_str, desc)) = rest.split_once(':') {
                    if let Ok(id) = id_str.trim().parse::<u64>() {
                        map.insert(id, desc.trim().to_string());
                    }
                }
            }
        }

        map
    }
}

const SYSTEM_PROMPT: &str = r#"You are a code documentation assistant. Your task is to write brief, human-readable descriptions of code nodes in a flow graph.

Rules:
- 5-10 words maximum per description
- Focus on WHAT the code does, not HOW
- Use plain English, avoid technical jargon
- Be specific: "Check if user is logged in" not "Condition check"
- For conditions: describe what's being checked
- For calls: describe the action being performed
- For returns: describe what's being returned

Examples:
- `if (user.isAdmin)` → "Check if user has admin privileges"
- `await db.users.findById(id)` → "Fetch user from database by ID"
- `return { success: true }` → "Return success response"
- `throw new NotFoundError()` → "Throw not found error"
"#;
