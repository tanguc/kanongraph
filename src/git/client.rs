//! Git client implementation.
//!
//! Provides a unified interface for cloning repositories from
//! multiple Git providers, with smart caching to avoid unnecessary re-clones.

use crate::config::Config;
use crate::error::{MonPhareError, Result};
use crate::git::cache::CacheManager;
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
/// Supports caching to avoid unnecessary re-clones when repositories haven't changed.
pub struct GitClient {
    config: Config,
    providers: Vec<Arc<dyn GitProvider>>,
    temp_dir: PathBuf,
    cache_manager: CacheManager,
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

        // Create temp directory for cloned repos (fallback when cache disabled)
        let temp_dir = std::env::temp_dir();

        // Create cache manager
        let cache_manager = CacheManager::new(&config.cache);

        Self {
            config,
            providers,
            temp_dir,
            cache_manager,
        }
    }

    /// Clone a repository and return the local path.
    ///
    /// If caching is enabled, this will:
    /// 1. Check if the repository is already cached
    /// 2. If cached, fetch updates and check if HEAD changed
    /// 3. If not cached or cache miss, perform a fresh clone
    ///
    /// The path returned can be used for scanning.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The provider is not supported
    /// - Authentication fails
    /// - Cloning/fetching fails
    pub async fn clone_repository(&self, url: &str) -> Result<PathBuf> {
        tracing::debug!(url = %url, cache_enabled = self.cache_manager.is_enabled(), "Starting repository clone");

        // Find the appropriate provider
        let provider = self.find_provider(url)?;
        let branch = self.config.git.branch.as_deref();
        let token = self.get_token_for_url(url)?;

        // Check if we should use cache
        if self.cache_manager.is_enabled() {
            return self.clone_with_cache(url, provider, branch, &token).await;
        }

        // Fallback to non-cached clone
        self.clone_without_cache(url, provider, branch, &token)
            .await
    }

    /// Clone a repository using the cache.
    async fn clone_with_cache(
        &self,
        url: &str,
        provider: &Arc<dyn GitProvider>,
        branch: Option<&str>,
        token: &str,
    ) -> Result<PathBuf> {
        // Ensure cache directory exists
        self.cache_manager.ensure_cache_dir().await?;

        let cache_path = self.cache_manager.get_cache_path(url);

        // Check if we have a cached version
        if let Some(cache_entry) = self.cache_manager.get_cached(url).await {
            // Check if cache is fresh enough to skip fetch entirely
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let age_seconds = now.saturating_sub(cache_entry.last_updated);
            let fresh_threshold = self.cache_manager.fresh_threshold_seconds();

            if age_seconds < fresh_threshold {
                tracing::info!(
                    url = %url,
                    sha = %cache_entry.head_sha,
                    age_seconds = age_seconds,
                    fresh_threshold = fresh_threshold,
                    "Using fresh cache, skipping fetch"
                );
                // Update last accessed time
                let _ = self.cache_manager.touch_cache_entry(url).await;
                return Ok(cache_path);
            }

            tracing::info!(
                url = %url,
                path = %cache_path.display(),
                cached_sha = %cache_entry.head_sha,
                age_seconds = age_seconds,
                "Cache stale, checking for updates"
            );

            // Try to fetch updates
            match self.cache_manager.fetch_updates(&cache_path, branch).await {
                Ok(new_sha) => {
                    if new_sha == cache_entry.head_sha {
                        tracing::info!(
                            url = %url,
                            sha = %new_sha,
                            "Repository unchanged"
                        );
                        // Refresh cache entry timestamps (both last_accessed and last_updated)
                        // This ensures subsequent runs within fresh_threshold won't re-fetch
                        let _ = self.cache_manager.refresh_cache_entry(url).await;
                    } else {
                        tracing::info!(
                            url = %url,
                            old_sha = %cache_entry.head_sha,
                            new_sha = %new_sha,
                            "Repository updated"
                        );
                        // Update cache entry with new SHA
                        self.cache_manager
                            .update_cache_entry(url, &new_sha, branch)
                            .await?;
                    }
                    return Ok(cache_path);
                }
                Err(e) => {
                    tracing::warn!(
                        url = %url,
                        error = %e,
                        "Failed to fetch updates, will re-clone"
                    );
                    // Remove corrupted cache and re-clone
                    let _ = tokio::fs::remove_dir_all(&cache_path).await;
                }
            }
        }

        // No cache or cache invalid - perform fresh clone
        tracing::info!(
            provider = provider.name(),
            url = %url,
            path = %cache_path.display(),
            "Cloning repository to cache"
        );

        // Remove existing directory if it exists (corrupted cache)
        if cache_path.exists() {
            tokio::fs::remove_dir_all(&cache_path)
                .await
                .map_err(|e| MonPhareError::io(&cache_path, e, file!(), line!()))?;
        }

        // Clone the repository
        provider
            .clone_repo(url, &cache_path, branch, Some(token))
            .await?;

        // Get the HEAD SHA and create cache entry
        let head_sha = self.cache_manager.get_head_sha(&cache_path).await?;
        self.cache_manager
            .update_cache_entry(url, &head_sha, branch)
            .await?;

        tracing::info!(
            path = %cache_path.display(),
            sha = %head_sha,
            "Repository cloned and cached successfully"
        );

        Ok(cache_path)
    }

    /// Clone a repository without caching (original behavior).
    async fn clone_without_cache(
        &self,
        url: &str,
        provider: &Arc<dyn GitProvider>,
        branch: Option<&str>,
        token: &str,
    ) -> Result<PathBuf> {
        tracing::info!(
            provider = provider.name(),
            url = %url,
            "Cloning repository (cache disabled)"
        );

        // Generate a unique directory name
        let repo_name = self.extract_repo_name(url);
        let target_path = self.temp_dir.join(&repo_name);

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
        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|e| MonPhareError::io(&self.temp_dir, e, file!(), line!()))?;

        // Clone the repository
        provider
            .clone_repo(url, &target_path, branch, Some(token))
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
        let url = url.trim_end_matches('/').trim_end_matches(".git");

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
    ///
    /// # Errors
    /// Returns an error if no token is found for the detected platform.
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
