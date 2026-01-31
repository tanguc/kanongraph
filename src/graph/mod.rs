//! Dependency Graph Module
//!
//! This module implements a directed graph data structure for representing
//! and analyzing dependencies between Terraform modules and providers.
//!
//! # Architecture Overview
//!
//! The dependency graph uses the `petgraph` library as its foundation,
//! providing efficient graph operations and algorithms. The graph is
//! structured as follows:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     DEPENDENCY GRAPH                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌──────────┐         ┌──────────┐         ┌──────────┐        │
//! │  │  Module  │────────▶│  Module  │────────▶│ Provider │        │
//! │  │  "vpc"   │ depends │  "subnet"│ requires│  "aws"   │        │
//! │  └──────────┘   on    └──────────┘         └──────────┘        │
//! │       │                    │                    ▲               │
//! │       │                    │                    │               │
//! │       ▼                    ▼                    │               │
//! │  ┌──────────┐         ┌──────────┐             │               │
//! │  │ Provider │         │ Provider │─────────────┘               │
//! │  │  "aws"   │         │  "aws"   │                             │
//! │  └──────────┘         └──────────┘                             │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Node Types
//!
//! The graph contains two types of nodes:
//!
//! 1. **Module Nodes**: Represent Terraform module blocks
//!    - Store module name, source, version constraint
//!    - Track file location and repository
//!
//! 2. **Provider Nodes**: Represent required providers
//!    - Store provider name, source, version constraint
//!    - Track where the requirement is defined
//!
//! # Edge Types
//!
//! Edges represent relationships between nodes:
//!
//! 1. **ModuleDependsOn**: Module A depends on Module B
//!    - Created when a module references another module's outputs
//!
//! 2. **ModuleRequiresProvider**: Module requires a specific provider
//!    - Created when a module uses resources from a provider
//!
//! 3. **ProviderAlias**: Provider A is an alias of Provider B
//!    - Created for provider aliases
//!
//! # Graph Operations
//!
//! The graph supports several key operations:
//!
//! ## Building the Graph
//!
//! ```rust,no_run
//! use kanongraph::graph::GraphBuilder;
//! use kanongraph::types::{ModuleRef, ProviderRef, RuntimeRef};
//!
//! let builder = GraphBuilder::new();
//! let modules: Vec<ModuleRef> = vec![/* ... */];
//! let providers: Vec<ProviderRef> = vec![/* ... */];
//! let runtimes: Vec<RuntimeRef> = vec![/* ... */];
//!
//! let graph = builder.build(&modules, &providers, &runtimes).unwrap();
//! ```
//!
//! ## Querying Dependencies
//!
//! ```rust,ignore
//! use kanongraph::graph::DependencyGraph;
//! let graph = DependencyGraph::new();
//! // Get all modules that depend on a specific module
//! let dependents = graph.get_dependents("vpc-module-id");
//!
//! // Get all dependencies of a module
//! let dependencies = graph.get_dependencies("app-module-id");
//!
//! // Find all modules using a specific provider
//! let aws_modules = graph.modules_using_provider("hashicorp/aws");
//! ```
//!
//! ## Detecting Cycles
//!
//! ```rust,ignore
//! use kanongraph::graph::DependencyGraph;
//! let graph = DependencyGraph::new();
//! if let Some(cycle) = graph.find_cycle() {
//!     println!("Circular dependency detected: {:?}", cycle);
//! }
//! ```
//!
//! # Data Flow
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  HCL Files  │────▶│   Parser    │────▶│  ModuleRef  │
//! │  (.tf)      │     │             │     │ ProviderRef │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!                                                │
//!                                                ▼
//!                                         ┌─────────────┐
//!                                         │ GraphBuilder│
//!                                         └─────────────┘
//!                                                │
//!                                                ▼
//!                                         ┌─────────────┐
//!                                         │ Dependency  │
//!                                         │   Graph     │
//!                                         └─────────────┘
//!                                                │
//!                           ┌────────────────────┼────────────────────┐
//!                           ▼                    ▼                    ▼
//!                    ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
//!                    │  Analyzer   │      │  Exporter   │      │  Reporter   │
//!                    │ (conflicts) │      │ (DOT/JSON)  │      │ (reports)   │
//!                    └─────────────┘      └─────────────┘      └─────────────┘
//! ```
//!
//! # Implementation Details
//!
//! ## Petgraph Integration
//!
//! We use `petgraph::Graph<GraphNode, EdgeType, Directed>` where:
//! - `GraphNode` is an enum containing either a Module or Provider
//! - `EdgeType` describes the relationship type
//! - The graph is directed (dependencies flow in one direction)
//!
//! ## Node Indexing
//!
//! Nodes are indexed in two ways:
//! 1. **Petgraph NodeIndex**: Internal graph index for traversal
//! 2. **Canonical ID**: String identifier for lookups (e.g., "hashicorp/vpc/aws")
//!
//! A `HashMap<String, NodeIndex>` provides O(1) lookup by canonical ID.
//!
//! ## Memory Efficiency
//!
//! - Nodes store references to original data where possible
//! - Large strings (file paths, sources) are interned
//! - The graph structure itself is compact (petgraph uses adjacency lists)
//!
//! # Example: Complete Workflow
//!
//! ```rust,no_run
//! use kanongraph::graph::{GraphBuilder, DependencyGraph, export_graph};
//! use kanongraph::types::{GraphFormat, ModuleRef, ProviderRef};
//!
//! // 1. Parse HCL files to get modules and providers
//! let modules: Vec<ModuleRef> = vec![/* from parser */];
//! let providers: Vec<ProviderRef> = vec![/* from parser */];
//! let runtimes: Vec<kanongraph::types::RuntimeRef> = vec![/* from parser */];
//!
//! // 2. Build the dependency graph
//! let builder = GraphBuilder::new();
//! let graph = builder.build(&modules, &providers, &runtimes).unwrap();
//!
//! // 3. Query the graph
//! println!("Total nodes: {}", graph.node_count());
//! println!("Total edges: {}", graph.edge_count());
//!
//! // 4. Export for visualization
//! let dot_output = export_graph(&graph, GraphFormat::Dot).unwrap();
//! std::fs::write("dependencies.dot", dot_output).unwrap();
//!
//! // 5. Render with Graphviz: dot -Tpng dependencies.dot -o dependencies.png
//! ```

mod builder;
mod export;
mod types;

pub use builder::GraphBuilder;
pub use export::export_graph;
pub use types::{DependencyGraph, EdgeType, GraphNode, NodeId};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Constraint, GraphFormat, ModuleRef, ModuleSource, ProviderRef};
    use std::path::PathBuf;

    fn create_test_module(name: &str, source: &str) -> ModuleRef {
        ModuleRef {
            name: name.to_string(),
            source: ModuleSource::Registry {
                hostname: "registry.terraform.io".to_string(),
                namespace: "terraform-aws-modules".to_string(),
                name: source.to_string(),
                provider: "aws".to_string(),
            },
            version_constraint: Some(Constraint::parse("~> 5.0").unwrap()),
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: Some("test-repo".to_string()),
            attributes: Default::default(),
        }
    }

    fn create_test_provider(name: &str, source: &str) -> ProviderRef {
        ProviderRef {
            name: name.to_string(),
            source: Some(source.to_string()),
            version_constraint: Some(Constraint::parse(">= 4.0").unwrap()),
            file_path: PathBuf::from("versions.tf"),
            line_number: 1,
            repository: Some("test-repo".to_string()),
        }
    }

    #[test]
    fn test_build_simple_graph() {
        let modules = vec![
            create_test_module("vpc", "vpc"),
            create_test_module("eks", "eks"),
        ];
        let providers = vec![create_test_provider("aws", "hashicorp/aws")];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &providers, &[]).unwrap();

        // Should have 3 nodes (2 modules + 1 provider)
        assert_eq!(graph.node_count(), 3);
    }

    #[test]
    fn test_graph_export_dot() {
        let modules = vec![create_test_module("vpc", "vpc")];
        let providers = vec![create_test_provider("aws", "hashicorp/aws")];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &providers, &[]).unwrap();

        let dot = export_graph(&graph, GraphFormat::Dot).unwrap();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("vpc"));
    }

    #[test]
    fn test_graph_export_json() {
        let modules = vec![create_test_module("vpc", "vpc")];
        let providers = vec![create_test_provider("aws", "hashicorp/aws")];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &providers, &[]).unwrap();

        let json = export_graph(&graph, GraphFormat::Json).unwrap();
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
    }

    #[test]
    fn test_graph_export_mermaid() {
        let modules = vec![create_test_module("vpc", "vpc")];
        let providers = vec![create_test_provider("aws", "hashicorp/aws")];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &providers, &[]).unwrap();

        let mermaid = export_graph(&graph, GraphFormat::Mermaid).unwrap();
        assert!(mermaid.contains("graph"));
    }
}

