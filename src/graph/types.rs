//! Graph type definitions.
//!
//! This module defines the core types used in the dependency graph:
//! - `DependencyGraph`: The main graph structure
//! - `GraphNode`: Nodes in the graph (modules or providers)
//! - `EdgeType`: Relationships between nodes
//! - `NodeId`: Unique identifier for nodes

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

    /// Add a module node to the graph.
    ///
    /// Returns the node ID if the module was added, or the existing ID
    /// if a module with the same canonical ID already exists.
    pub fn add_module(&mut self, module: &ModuleRef) -> NodeId {
        let node_id = self.module_node_id(module);

        if let Some(&_existing) = self.node_index.get(&node_id) {
            return node_id;
        }

        let node = GraphNode::Module(ModuleNode {
            id: node_id.clone(),
            name: module.name.clone(),
            source: module.source.clone(),
            version_constraint: module.version_constraint.clone(),
            file_path: module.file_path.clone(),
            line_number: module.line_number,
            repository: module.repository.clone(),
        });

        let idx = self.inner.add_node(node);
        self.node_index.insert(node_id.clone(), idx);
        self.modules.insert(node_id.clone(), idx);

        node_id
    }

    /// Add a provider node to the graph.
    ///
    /// Returns the node ID if the provider was added, or the existing ID
    /// if a provider with the same canonical ID already exists.
    pub fn add_provider(&mut self, provider: &ProviderRef) -> NodeId {
        let node_id = self.provider_node_id(provider);

        if let Some(&_existing) = self.node_index.get(&node_id) {
            return node_id;
        }

        let node = GraphNode::Provider(ProviderNode {
            id: node_id.clone(),
            name: provider.name.clone(),
            source: provider.qualified_source(),
            version_constraint: provider.version_constraint.clone(),
            file_path: provider.file_path.clone(),
            line_number: provider.line_number,
            repository: provider.repository.clone(),
        });

        let idx = self.inner.add_node(node);
        self.node_index.insert(node_id.clone(), idx);
        self.providers.insert(node_id.clone(), idx);

        node_id
    }

    /// Add a runtime node to the graph.
    ///
    /// Returns the node ID if the runtime was added, or the existing ID
    /// if a runtime with the same name already exists.
    pub fn add_runtime(&mut self, runtime: &RuntimeRef) -> NodeId {
        let node_id = format!("runtime:{}", runtime.name);

        let node = GraphNode::Runtime(RuntimeNode {
            id: node_id.clone(),
            name: runtime.name.clone(),
            version: runtime.version.clone(),
            source: runtime.source.to_string(),
            file_path: runtime.file_path.clone(),
            line_number: runtime.line_number,
            repository: runtime.repository.clone(),
        });

        let idx = self.inner.add_node(node);
        self.node_index.insert(node_id.clone(), idx);
        self.runtimes.insert(node_id.clone(), idx);

        node_id
    }

    /// Add an edge between two nodes.
    ///
    /// Returns true if the edge was added, false if it already exists
    /// or if either node doesn't exist.
    pub fn add_edge(&mut self, from: &NodeId, to: &NodeId, edge_type: EdgeType) -> bool {
        let from_idx = match self.node_index.get(from) {
            Some(&idx) => idx,
            None => return false,
        };
        let to_idx = match self.node_index.get(to) {
            Some(&idx) => idx,
            None => return false,
        };

        // Check if edge already exists
        if self.inner.find_edge(from_idx, to_idx).is_some() {
            return false;
        }

        self.inner.add_edge(from_idx, to_idx, edge_type);
        true
    }

    /// Get a node by its ID.
    #[must_use]
    pub fn get_node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.node_index.get(id).map(|&idx| &self.inner[idx])
    }

    /// Get the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Get the number of edges in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Get all module node IDs.
    #[must_use]
    pub fn module_ids(&self) -> Vec<&NodeId> {
        self.modules.keys().collect()
    }

    /// Get all provider node IDs.
    #[must_use]
    pub fn provider_ids(&self) -> Vec<&NodeId> {
        self.providers.keys().collect()
    }

    /// Get all nodes that depend on the given node (incoming edges).
    #[must_use]
    pub fn get_dependents(&self, id: &NodeId) -> Vec<&GraphNode> {
        let idx = match self.node_index.get(id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        self.inner
            .neighbors_directed(idx, petgraph::Direction::Incoming)
            .map(|neighbor_idx| &self.inner[neighbor_idx])
            .collect()
    }

    /// Get all nodes that the given node depends on (outgoing edges).
    #[must_use]
    pub fn get_dependencies(&self, id: &NodeId) -> Vec<&GraphNode> {
        let idx = match self.node_index.get(id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        self.inner
            .neighbors_directed(idx, petgraph::Direction::Outgoing)
            .map(|neighbor_idx| &self.inner[neighbor_idx])
            .collect()
    }

    /// Get all modules that use a specific provider.
    #[must_use]
    pub fn modules_using_provider(&self, provider_source: &str) -> Vec<&ModuleNode> {
        // Find the provider node
        let provider_idx = self.providers.iter().find_map(|(_id, &idx)| {
            if let GraphNode::Provider(p) = &self.inner[idx] {
                if p.source == provider_source {
                    return Some(idx);
                }
            }
            None
        });

        let provider_idx = match provider_idx {
            Some(idx) => idx,
            None => return Vec::new(),
        };

        // Get all incoming edges (modules that require this provider)
        self.inner
            .neighbors_directed(provider_idx, petgraph::Direction::Incoming)
            .filter_map(|neighbor_idx| {
                if let GraphNode::Module(m) = &self.inner[neighbor_idx] {
                    Some(m)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Merge another graph into this one.
    pub fn merge(&mut self, other: Self) {
        // Add all nodes from the other graph
        for node in other.inner.node_weights() {
            match node {
                GraphNode::Module(m) => {
                    if !self.node_index.contains_key(&m.id) {
                        let idx = self.inner.add_node(node.clone());
                        self.node_index.insert(m.id.clone(), idx);
                        self.modules.insert(m.id.clone(), idx);
                    }
                }
                GraphNode::Provider(p) => {
                    if !self.node_index.contains_key(&p.id) {
                        let idx = self.inner.add_node(node.clone());
                        self.node_index.insert(p.id.clone(), idx);
                        self.providers.insert(p.id.clone(), idx);
                    }
                }
                GraphNode::Runtime(r) => {
                    if !self.node_index.contains_key(&r.id) {
                        let idx = self.inner.add_node(node.clone());
                        self.node_index.insert(r.id.clone(), idx);
                        self.runtimes.insert(r.id.clone(), idx);
                    }
                }
            }
        }

        // Add all edges from the other graph
        for edge in other.inner.edge_references() {
            let from_node = &other.inner[edge.source()];
            let to_node = &other.inner[edge.target()];
            let from_id = from_node.id();
            let to_id = to_node.id();

            if let (Some(&from_idx), Some(&to_idx)) =
                (self.node_index.get(from_id), self.node_index.get(to_id))
            {
                if self.inner.find_edge(from_idx, to_idx).is_none() {
                    self.inner.add_edge(from_idx, to_idx, edge.weight().clone());
                }
            }
        }

        // Merge VCS metadata
        for (node_id, vcs_id) in other.vcs_metadata {
            self.vcs_metadata.entry(node_id).or_insert(vcs_id);
        }
    }

    /// Get an iterator over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.inner.node_weights()
    }

    /// Get an iterator over all edges.
    pub fn edges(&self) -> impl Iterator<Item = (&GraphNode, &GraphNode, &EdgeType)> {
        self.inner.edge_references().map(|edge| {
            (
                &self.inner[edge.source()],
                &self.inner[edge.target()],
                edge.weight(),
            )
        })
    }

    /// Get the underlying petgraph for advanced operations.
    #[must_use]
    pub fn inner(&self) -> &DiGraph<GraphNode, EdgeType> {
        &self.inner
    }

    /// Generate a canonical node ID for a module.
    fn module_node_id(&self, module: &ModuleRef) -> NodeId {
        let source_id = module.source.canonical_id();
        let repo = module.repository.as_deref().unwrap_or("local");
        format!("module:{repo}:{source_id}:{}", module.name)
    }

    /// Generate a canonical node ID for a provider.
    fn provider_node_id(&self, provider: &ProviderRef) -> NodeId {
        let source = provider.qualified_source();
        let repo = provider.repository.as_deref().unwrap_or("local");
        format!("provider:{repo}:{source}")
    }
}

/// A node in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GraphNode {
    /// A Terraform module
    Module(ModuleNode),
    /// A Terraform provider
    Provider(ProviderNode),
    /// A Terraform runtime
    Runtime(RuntimeNode),
}

impl GraphNode {
    /// Get the node's unique ID.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Module(m) => &m.id,
            Self::Provider(p) => &p.id,
            Self::Runtime(r) => &r.id,
        }
    }

    /// Get the node's display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        match self {
            Self::Module(m) => &m.name,
            Self::Provider(p) => &p.name,
            Self::Runtime(r) => &r.name,
        }
    }

    /// Check if this is a module node.
    #[must_use]
    pub const fn is_module(&self) -> bool {
        matches!(self, Self::Module(_))
    }

    /// Check if this is a provider node.
    #[must_use]
    pub const fn is_provider(&self) -> bool {
        matches!(self, Self::Provider(_))
    }

    #[must_use]
    pub const fn is_runtime(&self) -> bool {
        matches!(self, Self::Runtime(_))
    }
}

/// A module node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleNode {
    /// Unique identifier
    pub id: NodeId,
    /// Module name (from the module block label)
    pub name: String,
    /// Module source
    pub source: ModuleSource,
    /// Version constraint
    pub version_constraint: Option<Constraint>,
    /// File where defined
    pub file_path: PathBuf,
    /// Line number
    pub line_number: usize,
    /// Repository name
    pub repository: Option<String>,
}

/// A provider node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderNode {
    /// Unique identifier
    pub id: NodeId,
    /// Provider local name
    pub name: String,
    /// Provider source (e.g., "hashicorp/aws")
    pub source: String,
    /// Version constraint
    pub version_constraint: Option<Constraint>,
    /// File where defined
    pub file_path: PathBuf,
    /// Line number
    pub line_number: usize,
    /// Repository name
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeNode {
    /// Unique identifier
    pub id: NodeId,
    /// Runtime name
    pub name: String,
    /// Runtime version
    pub version: Constraint,
    /// Provider source (e.g., "hashicorp/aws")
    pub source: String,
    /// File where defined
    pub file_path: PathBuf,
    /// Line number
    pub line_number: usize,
    /// Repository name
    pub repository: Option<String>,
}

/// Type of edge in the dependency graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Module depends on another module
    ModuleDependsOn,
    /// Module requires a provider
    ModuleRequiresProvider,
    /// Provider is an alias of another provider
    ProviderAlias,
    /// Module uses a local module
    LocalModuleRef,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDependsOn => write!(f, "depends_on"),
            Self::ModuleRequiresProvider => write!(f, "requires"),
            Self::ProviderAlias => write!(f, "alias_of"),
            Self::LocalModuleRef => write!(f, "local_ref"),
        }
    }
}
