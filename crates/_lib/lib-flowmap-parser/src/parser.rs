use crate::nestjs::NestJsExtractor;
use crate::typescript::TypeScriptExtractor;
use crate::Result;
use lib_flowmap_core::*;
use std::path::Path;
use walkdir::WalkDir;

/// Parsing mode for different framework types
#[derive(Debug, Clone, Copy, Default)]
pub enum ParseMode {
    /// Auto-detect: tries NestJS first, falls back to generic
    #[default]
    Auto,
    /// NestJS framework with decorators
    NestJs,
    /// Generic Express/Fastify style
    Generic,
}

pub struct FlowParser {
    ts_extractor: TypeScriptExtractor,
    nestjs_extractor: NestJsExtractor,
    mode: ParseMode,
}

impl FlowParser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            ts_extractor: TypeScriptExtractor::new()?,
            nestjs_extractor: NestJsExtractor::new()?,
            mode: ParseMode::Auto,
        })
    }

    pub fn with_mode(mut self, mode: ParseMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn parse_directory(&mut self, root: &Path) -> Result<FlowIndex> {
        let mut index = FlowIndex::new(root.to_string_lossy().as_ref());

        // Collect all TypeScript files
        let ts_files: Vec<_> = WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                let path = e.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let path_str = path.to_string_lossy();

                // Skip common non-source directories
                !path_str.contains("node_modules")
                    && !path_str.contains(".git")
                    && !path_str.contains("dist")
                    && !path_str.contains(".next")
                    && !path_str.contains("coverage")
                    && !path_str.contains(".turbo")
                    && matches!(ext, "ts" | "tsx" | "js" | "jsx")
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        // Detect mode if auto
        let mode = match self.mode {
            ParseMode::Auto => self.detect_mode(&ts_files, root),
            other => other,
        };

        tracing::info!("Using parse mode: {:?} for {}", mode, root.display());

        match mode {
            ParseMode::NestJs => {
                self.parse_nestjs(&ts_files, root, &mut index)?;
            }
            ParseMode::Generic | ParseMode::Auto => {
                self.parse_generic(&ts_files, root, &mut index)?;
            }
        }

        // Calculate positions for visualization
        self.calculate_positions(&mut index);

        tracing::info!(
            "Parsed {} flows from {}",
            index.flows.len(),
            root.display()
        );

        Ok(index)
    }

    fn detect_mode(&self, files: &[std::path::PathBuf], root: &Path) -> ParseMode {
        // Check for NestJS indicators
        for path in files {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Look for NestJS decorators
                if content.contains("@Controller")
                    || content.contains("@Injectable")
                    || content.contains("@Module")
                {
                    return ParseMode::NestJs;
                }
            }
        }

        // Check for package.json
        let pkg_json = root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(pkg_json) {
            if content.contains("@nestjs/") {
                return ParseMode::NestJs;
            }
        }

        ParseMode::Generic
    }

    fn parse_nestjs(
        &mut self,
        files: &[std::path::PathBuf],
        root: &Path,
        index: &mut FlowIndex,
    ) -> Result<()> {
        // Recreate extractor to reset state
        self.nestjs_extractor = NestJsExtractor::new()?;

        // Pass 1: Index all files
        let mut source_files = Vec::new();
        for path in files {
            let source = std::fs::read_to_string(path)?;
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            self.nestjs_extractor.index_file(&source, &relative_path)?;
            source_files.push((relative_path, source));
        }

        // Log symbol index stats
        let symbol_index = self.nestjs_extractor.symbol_index();
        tracing::info!(
            "Symbol index: {} classes, {} endpoints",
            symbol_index.classes.len(),
            symbol_index.http_endpoints().len()
        );

        // Pass 2: Build flows
        let flows = self.nestjs_extractor.build_flows(&source_files)?;
        index.flows.extend(flows);

        Ok(())
    }

    fn parse_generic(
        &mut self,
        files: &[std::path::PathBuf],
        root: &Path,
        index: &mut FlowIndex,
    ) -> Result<()> {
        for path in files {
            if let Ok(flows) = self.parse_typescript_file(path, root) {
                index.flows.extend(flows);
            }
        }
        Ok(())
    }

    fn parse_typescript_file(&mut self, path: &Path, root: &Path) -> Result<Vec<FlowGraph>> {
        let source = std::fs::read_to_string(path)?;
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        self.ts_extractor.parse_file(&source, &relative_path)
    }

    fn calculate_positions(&self, index: &mut FlowIndex) {
        for flow in &mut index.flows {
            self.layout_flow(flow);
        }
    }

    fn layout_flow(&self, flow: &mut FlowGraph) {
        // Simple top-to-bottom layout
        let mut visited = std::collections::HashSet::new();
        let mut levels: std::collections::HashMap<NodeId, usize> = std::collections::HashMap::new();

        // Find entry node
        let entry = flow
            .nodes
            .values()
            .find(|n| n.inputs.is_empty())
            .map(|n| n.id);

        if let Some(entry_id) = entry {
            self.assign_levels(flow, entry_id, 0, &mut levels, &mut visited);
        }

        // Group nodes by level
        let mut level_nodes: std::collections::HashMap<usize, Vec<NodeId>> =
            std::collections::HashMap::new();
        for (&node_id, &level) in &levels {
            level_nodes.entry(level).or_default().push(node_id);
        }

        // Assign positions
        let x_spacing = 250.0;
        let y_spacing = 120.0;

        for (level, nodes) in level_nodes {
            let y = level as f64 * y_spacing + 50.0;
            let total_width = (nodes.len() - 1) as f64 * x_spacing;
            let start_x = -total_width / 2.0 + 400.0;

            for (i, node_id) in nodes.iter().enumerate() {
                if let Some(node) = flow.nodes.get_mut(node_id) {
                    let x = start_x + i as f64 * x_spacing;
                    node.position = Some(NodePosition { x, y });
                }
            }
        }
    }

    fn assign_levels(
        &self,
        flow: &FlowGraph,
        node_id: NodeId,
        level: usize,
        levels: &mut std::collections::HashMap<NodeId, usize>,
        visited: &mut std::collections::HashSet<NodeId>,
    ) {
        if visited.contains(&node_id) {
            return;
        }
        visited.insert(node_id);

        let existing = levels.get(&node_id).copied().unwrap_or(0);
        levels.insert(node_id, existing.max(level));

        // Find connected nodes
        for edge in &flow.edges {
            if edge.from_node == node_id {
                self.assign_levels(flow, edge.to_node, level + 1, levels, visited);
            }
        }
    }
}

impl Default for FlowParser {
    fn default() -> Self {
        Self::new().expect("Failed to create FlowParser")
    }
}
