//! Module source parsing utilities.
//!
//! This module handles parsing of Terraform module source strings into
//! structured `ModuleSource` types.
//!
//! # Supported Source Types
//!
//! - **Registry**: `namespace/name/provider` or `hostname/namespace/name/provider`
//! - **Git**: `git::https://...` or `git@github.com:...`
//! - **HTTP**: `https://...` (archive downloads)
//! - **S3**: `s3::https://...` or `s3://bucket/key`
//! - **GCS**: `gcs::https://...`
//! - **Local**: `./path` or `../path`

use crate::error::Result;
use crate::types::ModuleSource;
use regex::Regex;
use std::sync::LazyLock;

/// Default Terraform registry hostname.
const DEFAULT_REGISTRY: &str = "registry.terraform.io";

// Regex patterns for parsing sources
static REGISTRY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches: namespace/name/provider or hostname/namespace/name/provider
    Regex::new(r"^(?:([a-zA-Z0-9.-]+)/)?([a-zA-Z0-9_-]+)/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_-]+)$")
        .expect("Invalid regex")
});

static GIT_URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches git:: prefix URLs with optional ref and subdir
    // git::https://github.com/example/module.git?ref=v1.0.0//modules/vpc
    // Capture groups: 1=url, 2=ref, 3=subdir
    Regex::new(r"^git::(.+?\.git)(?:\?ref=([^/]+))?(?://(.+))?$").expect("Invalid regex")
});

static GIT_SSH_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches git@host:path format
    Regex::new(r"^git@([^:]+):(.+?)(?:\.git)?(?:\?ref=([^/]+))?(?://(.+))?$").expect("Invalid regex")
});

static GITHUB_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches github.com URLs without git:: prefix
    Regex::new(r"^(?:https?://)?github\.com/([^/]+)/([^/?]+)(?:\.git)?(?:\?ref=([^/]+))?(?://(.+))?$")
        .expect("Invalid regex")
});

static S3_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches s3:: prefix or s3:// URLs
    Regex::new(r"^s3::https://s3(?:-([a-z0-9-]+))?\.amazonaws\.com/([^/]+)/(.+)$|^s3://([^/]+)/(.+)$")
        .expect("Invalid regex")
});

static GCS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Matches gcs:: prefix URLs
    Regex::new(r"^gcs::https://(?:www\.)?googleapis\.com/storage/v1/([^/]+)/(.+)$")
        .expect("Invalid regex")
});

/// Parse a module source string into a structured `ModuleSource`.
///
/// # Examples
///
/// ```rust
/// use kanongraph::parser::parse_module_source;
/// use kanongraph::types::ModuleSource;
///
/// // Registry source
/// let source = parse_module_source("hashicorp/consul/aws").unwrap();
/// assert!(matches!(source, ModuleSource::Registry { .. }));
///
/// // Git source
/// let source = parse_module_source("git::https://github.com/example/module.git").unwrap();
/// assert!(matches!(source, ModuleSource::Git { .. }));
///
/// // Local source
/// let source = parse_module_source("../modules/vpc").unwrap();
/// assert!(matches!(source, ModuleSource::Local { .. }));
/// ```
///
/// # Errors
///
/// Returns an error if the source string cannot be parsed.
pub fn parse_module_source(source: &str) -> Result<ModuleSource> {
    let source = source.trim();

    // Check for local paths first
    if is_local_path(source) {
        return Ok(ModuleSource::Local {
            path: source.to_string(),
        });
    }

    // Check for Git sources
    if let Some(git_source) = try_parse_git_source(source) {
        return Ok(git_source);
    }

    // Check for S3 sources
    if let Some(s3_source) = try_parse_s3_source(source) {
        return Ok(s3_source);
    }

    // Check for GCS sources
    if let Some(gcs_source) = try_parse_gcs_source(source) {
        return Ok(gcs_source);
    }

    // Check for HTTP sources
    if source.starts_with("http://") || source.starts_with("https://") {
        // Could be a registry with custom hostname or HTTP archive
        if let Some(registry_source) = try_parse_registry_url(source) {
            return Ok(registry_source);
        }
        return Ok(ModuleSource::Http {
            url: source.to_string(),
        });
    }

    // Try registry format (namespace/name/provider)
    if let Some(registry_source) = try_parse_registry_source(source) {
        return Ok(registry_source);
    }

    // Unknown source format
    tracing::warn!(source = %source, "Unknown module source format");
    Ok(ModuleSource::Unknown(source.to_string()))
}

/// Check if a path is a local file path.
fn is_local_path(source: &str) -> bool {
    source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with('/')
        || source.starts_with('~')
        || (source.len() >= 2 && source.chars().nth(1) == Some(':')) // Windows paths
}

/// Try to parse a Git source.
fn try_parse_git_source(source: &str) -> Option<ModuleSource> {
    // git:: prefix
    if let Some(caps) = GIT_URL_PATTERN.captures(source) {
        let url = caps.get(1)?.as_str().to_string();
        let ref_ = caps.get(2).map(|m| m.as_str().to_string());
        let subdir = caps.get(3).map(|m| m.as_str().to_string());

        return Some(ModuleSource::Git { host: format!("git::{}", url.clone()), url, ref_, subdir });
    }

    // git@host:path format
    if let Some(caps) = GIT_SSH_PATTERN.captures(source) {
        let host = caps.get(1)?.as_str();
        let path = caps.get(2)?.as_str();
        let url = format!("ssh://git@{host}/{path}");
        let ref_ = caps.get(3).map(|m| m.as_str().to_string());
        let subdir = caps.get(4).map(|m| m.as_str().to_string());

        tracing::debug!(url = &url, ref_ = &ref_, subdir = subdir.as_deref().unwrap_or_default() , "Parsed Git SSH source");
        return Some(ModuleSource::Git { host: host.to_string(), url, ref_, subdir });
    }

    // GitHub shorthand
    if let Some(caps) = GITHUB_PATTERN.captures(source) {
        let owner = caps.get(1)?.as_str();
        let repo = caps.get(2)?.as_str();
        let url = format!("https://github.com/{owner}/{repo}.git");
        let ref_ = caps.get(3).map(|m| m.as_str().to_string());
        let subdir = caps.get(4).map(|m| m.as_str().to_string());
        return Some(ModuleSource::Git { host: format!("github.com/{owner}/{repo}.git"), url, ref_, subdir });
    }

    None
}

/// Try to parse an S3 source.
fn try_parse_s3_source(source: &str) -> Option<ModuleSource> {
    if let Some(caps) = S3_PATTERN.captures(source) {
        // s3:: format
        if let (Some(bucket), Some(key)) = (caps.get(2), caps.get(3)) {
            let region = caps.get(1).map(|m| m.as_str().to_string());
            return Some(ModuleSource::S3 {
                bucket: bucket.as_str().to_string(),
                key: key.as_str().to_string(),
                region,
            });
        }
        // s3:// format
        if let (Some(bucket), Some(key)) = (caps.get(4), caps.get(5)) {
            return Some(ModuleSource::S3 {
                bucket: bucket.as_str().to_string(),
                key: key.as_str().to_string(),
                region: None,
            });
        }
    }
    None
}

/// Try to parse a GCS source.
fn try_parse_gcs_source(source: &str) -> Option<ModuleSource> {
    if let Some(caps) = GCS_PATTERN.captures(source) {
        let bucket = caps.get(1)?.as_str().to_string();
        let path = caps.get(2)?.as_str().to_string();
        return Some(ModuleSource::Gcs { bucket, path });
    }
    None
}

/// Try to parse a Terraform Registry source.
fn try_parse_registry_source(source: &str) -> Option<ModuleSource> {
    if let Some(caps) = REGISTRY_PATTERN.captures(source) {
        // Four-part: hostname/namespace/name/provider
        // Three-part: namespace/name/provider (uses default registry)
        let (hostname, namespace, name, provider) = if caps.get(1).is_some() {
            (
                caps.get(1)?.as_str().to_string(),
                caps.get(2)?.as_str().to_string(),
                caps.get(3)?.as_str().to_string(),
                caps.get(4)?.as_str().to_string(),
            )
        } else {
            (
                DEFAULT_REGISTRY.to_string(),
                caps.get(2)?.as_str().to_string(),
                caps.get(3)?.as_str().to_string(),
                caps.get(4)?.as_str().to_string(),
            )
        };

        return Some(ModuleSource::Registry {
            hostname,
            namespace,
            name,
            provider,
        });
    }
    None
}

/// Try to parse a registry URL (e.g., https://registry.terraform.io/...).
fn try_parse_registry_url(source: &str) -> Option<ModuleSource> {
    // Parse URL and check if it looks like a registry
    let url = url::Url::parse(source).ok()?;
    let host = url.host_str()?;

    // Check if it's a known registry host
    if host.contains("registry") || host == "app.terraform.io" {
        let path_segments: Vec<&str> = url.path().trim_matches('/').split('/').collect();

        // Expected format: /modules/namespace/name/provider
        // or: /namespace/name/provider
        if path_segments.len() >= 3 {
            let offset = if path_segments[0] == "modules" { 1 } else { 0 };
            if path_segments.len() >= offset + 3 {
                return Some(ModuleSource::Registry {
                    hostname: host.to_string(),
                    namespace: path_segments[offset].to_string(),
                    name: path_segments[offset + 1].to_string(),
                    provider: path_segments[offset + 2].to_string(),
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry_source() {
        let source = parse_module_source("hashicorp/consul/aws").unwrap();
        match source {
            ModuleSource::Registry {
                hostname,
                namespace,
                name,
                provider,
            } => {
                assert_eq!(hostname, "registry.terraform.io");
                assert_eq!(namespace, "hashicorp");
                assert_eq!(name, "consul");
                assert_eq!(provider, "aws");
            }
            _ => panic!("Expected Registry source"),
        }
    }

    #[test]
    fn test_parse_registry_with_hostname() {
        let source = parse_module_source("app.terraform.io/my-org/vpc/aws").unwrap();
        match source {
            ModuleSource::Registry {
                hostname,
                namespace,
                ..
            } => {
                assert_eq!(hostname, "app.terraform.io");
                assert_eq!(namespace, "my-org");
            }
            _ => panic!("Expected Registry source"),
        }
    }

    #[test]
    fn test_parse_git_https_source() {
        let source = parse_module_source("git::https://github.com/example/module.git").unwrap();
        match source {
            ModuleSource::Git { host, url, ref_, subdir } => {
                assert_eq!(url, "https://github.com/example/module.git");
                assert_eq!(host, "github.com/example/module.git");
                assert!(ref_.is_none());
                assert!(subdir.is_none());
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_parse_git_with_ref() {
        let source =
            parse_module_source("git::https://github.com/example/module.git?ref=v1.0.0").unwrap();
        match source {
            ModuleSource::Git { ref_, .. } => {
                assert_eq!(ref_.as_deref(), Some("v1.0.0"));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_parse_git_with_subdir() {
        let source =
            parse_module_source("git::https://github.com/example/module.git//modules/vpc").unwrap();
        match source {
            ModuleSource::Git { subdir, .. } => {
                assert_eq!(subdir.as_deref(), Some("modules/vpc"));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_parse_git_ssh_source() {
        let source = parse_module_source("git@github.com:example/module.git").unwrap();
        match source {
            ModuleSource::Git { host, url, .. } => {
                assert_eq!(host, "github.com/example/module.git");
                assert!(url.contains("github.com"));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_parse_local_source_relative() {
        let source = parse_module_source("../modules/vpc").unwrap();
        match source {
            ModuleSource::Local { path } => {
                assert_eq!(path, "../modules/vpc");
            }
            _ => panic!("Expected Local source"),
        }
    }

    #[test]
    fn test_parse_local_source_current_dir() {
        let source = parse_module_source("./modules/vpc").unwrap();
        match source {
            ModuleSource::Local { path } => {
                assert_eq!(path, "./modules/vpc");
            }
            _ => panic!("Expected Local source"),
        }
    }

    #[test]
    fn test_parse_local_source_absolute() {
        let source = parse_module_source("/opt/terraform/modules/vpc").unwrap();
        match source {
            ModuleSource::Local { path } => {
                assert_eq!(path, "/opt/terraform/modules/vpc");
            }
            _ => panic!("Expected Local source"),
        }
    }

    #[test]
    fn test_parse_http_source() {
        let source = parse_module_source("https://example.com/module.zip").unwrap();
        match source {
            ModuleSource::Http { url } => {
                assert_eq!(url, "https://example.com/module.zip");
            }
            _ => panic!("Expected Http source"),
        }
    }

    #[test]
    fn test_parse_s3_source() {
        let source = parse_module_source("s3://my-bucket/modules/vpc.zip").unwrap();
        match source {
            ModuleSource::S3 { bucket, key, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(key, "modules/vpc.zip");
            }
            _ => panic!("Expected S3 source"),
        }
    }

    #[test]
    fn test_parse_github_shorthand() {
        let source = parse_module_source("github.com/example/terraform-module").unwrap();
        match source {
            ModuleSource::Git { url, .. } => {
                assert!(url.contains("github.com"));
                assert!(url.contains("example/terraform-module"));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_canonical_id_registry() {
        let source = ModuleSource::Registry {
            hostname: "registry.terraform.io".to_string(),
            namespace: "hashicorp".to_string(),
            name: "consul".to_string(),
            provider: "aws".to_string(),
        };
        assert_eq!(
            source.canonical_id(),
            "registry.terraform.io/hashicorp/consul/aws"
        );
    }

    #[test]
    fn test_canonical_id_local() {
        let source = ModuleSource::Local {
            path: "../modules/vpc".to_string(),
        };
        assert_eq!(source.canonical_id(), "local://../modules/vpc");
    }
}

