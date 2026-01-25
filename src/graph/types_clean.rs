//! Graph type definitions.
//!
//! This module defines the core types used in the dependency graph:
//! - `DependencyGraph`: The main graph structure
//! - `GraphNode`: Nodes in the graph (modules or providers)
//! - `EdgeType`: Relationships between nodes
//! - `NodeId`: Unique identifier for nodes

use crate::VersionRange;
use crate::types::{Constraint, ModuleRef, ModuleSource, ProviderRef, RuntimeRef};
use crate::vcs::VcsIdentifier;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unique identifier for a node in the graph.
///
/// This is a string-based identifier that uniquely identifies a module
/// or provider across all repositories being analyzed.
pub type NodeId = String;

/// The dependency graph structure.
///
/// This is the central data structure for representing relationships
/// between Terraform modules and providers. It wraps a petgraph directed
/// graph and provides domain-specific operations.
///
/// # Structure
///
/// ```text
/// DependencyGraph
/// ├── inner: DiGraph<GraphNode, EdgeType>  // The actual graph
/// ├── node_index: HashMap<NodeId, NodeIndex>  // Fast lookup by ID
/// ├── modules: HashMap<NodeId, NodeIndex>  // Module nodes only
/// └── providers: HashMap<NodeId, NodeIndex>  // Provider nodes only
/// └── runtimes: HashMap<NodeId, NodeIndex>  // Runtime nodes only
/// └── vcs_metadata: HashMap<NodeId, VcsIdentifier>  // VCS markers
/// ```
///
/// # Thread Safety
///
/// The graph is not thread-safe by default. For concurrent access,
/// wrap it in `Arc<RwLock<DependencyGraph>>`.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// The underlying petgraph directed graph
    inner: DiGraph<GraphNode, EdgeType>,

    /// Index from canonical node ID to petgraph NodeIndex
    node_index: HashMap<NodeId, NodeIndex>,

    /// Index of module nodes only
    modules: HashMap<NodeId, NodeIndex>,

    /// Index of provider nodes only
    providers: HashMap<NodeId, NodeIndex>,

    /// Index of runtime nodes only
    runtimes: HashMap<NodeId, NodeIndex>,

    /// VCS metadata for nodes that came from repositories
    vcs_metadata: HashMap<NodeId, VcsIdentifier>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    /// Create a new empty dependency graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: DiGraph::new(),
            node_index: HashMap::new(),
            modules: HashMap::new(),
            providers: HashMap::new(),
            runtimes: HashMap::new(),
            vcs_metadata: HashMap::new(),
        }
    }

    /// Set VCS metadata for a node
    pub fn set_vcs_metadata(&mut self, node_id: &NodeId, vcs_id: VcsIdentifier) {
        self.vcs_metadata.insert(node_id.clone(), vcs_id);
    }

    /// Get VCS metadata for a node
    #[must_use]
    pub fn get_vcs_metadata(&self, node_id: &NodeId) -> Option<&VcsIdentifier> {
        self.vcs_metadata.get(node_id)
    }

    /// Get all nodes with VCS metadata
    #[must_use]
    pub fn vcs_nodes(&self) -> &HashMap<NodeId, VcsIdentifier> {
        &self.vcs_metadata
    }
}
