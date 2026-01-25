//! VCS (Version Control System) support for bulk organization scanning.
//!
//! This module provides functionality for:
//! - Discovering repositories from VCS platforms (GitHub, GitLab, Azure DevOps, Bitbucket)
//! - Platform-agnostic VCS identifiers for tracking module ownership
//! - Token management with auto-detection and configuration override
//! - Rate limiting and caching for API operations

use serde::{Deserialize, Serialize};

/// Platform-agnostic VCS identifier using canonical format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VcsIdentifier {
    /// Canonical format: "vcs:platform:namespace1/namespace2/repo"
    /// Examples:
    /// - "vcs:github:hashicorp/terraform"
    /// - "vcs:gitlab:gitlab-org/gitlab"
    /// - "vcs:ado:myorg/myproject/myrepo"
    /// - "vcs:bitbucket:atlassian/bitbucket"
    /// - "vcs:local" (for local filesystem modules)
    pub canonical: String,

    /// Parsed components for easy access
    pub components: Vec<String>,
}

impl VcsIdentifier {
    /// Create a new VCS identifier from platform and namespace components
    #[must_use]
    pub fn new(platform: &str, components: &[&str]) -> Self {
        let canonical = format!("vcs:{}:{}", platform, components.join("/"));
        let components = components.iter().map(|s| s.to_string()).collect();

        Self {
            canonical,
            components,
        }
    }

    /// Create a local filesystem identifier
    #[must_use]
    pub fn local() -> Self {
        Self {
            canonical: "vcs:local".to_string(),
            components: vec!["local".to_string()],
        }
    }

    /// Parse VCS identifier from a canonical string
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not in the expected format
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 2 || parts[0] != "vcs" {
            return Err(format!("Invalid VCS identifier format: {}", s));
        }

        let platform = parts[1];
        let namespace_part = parts.get(2).copied().unwrap_or("");

        let components: Vec<String> = if namespace_part.is_empty() {
            vec![]
        } else {
            namespace_part.split('/').map(|s| s.to_string()).collect()
        };

        Ok(Self {
            canonical: s.to_string(),
            components,
        })
    }

    /// Get the platform (github, gitlab, ado, bitbucket, local)
    #[must_use]
    pub fn platform(&self) -> &str {
        self.canonical
            .strip_prefix("vcs:")
            .and_then(|s| s.split(':').next())
            .unwrap_or("unknown")
    }

    /// Get the namespace path (everything after platform)
    #[must_use]
    pub fn namespace(&self) -> &str {
        self.canonical
            .splitn(3, ':')
            .nth(2)
            .unwrap_or("")
    }

    /// Check if this represents a local module
    #[must_use]
    pub fn is_local(&self) -> bool {
        self.platform() == "local"
    }

    /// Check if this represents a VCS-hosted module
    #[must_use]
    pub fn is_vcs(&self) -> bool {
        !self.is_local()
    }
}

impl std::fmt::Display for VcsIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical)
    }
}

/// Supported VCS platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsPlatform {
    /// GitHub
    GitHub,
    /// GitLab
    GitLab,
    /// Azure DevOps
    AzureDevOps,
    /// Bitbucket
    Bitbucket,
    /// Local filesystem
    Local,
}

impl VcsPlatform {
    /// Get the platform name as a string
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
            Self::AzureDevOps => "ado",
            Self::Bitbucket => "bitbucket",
            Self::Local => "local",
        }
    }

    /// Parse platform from string
    ///
    /// # Errors
    ///
    /// Returns an error if the platform is not recognized
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "github" => Ok(Self::GitHub),
            "gitlab" => Ok(Self::GitLab),
            "ado" | "azure" | "azure-devops" => Ok(Self::AzureDevOps),
            "bitbucket" => Ok(Self::Bitbucket),
            "local" => Ok(Self::Local),
            _ => Err(format!("Unknown VCS platform: {}", s)),
        }
    }
}

/// Repository information returned by VCS APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsRepository {
    /// Repository name
    pub name: String,
    /// Full repository URL for cloning
    pub clone_url: String,
    /// Default branch name (e.g., "main", "master")
    pub default_branch: String,
    /// Whether the repository is archived (read-only)
    pub archived: bool,
    /// Whether the repository is a fork
    pub fork: bool,
    /// Platform-specific identifier
    pub platform_id: String,
}

/// Token configuration for VCS platforms
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VcsTokens {
    /// GitHub personal access token
    pub github: Option<String>,
    /// GitLab personal/group access token
    pub gitlab: Option<String>,
    /// Azure DevOps personal access token
    pub azure_devops: Option<String>,
    /// Bitbucket app password
    pub bitbucket: Option<String>,
}

impl VcsTokens {
    /// Get token for a specific platform
    #[must_use]
    pub fn get(&self, platform: VcsPlatform) -> Option<&str> {
        match platform {
            VcsPlatform::GitHub => self.github.as_deref(),
            VcsPlatform::GitLab => self.gitlab.as_deref(),
            VcsPlatform::AzureDevOps => self.azure_devops.as_deref(),
            VcsPlatform::Bitbucket => self.bitbucket.as_deref(),
            VcsPlatform::Local => None,
        }
    }

    /// Set token for a specific platform
    pub fn set(&mut self, platform: VcsPlatform, token: String) {
        match platform {
            VcsPlatform::GitHub => self.github = Some(token),
            VcsPlatform::GitLab => self.gitlab = Some(token),
            VcsPlatform::AzureDevOps => self.azure_devops = Some(token),
            VcsPlatform::Bitbucket => self.bitbucket = Some(token),
            VcsPlatform::Local => {} // No token needed for local
        }
    }
}

/// VCS client trait for platform-specific operations
#[async_trait::async_trait]
pub trait VcsClient: Send + Sync {
    /// Get the platform this client handles
    fn platform(&self) -> VcsPlatform;

    /// Discover all repositories in an organization/group
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails or authentication is invalid
    async fn discover_repositories(
        &self,
        org: &str,
        token: &str,
    ) -> crate::Result<Vec<VcsRepository>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcs_identifier_new() {
        let id = VcsIdentifier::new("github", &["hashicorp", "terraform"]);
        assert_eq!(id.canonical, "vcs:github:hashicorp/terraform");
        assert_eq!(id.components, vec!["hashicorp", "terraform"]);
        assert_eq!(id.platform(), "github");
        assert_eq!(id.namespace(), "hashicorp/terraform");
    }

    #[test]
    fn test_vcs_identifier_local() {
        let id = VcsIdentifier::local();
        assert_eq!(id.canonical, "vcs:local");
        assert!(id.is_local());
        assert!(!id.is_vcs());
    }

    #[test]
    fn test_vcs_identifier_parse() {
        let id = VcsIdentifier::parse("vcs:github:hashicorp/terraform").unwrap();
        assert_eq!(id.platform(), "github");
        assert_eq!(id.components, vec!["hashicorp", "terraform"]);

        let local = VcsIdentifier::parse("vcs:local").unwrap();
        assert!(local.is_local());
    }

    #[test]
    fn test_vcs_identifier_parse_invalid() {
        assert!(VcsIdentifier::parse("invalid").is_err());
        assert!(VcsIdentifier::parse("not-vcs:format").is_err());
    }

    #[test]
    fn test_platform_from_str() {
        assert_eq!(VcsPlatform::from_str("github").unwrap(), VcsPlatform::GitHub);
        assert_eq!(VcsPlatform::from_str("ado").unwrap(), VcsPlatform::AzureDevOps);
        assert_eq!(VcsPlatform::from_str("AZURE").unwrap(), VcsPlatform::AzureDevOps);
        assert!(VcsPlatform::from_str("unknown").is_err());
    }
}
