//! Git provider implementations.
//!
//! This module contains implementations for each supported Git provider.

use crate::{error::{MonPhareError, Result}};
use async_trait::async_trait;
use std::path::PathBuf;

/// Trait for Git provider implementations.
///
/// Each provider implements this trait to handle provider-specific
/// URL parsing, authentication, and cloning logic.
#[async_trait]
pub trait GitProvider: Send + Sync {
    /// Get the provider name.
    fn name(&self) -> &'static str;

    /// Check if this provider can handle the given URL.
    fn can_handle(&self, url: &str) -> bool;

    /// Normalize the URL for cloning.
    ///
    /// This converts various URL formats (SSH, HTTPS, shorthand) into
    /// a consistent format for cloning.
    fn normalize_url(&self, url: &str) -> Result<String>;

    /// Clone a repository to the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if cloning fails.
    async fn clone_repo(
        &self,
        url: &str,
        target_path: &PathBuf,
        branch: Option<&str>,
        token: Option<&str>,
    ) -> Result<()>;
}

/// GitHub provider implementation.
pub struct GitHubProvider;

impl GitHubProvider {
    /// Create a new GitHub provider.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitProvider for GitHubProvider {
    fn name(&self) -> &'static str {
        "GitHub"
    }

    fn can_handle(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        url_lower.contains("github.com") || url_lower.contains("github.")
    }

    fn normalize_url(&self, url: &str) -> Result<String> {
        // Handle various GitHub URL formats:
        // - https://github.com/org/repo
        // - https://github.com/org/repo.git
        // - git@github.com:org/repo.git
        // - github.com/org/repo

        let url = url.trim();

        // SSH format
        if url.starts_with("git@github.com:") {
            let path = url
                .strip_prefix("git@github.com:")
                .unwrap()
                .trim_end_matches(".git");
            return Ok(format!("https://github.com/{path}.git"));
        }

        // Already HTTPS
        if url.starts_with("https://github.com/") {
            let normalized = if url.ends_with(".git") {
                url.to_string()
            } else {
                format!("{url}.git")
            };
            return Ok(normalized);
        }

        // Shorthand (github.com/org/repo)
        if url.starts_with("github.com/") {
            let path = url.strip_prefix("github.com/").unwrap();
            return Ok(format!("https://github.com/{path}.git"));
        }

        Err(MonPhareError::InvalidGitUrl {
            url: url.to_string(),
            message: "Could not parse GitHub URL".to_string(),
        })
    }

    async fn clone_repo(
        &self,
        url: &str,
        target_path: &PathBuf,
        branch: Option<&str>,
        token: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(
            provider = "GitHub",
            url = %url,
            target_path = %target_path.display(),
            branch = ?branch,
            has_token = token.is_some(),
            "Normalizing GitHub URL"
        );
        let normalized_url = self.normalize_url(url)?;
        tracing::debug!(
            original_url = %url,
            normalized_url = %normalized_url,
            "URL normalized"
        );

        // Add authentication token if provided
        let clone_url = if let Some(token) = token {
            // Insert token into URL: https://token@github.com/...
            let authenticated_url = normalized_url.replace("https://", &format!("https://{token}@"));
            tracing::debug!(
                "Added authentication token to URL (token masked)"
            );
            authenticated_url
        } else {
            tracing::debug!("No token provided, cloning without authentication");
            normalized_url
        };

        tracing::debug!(
            target_path = %target_path.display(),
            branch = ?branch,
            "Starting git clone operation"
        );
        clone_repository_impl(&clone_url, target_path, branch).await
    }
}

/// GitLab provider implementation.
pub struct GitLabProvider;

impl GitLabProvider {
    /// Create a new GitLab provider.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for GitLabProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitProvider for GitLabProvider {
    fn name(&self) -> &'static str {
        "GitLab"
    }

    fn can_handle(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        url_lower.contains("gitlab.com") || url_lower.contains("gitlab.")
    }

    fn normalize_url(&self, url: &str) -> Result<String> {
        let url = url.trim();

        // SSH format
        if url.starts_with("git@gitlab.com:") {
            let path = url
                .strip_prefix("git@gitlab.com:")
                .unwrap()
                .trim_end_matches(".git");
            return Ok(format!("https://gitlab.com/{path}.git"));
        }

        // Already HTTPS
        if url.starts_with("https://gitlab.com/") {
            let normalized = if url.ends_with(".git") {
                url.to_string()
            } else {
                format!("{url}.git")
            };
            return Ok(normalized);
        }

        // Self-hosted GitLab
        if url.contains("gitlab.") && url.starts_with("https://") {
            return Ok(url.to_string());
        }

        Err(MonPhareError::InvalidGitUrl {
            url: url.to_string(),
            message: "Could not parse GitLab URL".to_string(),
        })
    }

    async fn clone_repo(
        &self,
        url: &str,
        target_path: &PathBuf,
        branch: Option<&str>,
        token: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(
            provider = "GitLab",
            url = %url,
            target_path = %target_path.display(),
            branch = ?branch,
            has_token = token.is_some(),
            "Normalizing GitLab URL"
        );
        let normalized_url = self.normalize_url(url)?;
        tracing::debug!(
            original_url = %url,
            normalized_url = %normalized_url,
            "URL normalized"
        );

        let clone_url = if let Some(token) = token {
            // GitLab uses oauth2 as username for token auth
            tracing::debug!("Adding GitLab OAuth2 token to URL (token masked)");
            normalized_url.replace("https://", &format!("https://oauth2:{token}@"))
        } else {
            tracing::debug!("No token provided, cloning without authentication");
            normalized_url
        };

        tracing::debug!(
            target_path = %target_path.display(),
            branch = ?branch,
            "Starting git clone operation"
        );
        clone_repository_impl(&clone_url, target_path, branch).await
    }
}

/// Bitbucket provider implementation.
pub struct BitbucketProvider;

impl BitbucketProvider {
    /// Create a new Bitbucket provider.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for BitbucketProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitProvider for BitbucketProvider {
    fn name(&self) -> &'static str {
        "Bitbucket"
    }

    fn can_handle(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        url_lower.contains("bitbucket.org") || url_lower.contains("bitbucket.")
    }

    fn normalize_url(&self, url: &str) -> Result<String> {
        let url = url.trim();

        // SSH format
        if url.starts_with("git@bitbucket.org:") {
            let path = url
                .strip_prefix("git@bitbucket.org:")
                .unwrap()
                .trim_end_matches(".git");
            return Ok(format!("https://bitbucket.org/{path}.git"));
        }

        // Already HTTPS
        if url.starts_with("https://bitbucket.org/") {
            let normalized = if url.ends_with(".git") {
                url.to_string()
            } else {
                format!("{url}.git")
            };
            return Ok(normalized);
        }

        Err(MonPhareError::InvalidGitUrl {
            url: url.to_string(),
            message: "Could not parse Bitbucket URL".to_string(),
        })
    }

    async fn clone_repo(
        &self,
        url: &str,
        target_path: &PathBuf,
        branch: Option<&str>,
        token: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(
            provider = "Bitbucket",
            url = %url,
            target_path = %target_path.display(),
            branch = ?branch,
            has_token = token.is_some(),
            "Normalizing Bitbucket URL"
        );
        let normalized_url = self.normalize_url(url)?;
        tracing::debug!(
            original_url = %url,
            normalized_url = %normalized_url,
            "URL normalized"
        );

        let clone_url = if let Some(token) = token {
            // Bitbucket uses x-token-auth as username
            tracing::debug!("Adding Bitbucket token to URL (token masked)");
            normalized_url.replace("https://", &format!("https://x-token-auth:{token}@"))
        } else {
            tracing::debug!("No token provided, cloning without authentication");
            normalized_url
        };

        tracing::debug!(
            target_path = %target_path.display(),
            branch = ?branch,
            "Starting git clone operation"
        );
        clone_repository_impl(&clone_url, target_path, branch).await
    }
}

/// Azure DevOps provider implementation.
pub struct AzureDevOpsProvider;

impl AzureDevOpsProvider {
    /// Create a new Azure DevOps provider.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for AzureDevOpsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitProvider for AzureDevOpsProvider {
    fn name(&self) -> &'static str {
        "Azure DevOps"
    }

    fn can_handle(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        url_lower.contains("dev.azure.com")
            || url_lower.contains("visualstudio.com")
            || url_lower.contains("azure.com")
    }

    fn normalize_url(&self, url: &str) -> Result<String> {
        let url = url.trim();

        // Azure DevOps URLs are typically already in HTTPS format
        // https://dev.azure.com/org/project/_git/repo
        // https://org.visualstudio.com/project/_git/repo

        if url.starts_with("https://") {
            return Ok(url.to_string());
        }

        // SSH format: git@ssh.dev.azure.com:v3/org/project/repo
        if url.starts_with("git@ssh.dev.azure.com:") {
            let path = url.strip_prefix("git@ssh.dev.azure.com:v3/").unwrap();
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 3 {
                return Ok(format!(
                    "https://dev.azure.com/{}/{}/_git/{}",
                    parts[0], parts[1], parts[2]
                ));
            }
        }

        Err(MonPhareError::InvalidGitUrl {
            url: url.to_string(),
            message: "Could not parse Azure DevOps URL".to_string(),
        })
    }

    async fn clone_repo(
        &self,
        url: &str,
        target_path: &PathBuf,
        branch: Option<&str>,
        token: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(
            provider = "AzureDevOps",
            url = %url,
            target_path = %target_path.display(),
            branch = ?branch,
            has_token = token.is_some(),
            "Normalizing Azure DevOps URL"
        );
        let normalized_url = self.normalize_url(url)?;
        tracing::debug!(
            original_url = %url,
            normalized_url = %normalized_url,
            "URL normalized"
        );

        let clone_url = if let Some(token) = token {
            // Azure DevOps uses PAT with empty username or specific format
            tracing::debug!("Adding Azure DevOps PAT to URL (token masked)");
            normalized_url.replace("https://", &format!("https://ADO:{token}@"))
        } else {
            tracing::debug!("No token provided, cloning without authentication");
            normalized_url
        };

        tracing::debug!(
            target_path = %target_path.display(),
            branch = ?branch,
            "Starting git clone operation"
        );
        clone_repository_impl(&clone_url, target_path, branch).await
    }
}

/// Common repository cloning implementation using git2.
async fn clone_repository_impl(
    url: &str,
    target_path: &PathBuf,
    branch: Option<&str>,
) -> Result<()> {
    let url = url.to_string();
    let target_path = target_path.clone();
    let branch = branch.map(String::from);

    // Run git2 operations in a blocking task
    tokio::task::spawn_blocking(move || {
        let mut builder = git2::build::RepoBuilder::new();
        let mut fetch_options = git2::FetchOptions::new();

        fetch_options.depth(1); // Shallow clone for performance
        builder.fetch_options(fetch_options);

        // Set branch if specified
        if let Some(ref branch) = branch {
            builder.branch(branch);
        }

        tracing::debug!(url = %url, path = %target_path.display(), "Cloning repository");

        builder.clone(&url, &target_path).map_err(|e| {
            MonPhareError::GitClone {
                url: url.clone(),
                message: e.message().to_string(),
            }
        })?;

        Ok(())
    })
    .await
    .map_err(|e| MonPhareError::Internal {
        message: format!("Clone task failed: {e}"),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_url_normalization() {
        let provider = GitHubProvider::new();

        assert_eq!(
            provider.normalize_url("https://github.com/org/repo").unwrap(),
            "https://github.com/org/repo.git"
        );

        assert_eq!(
            provider
                .normalize_url("git@github.com:org/repo.git")
                .unwrap(),
            "https://github.com/org/repo.git"
        );

        assert_eq!(
            provider.normalize_url("github.com/org/repo").unwrap(),
            "https://github.com/org/repo.git"
        );
    }

    #[test]
    fn test_gitlab_url_normalization() {
        let provider = GitLabProvider::new();

        assert_eq!(
            provider.normalize_url("https://gitlab.com/org/repo").unwrap(),
            "https://gitlab.com/org/repo.git"
        );

        assert_eq!(
            provider
                .normalize_url("git@gitlab.com:org/repo.git")
                .unwrap(),
            "https://gitlab.com/org/repo.git"
        );
    }

    #[test]
    fn test_bitbucket_url_normalization() {
        let provider = BitbucketProvider::new();

        assert_eq!(
            provider
                .normalize_url("https://bitbucket.org/org/repo")
                .unwrap(),
            "https://bitbucket.org/org/repo.git"
        );
    }

    #[test]
    fn test_azure_devops_url_normalization() {
        let provider = AzureDevOpsProvider::new();

        assert_eq!(
            provider
                .normalize_url("https://dev.azure.com/org/project/_git/repo")
                .unwrap(),
            "https://dev.azure.com/org/project/_git/repo"
        );
    }

    #[test]
    fn test_provider_can_handle() {
        let github = GitHubProvider::new();
        let gitlab = GitLabProvider::new();
        let bitbucket = BitbucketProvider::new();
        let azure = AzureDevOpsProvider::new();

        assert!(github.can_handle("https://github.com/org/repo"));
        assert!(!github.can_handle("https://gitlab.com/org/repo"));

        assert!(gitlab.can_handle("https://gitlab.com/org/repo"));
        assert!(!gitlab.can_handle("https://github.com/org/repo"));

        assert!(bitbucket.can_handle("https://bitbucket.org/org/repo"));
        assert!(!bitbucket.can_handle("https://github.com/org/repo"));

        assert!(azure.can_handle("https://dev.azure.com/org/project/_git/repo"));
        assert!(!azure.can_handle("https://github.com/org/repo"));
    }
}

