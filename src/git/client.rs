//! Git client implementation.
//!
//! Provides a unified interface for cloning repositories from
//! multiple Git providers.

use crate::config::Config;
use crate::error::{MonPhareError, Result};
use crate::git::providers::{
    AzureDevOpsProvider, BitbucketProvider, GitHubProvider, GitLabProvider, GitProvider,
};
use crate::git::ProviderType;
use std::path::PathBuf;
use std::sync::Arc;

/// Git client for cloning repositories from multiple providers.
///
/// The client automatically detects the provider from the URL and
/// uses the appropriate authentication and cloning strategy.
pub struct GitClient {
    config: Config,
    providers: Vec<Arc<dyn GitProvider>>,
    temp_dir: PathBuf,
}

impl GitClient {
    /// Create a new Git client with the given configuration.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let providers: Vec<Arc<dyn GitProvider>> = vec![
            Arc::new(GitHubProvider::new()),
            Arc::new(GitLabProvider::new()),
            Arc::new(BitbucketProvider::new()),
            Arc::new(AzureDevOpsProvider::new()),
        ];

        // Create temp directory for cloned repos
        let temp_dir = std::env::temp_dir().join("monphare-repos");

        Self {
            config,
            providers,
            temp_dir,
        }
    }

    /// Clone a repository and return the local path.
    ///
    /// The repository is cloned to a temporary directory and the path
    /// is returned for scanning.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The provider is not supported
    /// - Authentication fails
    /// - Cloning fails
    pub async fn clone_repository(&self, url: &str) -> Result<PathBuf> {
        tracing::debug!(url = %url, "Starting repository clone");
        // Find the appropriate provider
        let provider = self.find_provider(url)?;
        tracing::debug!(
            provider = provider.name(),
            url = %url,
            "Found provider for URL"
        );

        tracing::info!(
            provider = provider.name(),
            url = %url,
            "Cloning repository"
        );

        // Generate a unique directory name
        let repo_name = self.extract_repo_name(url);
        let target_path = self.temp_dir.join(&repo_name);
        tracing::debug!(
            repo_name = %repo_name,
            target_path = %target_path.display(),
            "Generated target path for clone"
        );

        // Remove existing directory if it exists
        if target_path.exists() {
            tracing::debug!(
                path = %target_path.display(),
                "Removing existing directory before clone"
            );
            tokio::fs::remove_dir_all(&target_path)
                .await
                .map_err(|e| MonPhareError::io(&target_path, e, file!(), line!()))?;
        }

        // Create parent directory
        tracing::debug!(
            temp_dir = %self.temp_dir.display(),
            "Creating temp directory if needed"
        );
        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|e| MonPhareError::io(&self.temp_dir, e, file!(), line!()))?;

        // Clone the repository
        let branch = self.config.git.branch.as_deref();
        let token = self.get_token_for_url(url)?;
        tracing::debug!(
            url = %url,
            target_path = %target_path.display(),
            branch = ?branch,
            has_token = token,
            "Starting repository clone operation"
        );
        provider
            .clone_repo(
                url,
                &target_path,
                branch,
                Some(&token),
            )
            .await?;

        tracing::info!(
            path = %target_path.display(),
            "Repository cloned successfully"
        );

        Ok(target_path)
    }

    /// Find the provider that can handle the given URL.
    fn find_provider(&self, url: &str) -> Result<&Arc<dyn GitProvider>> {
        tracing::debug!(url = %url, "Finding provider for URL");
        for provider in &self.providers {
            if provider.can_handle(url) {
                tracing::debug!(
                    url = %url,
                    provider = provider.name(),
                    "Found matching provider"
                );
                return Ok(provider);
            }
        }

        tracing::debug!(url = %url, "No provider found for URL");
        Err(crate::err!(UnsupportedGitProvider {
            url: url.to_string(),
        }))
    }

    /// Extract a repository name from a URL for use as a directory name.
    fn extract_repo_name(&self, url: &str) -> String {
        // Try to extract org/repo from the URL
        let url = url
            .trim_end_matches('/')
            .trim_end_matches(".git");

        // Split by common separators
        let parts: Vec<&str> = url.split(&['/', ':'][..]).collect();

        // Take the last two parts (org/repo) or just the last part
        if parts.len() >= 2 {
            let org = parts[parts.len() - 2];
            let repo = parts[parts.len() - 1];
            format!("{org}-{repo}")
        } else if let Some(last) = parts.last() {
            (*last).to_string()
        } else {
            // Fallback to a hash of the URL
            format!("repo-{:x}", md5_hash(url))
        }
    }

    /// Get the provider type for a URL.
    #[must_use]
    pub fn get_provider_type(&self, url: &str) -> ProviderType {
        ProviderType::from_url(url)
    }

    /// Get appropriate token for a URL based on platform detection.
    #[must_use]
    pub fn get_token_for_url(&self, url: &str) -> Result<String> {
        let platform = self.get_provider_type(url);
        let platform_str = match platform {
            ProviderType::GitHub => "github",
            ProviderType::GitLab => "gitlab",
            ProviderType::Bitbucket => "bitbucket",
            ProviderType::AzureDevOps => "ado",
            ProviderType::Unknown => {
                tracing::debug!(url = %url, "Unknown platform, no token available");
                return Err(crate::err!(UnsupportedGitProvider {
                    url: url.to_string(),
                }));
            }
        };

        tracing::debug!(
            url = %url,
            platform = platform_str,
            "Getting token for platform"
        );
        let token = self.config.git.get_token_for_platform(platform_str);

        tracing::debug!(
            url = %url,
            platform = platform_str,
            has_token = token.is_ok(),
            "Token retrieval complete"
        );

        token
    }

    /// Clean up temporary directories.
    ///
    /// # Errors
    ///
    /// Returns an error if cleanup fails.
    pub async fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            tokio::fs::remove_dir_all(&self.temp_dir)
                .await
                .map_err(|e| MonPhareError::io(&self.temp_dir, e, file!(), line!()))?;
        }
        Ok(())
    }
}

/// Simple hash function for generating unique directory names.
fn md5_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name() {
        let config = Config::default();
        let client = GitClient::new(config);

        assert_eq!(
            client.extract_repo_name("https://github.com/hashicorp/terraform"),
            "hashicorp-terraform"
        );

        assert_eq!(
            client.extract_repo_name("https://github.com/hashicorp/terraform.git"),
            "hashicorp-terraform"
        );

        assert_eq!(
            client.extract_repo_name("git@github.com:hashicorp/terraform.git"),
            "hashicorp-terraform"
        );
    }

    #[test]
    fn test_get_provider_type() {
        let config = Config::default();
        let client = GitClient::new(config);

        assert_eq!(
            client.get_provider_type("https://github.com/org/repo"),
            ProviderType::GitHub
        );

        assert_eq!(
            client.get_provider_type("https://gitlab.com/org/repo"),
            ProviderType::GitLab
        );

        assert_eq!(
            client.get_provider_type("https://bitbucket.org/org/repo"),
            ProviderType::Bitbucket
        );

        assert_eq!(
            client.get_provider_type("https://dev.azure.com/org/project/_git/repo"),
            ProviderType::AzureDevOps
        );
    }

    #[test]
    fn test_find_provider() {
        let config = Config::default();
        let client = GitClient::new(config);

        assert!(client.find_provider("https://github.com/org/repo").is_ok());
        assert!(client.find_provider("https://gitlab.com/org/repo").is_ok());
        assert!(client
            .find_provider("https://bitbucket.org/org/repo")
            .is_ok());
        assert!(client
            .find_provider("https://dev.azure.com/org/project/_git/repo")
            .is_ok());
        assert!(client
            .find_provider("https://unknown-provider.com/repo")
            .is_err());
    }
}

