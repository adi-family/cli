use adi_knowledgebase_core::{EdgeType, KnowledgeSource, Knowledgebase, NodeType};
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use lib_cli_common::setup_logging;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "kb")]
#[command(about = "ADI Knowledgebase CLI - Knowledge management with graph + embeddings")]
#[command(version)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Data directory
    #[arg(short, long)]
    data_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add knowledge - must include exact user statement
    Add {
        /// Exact user statement (verbatim)
        #[arg(long)]
        user_said: String,

        /// Derived knowledge interpretation
        knowledge: String,

        /// Node type
        #[arg(short = 't', long, default_value = "fact")]
        node_type: NodeTypeArg,
    },

    /// Query the knowledgebase
    Query {
        /// Search query
        question: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "5")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Approve a node (set confidence to 1.0)
    Approve {
        /// Node ID
        node_id: Uuid,
    },

    /// Request clarification for a node
    Clarify {
        /// Node ID
        node_id: Uuid,
    },

    /// Show conflicts
    Conflicts,

    /// Ask user a question (system-initiated)
    Ask {
        /// Question to ask
        question: String,
    },

    /// Show node details
    Show {
        /// Node ID
        node_id: Uuid,
    },

    /// Delete a node
    Delete {
        /// Node ID
        node_id: Uuid,
    },

    /// Link two nodes
    Link {
        /// Source node ID
        from: Uuid,

        /// Target node ID
        to: Uuid,

        /// Edge type
        #[arg(short = 't', long, default_value = "related-to")]
        edge_type: EdgeTypeArg,

        /// Edge weight (0.0-1.0)
        #[arg(short, long, default_value = "0.5")]
        weight: f32,
    },

    /// Show orphan nodes (no edges)
    Orphans,

    /// Show status
    Status,
}

#[derive(Clone, ValueEnum)]
enum NodeTypeArg {
    Decision,
    Fact,
    Error,
    Guide,
    Glossary,
    Context,
    Assumption,
}

impl From<NodeTypeArg> for NodeType {
    fn from(arg: NodeTypeArg) -> Self {
        match arg {
            NodeTypeArg::Decision => NodeType::Decision,
            NodeTypeArg::Fact => NodeType::Fact,
            NodeTypeArg::Error => NodeType::Error,
            NodeTypeArg::Guide => NodeType::Guide,
            NodeTypeArg::Glossary => NodeType::Glossary,
            NodeTypeArg::Context => NodeType::Context,
            NodeTypeArg::Assumption => NodeType::Assumption,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum EdgeTypeArg {
    Supersedes,
    Contradicts,
    Requires,
    #[value(name = "related-to")]
    RelatedTo,
    #[value(name = "derived-from")]
    DerivedFrom,
    Answers,
}

impl From<EdgeTypeArg> for EdgeType {
    fn from(arg: EdgeTypeArg) -> Self {
        match arg {
            EdgeTypeArg::Supersedes => EdgeType::Supersedes,
            EdgeTypeArg::Contradicts => EdgeType::Contradicts,
            EdgeTypeArg::Requires => EdgeType::Requires,
            EdgeTypeArg::RelatedTo => EdgeType::RelatedTo,
            EdgeTypeArg::DerivedFrom => EdgeType::DerivedFrom,
            EdgeTypeArg::Answers => EdgeType::Answers,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    let data_dir = cli
        .data_dir
        .unwrap_or_else(adi_knowledgebase_core::default_data_dir);

    let kb = Knowledgebase::open(&data_dir).await?;

    match cli.command {
        Commands::Add {
            user_said,
            knowledge,
            node_type,
        } => {
            let node = kb
                .add_from_user(&user_said, &knowledge, node_type.into())
                .await?;
            println!("Added node: {}", node.id);
            println!("  Type: {:?}", node.node_type);
            println!("  Title: {}", node.title);
            println!("  Confidence: {:.2}", node.confidence.0);
        }

        Commands::Query {
            question,
            limit,
            format,
        } => {
            let results = kb.query(&question).await?;
            let results: Vec<_> = results.into_iter().take(limit).collect();

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
                OutputFormat::Text => {
                    if results.is_empty() {
                        println!("No results found.");
                    } else {
                        for (i, result) in results.iter().enumerate() {
                            println!(
                                "\n{}. {} (score: {:.2})",
                                i + 1,
                                result.node.id,
                                result.score
                            );
                            println!("   Type: {:?}", result.node.node_type);
                            println!("   Title: {}", result.node.title);
                            println!("   Confidence: {:.2}", result.node.confidence.0);
                            if !result.node.content.is_empty() {
                                let content = if result.node.content.len() > 200 {
                                    format!("{}...", &result.node.content[..200])
                                } else {
                                    result.node.content.clone()
                                };
                                println!("   Content: {}", content);
                            }
                        }
                    }
                }
            }
        }

        Commands::Approve { node_id } => {
            kb.approve(node_id)?;
            println!("Approved node: {}", node_id);
        }

        Commands::Clarify { node_id } => {
            if let Some(node) = kb.get_node(node_id)? {
                println!("Clarification requested for node: {}", node_id);
                println!("  Type: {:?}", node.node_type);
                println!("  Title: {}", node.title);
                println!("  Content: {}", node.content);
                println!("\nPlease provide clarification:");
                // In a real implementation, this would create a ClarificationRequest
            } else {
                println!("Node not found: {}", node_id);
            }
        }

        Commands::Conflicts => {
            let conflicts = kb.get_conflicts()?;
            if conflicts.is_empty() {
                println!("No conflicts found.");
            } else {
                println!("Found {} conflicts:\n", conflicts.len());
                for (a, b) in conflicts {
                    println!("Conflict:");
                    println!("  A: {} - {}", a.id, a.title);
                    println!("  B: {} - {}", b.id, b.title);
                    println!();
                }
            }
        }

        Commands::Ask { question } => {
            println!("Question: {}", question);
            println!("\nPlease provide your answer:");
            // In a real implementation, this would wait for user input
        }

        Commands::Show { node_id } => {
            if let Some(node) = kb.get_node(node_id)? {
                println!("Node: {}", node.id);
                println!("  Type: {:?}", node.node_type);
                println!("  Title: {}", node.title);
                println!("  Content: {}", node.content);
                println!("  Confidence: {:.2}", node.confidence.0);
                println!("  Created: {}", node.created_at);
                println!("  Updated: {}", node.updated_at);
                println!("  Last accessed: {}", node.last_accessed_at);
                match &node.source {
                    KnowledgeSource::User { statement } => {
                        println!("  Source: User said: \"{}\"", statement);
                    }
                    KnowledgeSource::Derived {
                        interpretation,
                        source_id,
                    } => {
                        println!("  Source: Derived - {}", interpretation);
                        if let Some(id) = source_id {
                            println!("    From: {}", id);
                        }
                    }
                }
            } else {
                println!("Node not found: {}", node_id);
            }
        }

        Commands::Delete { node_id } => {
            kb.delete_node(node_id)?;
            println!("Deleted node: {}", node_id);
        }

        Commands::Link {
            from,
            to,
            edge_type,
            weight,
        } => {
            let edge = kb.add_edge(from, to, edge_type.into(), weight)?;
            println!("Created edge: {}", edge.id);
            println!("  From: {}", from);
            println!("  To: {}", to);
            println!("  Type: {:?}", edge.edge_type);
            println!("  Weight: {:.2}", edge.weight);
        }

        Commands::Orphans => {
            let orphans = kb.get_orphans()?;
            if orphans.is_empty() {
                println!("No orphan nodes found.");
            } else {
                println!("Found {} orphan nodes:\n", orphans.len());
                for node in orphans {
                    println!("  {} - {} ({:?})", node.id, node.title, node.node_type);
                }
            }
        }

        Commands::Status => {
            println!("Knowledgebase Status");
            println!("  Data directory: {}", kb.data_dir().display());
            println!("  Embeddings: {}", kb.storage().embedding.count());
        }
    }

    Ok(())
}
