//! Graph builder implementation.
//!
//! This module provides the `GraphBuilder` which constructs a `DependencyGraph`
//! from parsed module and provider references.

use crate::error::Result;
use crate::graph::types::{DependencyGraph, EdgeType};
use crate::types::{ModuleRef, ModuleSource, ProviderRef, RuntimeRef};
use std::collections::HashMap;

/// Builder for constructing dependency graphs.
///
/// The builder takes parsed module and provider references and constructs
/// a complete dependency graph with all relationships.
///
/// # Algorithm
///
/// 1. **Node Creation Phase**:
///    - Add all modules as nodes
///    - Add all providers as nodes
///    - Deduplicate by canonical ID
///
/// 2. **Edge Creation Phase**:
///    - Link modules to their required providers
///    - Link modules to other modules they reference
///    - Detect local module references
///
/// # Example
///
/// ```rust,no_run
/// use monphare::graph::GraphBuilder;
/// use monphare::types::{ModuleRef, ProviderRef, RuntimeRef};
///
/// let builder = GraphBuilder::new();
/// let modules: Vec<ModuleRef> = vec![/* ... */];
/// let providers: Vec<ProviderRef> = vec![/* ... */];
/// let runtimes: Vec<RuntimeRef> = vec![/* ... */];
///
/// let graph = builder.build(&modules, &providers, &runtimes).unwrap();
/// println!("Built graph with {} nodes", graph.node_count());
/// ```
pub struct GraphBuilder {
    /// Map from provider name to provider source for resolution
    provider_map: HashMap<String, String>,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    /// Create a new graph builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider_map: HashMap::new(),
        }
    }

    /// Build a dependency graph from modules and providers.
    ///
    /// # Errors
    ///
    /// Returns an error if graph construction fails.
    pub fn build(
        mut self,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
        runtimes: &[RuntimeRef],
    ) -> Result<DependencyGraph> {
        tracing::debug!(
            modules = modules.len(),
            providers = providers.len(),
            runtimes = runtimes.len(),
            "Starting graph construction"
        );
        let mut graph = DependencyGraph::new();

        // Phase 1: Build provider map for resolution
        tracing::debug!("Phase 1: Building provider map");
        self.build_provider_map(providers);
        tracing::debug!(
            provider_map_size = self.provider_map.len(),
            "Provider map built"
        );

        // Phase 2: Add all provider nodes
        tracing::debug!("Phase 2: Adding provider nodes");
        let provider_ids: HashMap<String, String> = providers
            .iter()
            .map(|p| {
                let id = graph.add_provider(p);
                (p.qualified_source(), id)
            })
            .collect();
        tracing::debug!(provider_nodes = provider_ids.len(), "Provider nodes added");

        // Phase 3: Add all module nodes
        tracing::debug!("Phase 3: Adding module nodes");
        let module_ids: Vec<(String, &ModuleRef)> = modules
            .iter()
            .map(|m| {
                let id = graph.add_module(m);
                (id, m)
            })
            .collect();
        tracing::debug!(module_nodes = module_ids.len(), "Module nodes added");

        tracing::debug!("Phase 3a: Adding runtime nodes");
        let runtime_ids: Vec<(String, &RuntimeRef)> = runtimes
            .iter()
            .map(|r| {
                let id = graph.add_runtime(r);
                (id, r)
            })
            .collect();
        tracing::debug!(runtime_nodes = runtime_ids.len(), "Runtime nodes added");

        // Phase 4: Create edges
        tracing::debug!("Phase 4: Creating edges");
        let mut module_provider_edges = 0;
        let mut local_module_edges = 0;
        for (module_id, module) in &module_ids {
            // Link to providers based on module source
            if let Some(provider_source) = self.infer_provider_for_module(module) {
                if let Some(provider_id) = provider_ids.get(&provider_source) {
                    graph.add_edge(module_id, provider_id, EdgeType::ModuleRequiresProvider);
                    module_provider_edges += 1;
                    tracing::debug!(
                        module_id = %module_id,
                        provider_id = %provider_id,
                        provider_source = %provider_source,
                        "Added module-provider edge"
                    );
                } else {
                    tracing::debug!(
                        module_id = %module_id,
                        provider_source = %provider_source,
                        "Provider source not found in provider map"
                    );
                }
            }

            // Link local module references
            if let ModuleSource::Local { path } = &module.source {
                // Find other modules that might be the target of this local reference
                for (other_id, other_module) in &module_ids {
                    if self.is_local_module_match(path, other_module) {
                        graph.add_edge(module_id, other_id, EdgeType::LocalModuleRef);
                        local_module_edges += 1;
                        tracing::debug!(
                            module_id = %module_id,
                            other_id = %other_id,
                            path = %path,
                            "Added local module reference edge"
                        );
                    }
                }
            }
        }
        tracing::debug!(
            module_provider_edges = module_provider_edges,
            local_module_edges = local_module_edges,
            "Edge creation complete"
        );

        // Phase 5: Link modules that share the same source (implicit dependency)
        tracing::debug!("Phase 5: Linking modules with shared sources");
        self.link_shared_sources(&mut graph, &module_ids);

        tracing::info!(
            nodes = graph.node_count(),
            edges = graph.edge_count(),
            "Graph built successfully"
        );

        Ok(graph)
    }

    /// Build a map from provider local names to their sources.
    fn build_provider_map(&mut self, providers: &[ProviderRef]) {
        for provider in providers {
            let source = provider.qualified_source();
            tracing::debug!(
                provider_name = %provider.name,
                qualified_source = %source,
                "Mapping provider name to source"
            );
            self.provider_map.insert(provider.name.clone(), source);
        }
    }

    /// Infer which provider a module requires based on its source.
    fn infer_provider_for_module(&self, module: &ModuleRef) -> Option<String> {
        let inferred = match &module.source {
            ModuleSource::Registry { provider, .. } => {
                tracing::debug!(
                    module_name = %module.name,
                    provider = %provider,
                    "Inferring provider from registry module"
                );
                // Registry modules have an explicit provider
                // Look up the full source from our provider map
                self.provider_map
                    .get(provider)
                    .cloned()
                    .or_else(|| Some(format!("hashicorp/{provider}")))
            }
            ModuleSource::Git { url, .. } => {
                tracing::debug!(
                    module_name = %module.name,
                    url = %url,
                    "Attempting to infer provider from Git URL"
                );
                // Try to infer provider from Git URL
                // e.g., terraform-aws-modules implies AWS
                if url.contains("-aws-") || url.contains("/aws-") {
                    let provider = self
                        .provider_map
                        .get("aws")
                        .cloned()
                        .or_else(|| Some("hashicorp/aws".to_string()));
                    tracing::debug!(
                        module_name = %module.name,
                        inferred_provider = %provider.as_ref().unwrap_or(&"none".to_string()),
                        "Inferred AWS provider from Git URL"
                    );
                    return provider;
                }
                if url.contains("-google-") || url.contains("/google-") {
                    let provider = self
                        .provider_map
                        .get("google")
                        .cloned()
                        .or_else(|| Some("hashicorp/google".to_string()));
                    tracing::debug!(
                        module_name = %module.name,
                        inferred_provider = %provider.as_ref().unwrap_or(&"none".to_string()),
                        "Inferred Google provider from Git URL"
                    );
                    return provider;
                }
                if url.contains("-azurerm-") || url.contains("/azurerm-") {
                    let provider = self
                        .provider_map
                        .get("azurerm")
                        .cloned()
                        .or_else(|| Some("hashicorp/azurerm".to_string()));
                    tracing::debug!(
                        module_name = %module.name,
                        inferred_provider = %provider.as_ref().unwrap_or(&"none".to_string()),
                        "Inferred AzureRM provider from Git URL"
                    );
                    return provider;
                }
                tracing::debug!(
                    module_name = %module.name,
                    url = %url,
                    "Could not infer provider from Git URL"
                );
                None
            }
            _ => {
                tracing::debug!(
                    module_name = %module.name,
                    "Cannot infer provider for module source type"
                );
                None
            }
        };
        if let Some(ref provider) = inferred {
            tracing::debug!(
                module_name = %module.name,
                provider = %provider,
                "Successfully inferred provider"
            );
        }
        inferred
    }

    /// Check if a local path might reference another module.
    fn is_local_module_match(&self, path: &str, other_module: &ModuleRef) -> bool {
        // Simplified matching: check if the path ends with the module name
        // A more sophisticated implementation would resolve paths
        let path_parts: Vec<&str> = path.split('/').collect();
        if let Some(last_part) = path_parts.last() {
            return *last_part == other_module.name;
        }
        false
    }

    /// Link modules that share the same source.
    ///
    /// This helps identify modules that might have version conflicts.
    fn link_shared_sources(
        &self,
        _graph: &mut DependencyGraph,
        module_ids: &[(String, &ModuleRef)],
    ) {
        // Group modules by source
        let mut by_source: HashMap<String, Vec<&str>> = HashMap::new();

        for (id, module) in module_ids {
            let source_key = module.source.canonical_id();
            by_source.entry(source_key).or_default().push(id.as_str());
        }

        // For sources with multiple modules, we don't create edges
        // (that would be misleading), but we track this for analysis
        for (source, ids) in &by_source {
            if ids.len() > 1 {
                tracing::debug!(
                    source = %source,
                    count = ids.len(),
                    "Multiple modules share the same source"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Constraint;
    use std::path::PathBuf;

    fn create_module(name: &str, namespace: &str, provider: &str) -> ModuleRef {
        ModuleRef {
            name: name.to_string(),
            source: ModuleSource::Registry {
                hostname: "registry.terraform.io".to_string(),
                namespace: namespace.to_string(),
                name: name.to_string(),
                provider: provider.to_string(),
            },
            version_constraint: Some(Constraint::parse("~> 1.0").unwrap()),
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: Some("test".to_string()),
            attributes: Default::default(),
        }
    }

    fn create_provider(name: &str, source: &str) -> ProviderRef {
        ProviderRef {
            name: name.to_string(),
            source: Some(source.to_string()),
            version_constraint: Some(Constraint::parse(">= 4.0").unwrap()),
            file_path: PathBuf::from("versions.tf"),
            line_number: 1,
            repository: Some("test".to_string()),
        }
    }

    #[test]
    fn test_build_empty_graph() {
        let builder = GraphBuilder::new();
        let graph = builder.build(&[], &[], &[]).unwrap();

        assert_eq!(graph.node_count(), 0); // 0 modules, 0 providers, 0 runtimes
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_build_with_modules_and_providers() {
        let modules = vec![
            create_module("vpc", "terraform-aws-modules", "aws"),
            create_module("eks", "terraform-aws-modules", "aws"),
        ];
        let providers = vec![create_provider("aws", "hashicorp/aws")];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &providers, &[]).unwrap();

        // 2 modules + 1 provider
        assert_eq!(graph.node_count(), 3);

        // Both modules should link to AWS provider
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_build_with_local_module() {
        let modules = vec![
            ModuleRef {
                name: "local_vpc".to_string(),
                source: ModuleSource::Local {
                    path: "../modules/vpc".to_string(),
                },
                version_constraint: None,
                file_path: PathBuf::from("main.tf"),
                line_number: 1,
                repository: Some("test".to_string()),
                attributes: Default::default(),
            },
            ModuleRef {
                name: "vpc".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "vpc".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: None,
                file_path: PathBuf::from("modules/vpc/main.tf"),
                line_number: 1,
                repository: Some("test".to_string()),
                attributes: Default::default(),
            },
        ];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &[], &[]).unwrap();

        assert_eq!(graph.node_count(), 2);
        // Local module should link to the vpc module
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_infer_provider_from_git_url() {
        let builder = GraphBuilder::new();

        // Create a module with AWS in the Git URL
        let module = ModuleRef {
            name: "vpc".to_string(),
            source: ModuleSource::Git {
                host: "github.com/terraform-aws-modules/terraform-aws-vpc".to_string(),
                url: "https://github.com/terraform-aws-modules/terraform-aws-vpc.git".to_string(),
                ref_: None,
                subdir: None,
            },
            version_constraint: None,
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: None,
            attributes: Default::default(),
        };

        let provider = builder.infer_provider_for_module(&module);
        assert_eq!(provider, Some("hashicorp/aws".to_string()));
    }

    #[test]
    fn test_multiple_repos_same_module() {
        let modules = vec![
            ModuleRef {
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
                repository: Some("repo-a".to_string()),
                attributes: Default::default(),
            },
            ModuleRef {
                name: "vpc".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "vpc".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("~> 4.0").unwrap()),
                file_path: PathBuf::from("main.tf"),
                line_number: 1,
                repository: Some("repo-b".to_string()),
                attributes: Default::default(),
            },
        ];

        let builder = GraphBuilder::new();
        let graph = builder.build(&modules, &[], &[]).unwrap();

        // Should have 2 separate nodes (different repos)
        assert_eq!(graph.node_count(), 2);
    }
}
