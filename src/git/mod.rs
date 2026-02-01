//! Git provider abstraction layer.
//!
//! This module provides a unified interface for cloning repositories
//! from multiple Git providers:
//!
//! - GitHub
//! - GitLab
//! - Bitbucket
//! - Azure DevOps
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        GitClient                                │
//! │  - Unified interface for all providers                         │
//! │  - Handles authentication                                       │
//! │  - Manages temporary directories                                │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      GitProvider (trait)                        │
//! └─────────────────────────────────────────────────────────────────┘
//!          │              │              │              │
//!          ▼              ▼              ▼              ▼
//!     ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
//!     │ GitHub  │   │ GitLab  │   │Bitbucket│   │  Azure  │
//!     │Provider │   │Provider │   │Provider │   │ DevOps  │
//!     └─────────┘   └─────────┘   └─────────┘   └─────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use monphare::git::GitClient;
//! use monphare::Config;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::default();
//!     let client = GitClient::new(config);
//!     
//!     // Clone from any supported provider
//!     let path = client.clone_repository("https://github.com/org/repo").await?;
//!     println!("Cloned to: {}", path.display());
//!     
//!     Ok(())
//! }
//! ```

mod client;
mod providers;

pub use client::GitClient;
pub use providers::{
    AzureDevOpsProvider, BitbucketProvider, GitHubProvider, GitLabProvider, GitProvider,
};

/// Supported Git providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// GitHub (github.com)
    GitHub,
    /// GitLab (gitlab.com or self-hosted)
    GitLab,
    /// Bitbucket (bitbucket.org)
    Bitbucket,
    /// Azure DevOps (dev.azure.com)
    AzureDevOps,
    /// Unknown provider
    Unknown,
}

impl ProviderType {
    /// Detect the provider type from a URL.
    #[must_use]
    pub fn from_url(url: &str) -> Self {
        let url_lower = url.to_lowercase();

        if url_lower.contains("github.com") || url_lower.contains("github.") {
            Self::GitHub
        } else if url_lower.contains("gitlab.com") || url_lower.contains("gitlab.") {
            Self::GitLab
        } else if url_lower.contains("bitbucket.org") || url_lower.contains("bitbucket.") {
            Self::Bitbucket
        } else if url_lower.contains("dev.azure.com")
            || url_lower.contains("visualstudio.com")
            || url_lower.contains("azure.com")
        {
            Self::AzureDevOps
        } else {
            Self::Unknown
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub => write!(f, "GitHub"),
            Self::GitLab => write!(f, "GitLab"),
            Self::Bitbucket => write!(f, "Bitbucket"),
            Self::AzureDevOps => write!(f, "Azure DevOps"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection() {
        assert_eq!(
            ProviderType::from_url("https://github.com/org/repo"),
            ProviderType::GitHub
        );
        assert_eq!(
            ProviderType::from_url("git@github.com:org/repo.git"),
            ProviderType::GitHub
        );
        assert_eq!(
            ProviderType::from_url("https://gitlab.com/org/repo"),
            ProviderType::GitLab
        );
        assert_eq!(
            ProviderType::from_url("https://bitbucket.org/org/repo"),
            ProviderType::Bitbucket
        );
        assert_eq!(
            ProviderType::from_url("https://dev.azure.com/org/project/_git/repo"),
            ProviderType::AzureDevOps
        );
        assert_eq!(
            ProviderType::from_url("https://example.com/repo"),
            ProviderType::Unknown
        );
    }
}

