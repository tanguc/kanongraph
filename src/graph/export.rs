//! Graph export functionality.
//!
//! This module provides functions to export the dependency graph
//! in various formats for visualization and analysis.

use crate::error::Result;
use crate::graph::types::{DependencyGraph, EdgeType, GraphNode};
use crate::types::GraphFormat;
use serde::Serialize;

/// Export the dependency graph to the specified format.
///
/// # Supported Formats
///
/// - **DOT**: Graphviz DOT format for visualization
/// - **JSON**: Structured JSON for programmatic access
/// - **Mermaid**: Mermaid diagram syntax for documentation
///
/// # Example
///
/// ```rust,no_run
/// use kanongraph::graph::{export_graph, DependencyGraph};
/// use kanongraph::types::GraphFormat;
///
/// let graph = DependencyGraph::new();
/// let dot = export_graph(&graph, GraphFormat::Dot).unwrap();
/// println!("{}", dot);
/// ```
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn export_graph(graph: &DependencyGraph, format: GraphFormat) -> Result<String> {
    match format {
        GraphFormat::Dot => export_dot(graph),
        GraphFormat::Json => export_json(graph),
        GraphFormat::Mermaid => export_mermaid(graph),
    }
}

/// Export to Graphviz DOT format.
fn export_dot(graph: &DependencyGraph) -> Result<String> {
    let mut dot = String::new();
    dot.push_str("digraph KanonGraph {\n");
    dot.push_str("    rankdir=TB;\n");
    dot.push_str("    node [shape=box, style=rounded];\n");
    dot.push_str("    \n");

    // Add subgraphs for modules and providers
    dot.push_str("    subgraph cluster_modules {\n");
    dot.push_str("        label=\"Modules\";\n");
    dot.push_str("        style=dashed;\n");
    dot.push_str("        color=blue;\n");

    for node in graph.nodes() {
        if let GraphNode::Module(m) = node {
            let label = escape_dot_string(&format!(
                "{}\\n{}",
                m.name,
                m.version_constraint
                    .as_ref()
                    .map(|c| c.raw.as_str())
                    .unwrap_or("no constraint")
            ));
            let node_id = escape_dot_id(&m.id);
            dot.push_str(&format!(
                "        \"{node_id}\" [label=\"{label}\", fillcolor=lightblue, style=\"rounded,filled\"];\n"
            ));
        }
    }
    dot.push_str("    }\n\n");

    dot.push_str("    subgraph cluster_providers {\n");
    dot.push_str("        label=\"Providers\";\n");
    dot.push_str("        style=dashed;\n");
    dot.push_str("        color=green;\n");

    for node in graph.nodes() {
        if let GraphNode::Provider(p) = node {
            let label = escape_dot_string(&format!(
                "{}\\n{}",
                p.source,
                p.version_constraint
                    .as_ref()
                    .map(|c| c.raw.as_str())
                    .unwrap_or("no constraint")
            ));
            let node_id = escape_dot_id(&p.id);
            dot.push_str(&format!(
                "        \"{node_id}\" [label=\"{label}\", fillcolor=lightgreen, style=\"rounded,filled\", shape=ellipse];\n"
            ));
        }
    }
    dot.push_str("    }\n\n");

    // Add edges
    for (from, to, edge_type) in graph.edges() {
        let from_id = escape_dot_id(from.id());
        let to_id = escape_dot_id(to.id());
        let style = match edge_type {
            EdgeType::ModuleDependsOn => "style=solid, color=blue",
            EdgeType::ModuleRequiresProvider => "style=dashed, color=green",
            EdgeType::ProviderAlias => "style=dotted, color=gray",
            EdgeType::LocalModuleRef => "style=solid, color=orange",
        };
        dot.push_str(&format!(
            "    \"{from_id}\" -> \"{to_id}\" [{style}, label=\"{edge_type}\"];\n"
        ));
    }

    dot.push_str("}\n");
    Ok(dot)
}

/// Export to JSON format.
fn export_json(graph: &DependencyGraph) -> Result<String> {
    #[derive(Serialize)]
    struct JsonGraph {
        nodes: Vec<JsonNode>,
        edges: Vec<JsonEdge>,
        metadata: JsonMetadata,
    }

    #[derive(Serialize)]
    struct JsonNode {
        id: String,
        #[serde(rename = "type")]
        node_type: String,
        name: String,
        source: Option<String>,
        version_constraint: Option<String>,
        file_path: String,
        repository: Option<String>,
    }

    #[derive(Serialize)]
    struct JsonEdge {
        from: String,
        to: String,
        #[serde(rename = "type")]
        edge_type: String,
    }

    #[derive(Serialize)]
    struct JsonMetadata {
        total_nodes: usize,
        total_edges: usize,
        module_count: usize,
        provider_count: usize,
        runtime_count: usize,
    }

    let mut nodes = Vec::new();
    let mut module_count = 0;
    let mut provider_count = 0;
    let mut runtime_count = 0;
    for node in graph.nodes() {
        match node {
            GraphNode::Module(m) => {
                module_count += 1;
                nodes.push(JsonNode {
                    id: m.id.clone(),
                    node_type: "module".to_string(),
                    name: m.name.clone(),
                    source: Some(m.source.canonical_id()),
                    version_constraint: m.version_constraint.as_ref().map(|c| c.raw.clone()),
                    file_path: m.file_path.to_string_lossy().to_string(),
                    repository: m.repository.clone(),
                });
            }
            GraphNode::Provider(p) => {
                provider_count += 1;
                nodes.push(JsonNode {
                    id: p.id.clone(),
                    node_type: "provider".to_string(),
                    name: p.name.clone(),
                    source: Some(p.source.clone()),
                    version_constraint: p.version_constraint.as_ref().map(|c| c.raw.clone()),
                    file_path: p.file_path.to_string_lossy().to_string(),
                    repository: p.repository.clone(),
                });
            },
            GraphNode::Runtime(r) => {
                runtime_count += 1;
                nodes.push(JsonNode {
                    id: r.id.clone(),
                    node_type: "runtime".to_string(),
                    name: r.name.clone(),
                    source: None,
                    version_constraint: None,
                    file_path: "".to_string(),
                    repository: None,
                });
            }
        }
    }

    let edges: Vec<JsonEdge> = graph
        .edges()
        .map(|(from, to, edge_type)| JsonEdge {
            from: from.id().to_string(),
            to: to.id().to_string(),
            edge_type: edge_type.to_string(),
        })
        .collect();

    let json_graph = JsonGraph {
        metadata: JsonMetadata {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
            module_count,
            provider_count,
            runtime_count,
        },
        nodes,
        edges,
    };

    serde_json::to_string_pretty(&json_graph).map_err(|e| {
        crate::error::KanonGraphError::ReportGeneration {
            message: format!("Failed to serialize graph to JSON: {e}"),
        }
    })
}

/// Export to Mermaid diagram format.
fn export_mermaid(graph: &DependencyGraph) -> Result<String> {
    let mut mermaid = String::new();
    mermaid.push_str("graph TD\n");
    mermaid.push_str("    %% KanonGraph Dependency Graph\n\n");

    // Add node definitions
    for node in graph.nodes() {
        match node {
            GraphNode::Module(m) => {
                let id = sanitize_mermaid_id(&m.id);
                let label = escape_mermaid_string(&m.name);
                mermaid.push_str(&format!("    {id}[\"📦 {label}\"]\n"));
            }
            GraphNode::Provider(p) => {
                let id = sanitize_mermaid_id(&p.id);
                let label = escape_mermaid_string(&p.source);
                mermaid.push_str(&format!("    {id}((\"🔌 {label}\"))\n"));
            }
            GraphNode::Runtime(r) => {
                let id = sanitize_mermaid_id(&r.id);
                let label = escape_mermaid_string(&r.name);
                mermaid.push_str(&format!("    {id}[\"🚀 {label}\"]\n"));
            }
        }
    }

    mermaid.push('\n');

    // Add edges
    for (from, to, edge_type) in graph.edges() {
        let from_id = sanitize_mermaid_id(from.id());
        let to_id = sanitize_mermaid_id(to.id());
        let arrow = match edge_type {
            EdgeType::ModuleDependsOn => "-->",
            EdgeType::ModuleRequiresProvider => "-.->",
            EdgeType::ProviderAlias => "-.-",
            EdgeType::LocalModuleRef => "==>",
        };
        mermaid.push_str(&format!("    {from_id} {arrow} {to_id}\n"));
    }

    // Add styling
    mermaid.push_str("\n    %% Styling\n");
    mermaid.push_str("    classDef module fill:#e1f5fe,stroke:#01579b\n");
    mermaid.push_str("    classDef provider fill:#e8f5e9,stroke:#1b5e20\n");

    // Apply classes
    let module_ids: Vec<String> = graph
        .nodes()
        .filter_map(|n| {
            if n.is_module() {
                Some(sanitize_mermaid_id(n.id()))
            } else {
                None
            }
        })
        .collect();

    let provider_ids: Vec<String> = graph
        .nodes()
        .filter_map(|n| {
            if n.is_provider() {
                Some(sanitize_mermaid_id(n.id()))
            } else {
                None
            }
        })
        .collect();

    if !module_ids.is_empty() {
        mermaid.push_str(&format!("    class {} module\n", module_ids.join(",")));
    }
    if !provider_ids.is_empty() {
        mermaid.push_str(&format!("    class {} provider\n", provider_ids.join(",")));
    }

    Ok(mermaid)
}

/// Escape a string for use in DOT labels.
fn escape_dot_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Escape a string for use as a DOT node ID.
fn escape_dot_id(s: &str) -> String {
    s.replace(':', "_")
        .replace('/', "_")
        .replace('.', "_")
        .replace('-', "_")
}

/// Sanitize a string for use as a Mermaid node ID.
fn sanitize_mermaid_id(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Escape a string for use in Mermaid labels.
fn escape_mermaid_string(s: &str) -> String {
    s.replace('"', "'").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;
    use crate::VersionRange;
    use crate::graph::GraphBuilder;
    use crate::types::{Constraint, ModuleRef, ModuleSource, ProviderRef, RuntimeRef, RuntimeSource};
    use std::path::PathBuf;

    fn create_test_graph() -> DependencyGraph {
        let modules = vec![ModuleRef {
            name: "vpc".to_string(),
            source: ModuleSource::Registry {
                hostname: "registry.terraform.io".to_string(),
                namespace: "terraform-aws-modules".to_string(),
                name: "vpc".to_string(),
                provider: "aws".to_string(),
            },
            version_constraint: Some(Constraint::parse("~> 5.0").unwrap()),
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: Some("test".to_string()),
            attributes: Default::default(),
        }];

        let providers = vec![ProviderRef {
            name: "aws".to_string(),
            source: Some("hashicorp/aws".to_string()),
            version_constraint: Some(Constraint::parse(">= 4.0").unwrap()),
            file_path: PathBuf::from("versions.tf"),
            line_number: 1,
            repository: Some("test".to_string()),
        }];

        let runtimes = vec![RuntimeRef {
            name: "terraform".to_string(),
            version: Constraint::parse("1.0.0").unwrap(),
            source: RuntimeSource::Terraform,
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: Some("test".to_string()),
        }];

        GraphBuilder::new().build(&modules, &providers, &runtimes).unwrap()
    }

    #[test]
    fn test_export_dot() {
        let graph = create_test_graph();
        let dot = export_dot(&graph).unwrap();

        assert!(dot.contains("digraph KanonGraph"));
        assert!(dot.contains("vpc"));
        assert!(dot.contains("hashicorp/aws"));
    }

    #[test]
    fn test_export_json() {
        let graph = create_test_graph();
        let json = export_json(&graph).unwrap();

        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
        assert!(json.contains("\"metadata\""));

        // Parse to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["metadata"]["total_nodes"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_export_mermaid() {
        let graph = create_test_graph();
        let mermaid = export_mermaid(&graph).unwrap();

        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("📦")); // Module emoji
        assert!(mermaid.contains("🔌")); // Provider emoji
    }

    #[test]
    fn test_escape_dot_string() {
        assert_eq!(escape_dot_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_dot_string("say \"hi\""), "say \\\"hi\\\"");
    }

    #[test]
    fn test_sanitize_mermaid_id() {
        assert_eq!(
            sanitize_mermaid_id("module:test/vpc"),
            "module_test_vpc"
        );
    }
}

