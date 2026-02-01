//! # MonPhare
//!
//! A Terraform/OpenTofu module constraint analyzer and dependency mapper.
//!
//! MonPhare scans Terraform/OpenTofu repositories, parses HCL files, builds
//! dependency graphs, and detects version constraint conflicts, deprecated
//! modules, and risky patterns.
//!
//! ## Features
//!
//! - **Multi-provider Git support**: Clone repositories from GitHub, GitLab,
//!   Bitbucket, and Azure DevOps
//! - **HCL parsing**: Extract module blocks, provider requirements, and version
//!   constraints
//! - **Dependency graph**: Build and visualize module/provider relationships
//! - **Conflict detection**: Identify version constraint conflicts across
//!   repositories
//! - **Risk analysis**: Flag deprecated modules, missing constraints, and risky
//!   patterns
//! - **Multiple output formats**: JSON, plain text, and self-contained HTML
//!   reports
//!
//! ## Example
//!
//! ```rust,no_run
//! use monphare::{Scanner, Config, ReportFormat};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::default();
//!     let scanner = Scanner::new(config);
//!     
//!     // Scan a local directory
//!     let result = scanner.scan_path("./terraform").await?;
//!     
//!     // Generate a report
//!     let report = result.generate_report(ReportFormat::Json)?;
//!     println!("{}", report);
//!     
//!     Ok(())
//! }
//! ```

// Note: README is not included as doc to avoid doctest failures
// See README.md for full documentation
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

pub mod analyzer;
pub mod cli;
pub mod config;
pub mod error;
pub mod git;
pub mod graph;
pub mod parser;
pub mod reporter;
pub mod types;
pub mod vcs;
pub mod vcs_clients;
// Re-export commonly used types at crate root
pub use config::Config;
pub use error::{MonPhareError, Result};
use tokio::io::AsyncReadExt;
pub use vcs::VcsPlatform;
pub use types::{ 
    AnalysisResult, Constraint, ModuleRef, ProviderRef, ReportFormat, ScanResult, Severity,
    VersionRange,
};

use std::path::Path;

/// Main scanner orchestrator that coordinates all analysis operations.
///
/// The `Scanner` is the primary entry point for using MonPhare as a library.
/// It handles:
/// - Cloning remote repositories
/// - Scanning local directories
/// - Coordinating parsing, graph building, and analysis
///
/// # Example
///
/// ```rust,no_run
/// use monphare::{Scanner, Config};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = Config::default();
///     let scanner = Scanner::new(config);
///     
///     // Scan multiple paths
///     let paths = vec!["./repo1", "./repo2"];
///     let result = scanner.scan_paths(&paths).await?;
///     
///     println!("Found {} modules", result.modules.len());
///     Ok(())
/// }
/// ```
pub struct Scanner {
    config: Config,
    git_client: git::GitClient,
}

impl Scanner {
    /// Create a new scanner with the given configuration.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let git_client = git::GitClient::new(config.clone());
        Self { config, git_client }
    }

    /// Scan a single local path for Terraform/OpenTofu files.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The path doesn't exist or isn't accessible
    /// - HCL parsing fails
    /// - Graph construction fails
    pub async fn scan_path<P: AsRef<Path>>(&self, path: P) -> Result<ScanResult> {
        self.scan_paths(&[path.as_ref()]).await
    }

    /// Scan multiple local paths for Terraform/OpenTofu files.
    ///
    /// # Errors
    ///
    /// Returns an error if any path fails to scan.
    pub async fn scan_paths<P: AsRef<Path>>(&self, paths: &[P]) -> Result<ScanResult> {
        let parser = parser::HclParser::new(&self.config);
        let mut all_modules = Vec::new();
        let mut all_providers = Vec::new();
        let mut all_runtimes = Vec::new();
        let mut all_files = Vec::new();

        for path in paths {
            let path = path.as_ref();
            tracing::info!(path = %path.display(), "Scanning path");

            let parsed = parser.parse_directory(path).await?;
            all_runtimes.extend(parsed.runtimes);
            all_modules.extend(parsed.modules);
            all_providers.extend(parsed.providers);
            all_files.extend(parsed.files);
        }

        // Build dependency graph
        let graph_builder = graph::GraphBuilder::new();
        let dependency_graph = graph_builder.build(&all_modules, &all_providers, &all_runtimes)?;

        // Run analysis
        let analyzer = analyzer::Analyzer::new(&self.config);
        let analysis = analyzer.analyze(&dependency_graph, &all_modules, &all_providers, &all_runtimes)?;

        Ok(ScanResult {
            modules: all_modules,
            providers: all_providers,
            runtimes: all_runtimes,
            files_scanned: all_files,
            graph: dependency_graph,
            analysis,
        })
    }

    /// Clone and scan a remote Git repository.
    ///
    /// Supports GitHub, GitLab, Bitbucket, and Azure DevOps URLs.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The repository cannot be cloned
    /// - Authentication fails
    /// - Scanning fails
    pub async fn scan_repository(&self, url: &str) -> Result<ScanResult> {
        tracing::info!(url = %url, "Cloning repository");
        let local_path = self.git_client.clone_repository(url).await?;
        self.scan_path(&local_path).await
    }

    /// Scan multiple repositories in parallel.
    ///
    /// # Errors
    ///
    /// Returns an error if any repository fails to clone or scan.
    pub async fn scan_repositories(&self, urls: &[&str]) -> Result<ScanResult> {
        use futures::future::try_join_all;

        let futures: Vec<_> = urls
            .iter()
            .map(|url| self.scan_repository(url))
            .collect();

        let results = try_join_all(futures).await?;

        // Merge all results
        let mut merged = ScanResult::default();
        for result in results {
            merged.merge(result);
        }

        Ok(merged)
    }    
    /// Scan all repositories in a VCS organization/group
    ///
    /// # Errors
    ///
    /// Returns an error if repository discovery fails or scanning fails
    pub async fn scan_vcs_organization(
        &self,
        platform: VcsPlatform,
        org_spec: &str,
        _skip_confirmation: bool,
    ) -> Result<ScanResult> {
        use crate::vcs::{VcsClient, VcsIdentifier};
        use crate::vcs_clients::{
            CachedRateLimitedClient, GitHubClient, GitLabClient,
            AzureDevOpsClient, BitbucketClient
        };
        use indicatif::{ProgressBar, ProgressStyle};

        tracing::info!(
            platform = %platform.as_str(),
            org = %org_spec,
            "Starting bulk VCS organization scanning"
        );

        // Get token for the platform
        let token = self.config.git.get_token_for_platform(platform.as_str())?;

        // Create the appropriate client
        let client = CachedRateLimitedClient::default();
        let repositories = match platform {
            VcsPlatform::GitHub => {
                let gh_client = GitHubClient::new(client);
                gh_client.discover_repositories(org_spec, &token).await?
            }
            VcsPlatform::GitLab => {
                let gl_client = GitLabClient::new(client);
                gl_client.discover_repositories(org_spec, &token).await?
            }
            VcsPlatform::AzureDevOps => {
                let ado_client = AzureDevOpsClient::new(client);
                ado_client.discover_repositories(org_spec, &token).await?
            }
            VcsPlatform::Bitbucket => {
                let bb_client = BitbucketClient::new(client);
                bb_client.discover_repositories(org_spec, &token).await?
            }
            VcsPlatform::Local => {
                return Err(crate::error::MonPhareError::ConfigParse {
                    message: "Cannot use Local platform for organization scanning".to_string(),
                    source: None,
                });
            }
        };

        // Filter out archived and fork repositories
        let repos_to_scan: Vec<_> = repositories
            .into_iter()
            .filter(|r| !r.archived && !r.fork)
            .collect();

        let repo_count = repos_to_scan.len();
        tracing::info!(
            platform = %platform.as_str(),
            org = %org_spec,
            count = repo_count,
            "Discovered {} repositories to scan",
            repo_count
        );

        if repo_count == 0 {
            return Ok(ScanResult::default());
        }

        // Create progress bar
        let progress = ProgressBar::new(repo_count as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        // Scan each repository
        let mut merged = ScanResult::default();
        let mut errors = Vec::new();

        for repo in repos_to_scan {
            progress.set_message(format!("Scanning {}", repo.name));

            match self.scan_repository(&repo.clone_url).await {
                Ok(mut result) => {
                    // Add VCS metadata to the graph
                    let vcs_id = VcsIdentifier::new(
                        platform.as_str(),
                        &repo.name.split('/').collect::<Vec<_>>()
                    );

                    // Mark all nodes from this repo with VCS identifier
                    // Collect IDs first to avoid borrow checker issues
                    let module_ids: Vec<_> = result.graph.module_ids().into_iter().cloned().collect();
                    let provider_ids: Vec<_> = result.graph.provider_ids().into_iter().cloned().collect();

                    for module_id in module_ids {
                        result.graph.set_vcs_metadata(&module_id, vcs_id.clone());
                    }
                    for provider_id in provider_ids {
                        result.graph.set_vcs_metadata(&provider_id, vcs_id.clone());
                    }

                    merged.merge(result);
                }
                Err(e) => {
                    tracing::warn!(repo = %repo.name, error = %e, "Failed to scan repository");
                    if !self.config.scan.continue_on_error {
                        return Err(e);
                    }
                    errors.push((repo.name.clone(), e));
                }
            }

            progress.inc(1);
        }

        progress.finish_with_message(format!(
            "Scanned {} repositories ({} errors)",
            repo_count - errors.len(),
            errors.len()
        ));

        if !errors.is_empty() {
            tracing::warn!(
                "Failed to scan {} repositories: {:?}",
                errors.len(),
                errors.iter().map(|(name, _)| name).collect::<Vec<_>>()
            );
        }

        Ok(merged)
    }
}

pub async fn wait_for_user_input() {
    println!("Press Enter to continue...");
    let mut input: [u8; 1] = [200; 1];
    tokio::io::stdin().read(&mut input).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let config = Config::default();
        let _scanner = Scanner::new(config);
    }
}

