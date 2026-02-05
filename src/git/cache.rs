//! Repository cache manager.
//!
//! Caches cloned repositories to avoid unnecessary re-cloning.
//! Uses `git fetch` to check for updates instead of fresh clones.

use crate::config::CacheOptions;
use crate::error::{MonPhareError, Result};
use std::path::{Path, PathBuf};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Cache entry metadata stored in a `.monphare-cache` file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    /// Original repository URL
    pub url: String,
    /// Current HEAD commit SHA
    pub head_sha: String,
    /// Branch that was checked out
    pub branch: Option<String>,
    /// Timestamp when the cache was last updated (Unix epoch seconds)
    pub last_updated: u64,
    /// Timestamp when the cache was last accessed (Unix epoch seconds)
    pub last_accessed: u64,
}

/// Result of a cache operation.
#[derive(Debug)]
pub enum CacheResult {
    /// Cache hit - repository exists and is up to date
    Hit {
        path: PathBuf,
        sha: String,
    },
    /// Cache hit but repository was updated (fetched new changes)
    Updated {
        path: PathBuf,
        old_sha: String,
        new_sha: String,
    },
    /// Cache miss - repository was freshly cloned
    Miss {
        path: PathBuf,
        sha: String,
    },
}

impl CacheResult {
    /// Get the path to the cached repository.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            CacheResult::Hit { path, .. }
            | CacheResult::Updated { path, .. }
            | CacheResult::Miss { path, .. } => path,
        }
    }

    /// Check if the cache had changes (was updated or newly cloned).
    #[must_use]
    pub fn had_changes(&self) -> bool {
        !matches!(self, CacheResult::Hit { .. })
    }
}

/// Repository cache manager.
pub struct CacheManager {
    /// Cache directory
    cache_dir: PathBuf,
    /// Whether caching is enabled
    enabled: bool,
    /// Cache TTL in seconds
    ttl_seconds: u64,
    /// Fresh threshold in seconds (skip fetch if cache is newer than this)
    fresh_threshold_seconds: u64,
}

impl CacheManager {
    /// Create a new cache manager.
    #[must_use]
    pub fn new(options: &CacheOptions) -> Self {
        Self {
            cache_dir: options.get_cache_dir(),
            enabled: options.enabled,
            ttl_seconds: options.ttl_hours * 3600,
            fresh_threshold_seconds: options.fresh_threshold_minutes * 60,
        }
    }

    /// Check if caching is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the fresh threshold in seconds.
    #[must_use]
    pub fn fresh_threshold_seconds(&self) -> u64 {
        self.fresh_threshold_seconds
    }

    /// Get the cache directory.
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Generate a cache key (directory name) for a repository URL.
    #[must_use]
    pub fn cache_key(&self, url: &str) -> String {
        // Extract a readable name from the URL
        let readable_name = extract_repo_name(url);
        
        // Add a short hash to ensure uniqueness for similar URLs
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        let short_hash = format!("{:08x}", hash & 0xFFFFFFFF);
        
        format!("{}-{}", readable_name, short_hash)
    }

    /// Get the path where a repository would be cached.
    #[must_use]
    pub fn get_cache_path(&self, url: &str) -> PathBuf {
        self.cache_dir.join(self.cache_key(url))
    }

    /// Check if a cached repository exists and is valid.
    pub async fn get_cached(&self, url: &str) -> Option<CacheEntry> {
        if !self.enabled {
            return None;
        }

        let cache_path = self.get_cache_path(url);
        let meta_path = cache_path.join(".monphare-cache");

        if !cache_path.exists() || !meta_path.exists() {
            return None;
        }

        // Read the cache metadata
        let content = tokio::fs::read_to_string(&meta_path).await.ok()?;
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;

        // Check if the cache has expired
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now - entry.last_updated > self.ttl_seconds {
            tracing::debug!(
                url = %url,
                age_hours = (now - entry.last_updated) / 3600,
                ttl_hours = self.ttl_seconds / 3600,
                "Cache entry expired"
            );
            // Don't return None - we can still use the cache but should refresh
            // The caller will decide whether to fetch or not
        }

        Some(entry)
    }

    /// Update the cache metadata after a successful clone/fetch.
    pub async fn update_cache_entry(
        &self,
        url: &str,
        head_sha: &str,
        branch: Option<&str>,
    ) -> Result<()> {
        let cache_path = self.get_cache_path(url);
        let meta_path = cache_path.join(".monphare-cache");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = CacheEntry {
            url: url.to_string(),
            head_sha: head_sha.to_string(),
            branch: branch.map(String::from),
            last_updated: now,
            last_accessed: now,
        };

        let content = serde_json::to_string_pretty(&entry).map_err(|e| {
            crate::err!(Git {
                message: format!("Failed to serialize cache entry: {}", e),
            })
        })?;

        tokio::fs::write(&meta_path, content)
            .await
            .map_err(|e| MonPhareError::io(&meta_path, e, file!(), line!()))?;

        Ok(())
    }

    /// Update the last accessed timestamp.
    pub async fn touch_cache_entry(&self, url: &str) -> Result<()> {
        let cache_path = self.get_cache_path(url);
        let meta_path = cache_path.join(".monphare-cache");

        if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
            if let Ok(mut entry) = serde_json::from_str::<CacheEntry>(&content) {
                entry.last_accessed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let updated = serde_json::to_string_pretty(&entry).unwrap_or(content);
                let _ = tokio::fs::write(&meta_path, updated).await;
            }
        }

        Ok(())
    }

    /// Refresh the cache entry timestamps after a successful fetch check.
    /// Updates both last_accessed and last_updated to current time.
    pub async fn refresh_cache_entry(&self, url: &str) -> Result<()> {
        let cache_path = self.get_cache_path(url);
        let meta_path = cache_path.join(".monphare-cache");

        if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
            if let Ok(mut entry) = serde_json::from_str::<CacheEntry>(&content) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                entry.last_accessed = now;
                entry.last_updated = now;

                let updated = serde_json::to_string_pretty(&entry).unwrap_or(content);
                let _ = tokio::fs::write(&meta_path, updated).await;
            }
        }

        Ok(())
    }

    /// Ensure the cache directory exists.
    pub async fn ensure_cache_dir(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            tokio::fs::create_dir_all(&self.cache_dir)
                .await
                .map_err(|e| MonPhareError::io(&self.cache_dir, e, file!(), line!()))?;
        }
        Ok(())
    }

    /// Get the current HEAD SHA for a cached repository using git.
    pub async fn get_head_sha(&self, repo_path: &Path) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| crate::err!(Git {
                message: format!("Failed to get HEAD SHA: {}", e),
            }))?;

        if !output.status.success() {
            return Err(crate::err!(Git {
                message: format!(
                    "git rev-parse failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            }));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Fetch updates from the remote repository.
    /// Returns the new HEAD SHA after fetch.
    pub async fn fetch_updates(&self, repo_path: &Path, branch: Option<&str>) -> Result<String> {
        tracing::debug!(path = %repo_path.display(), "Fetching updates from remote");

        // Fetch from origin
        let mut args = vec!["fetch", "origin", "--depth=1"];
        if let Some(b) = branch {
            args.push(b);
        }

        let output = tokio::process::Command::new("git")
            .args(&args)
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| crate::err!(Git {
                message: format!("Failed to fetch: {}", e),
            }))?;

        if !output.status.success() {
            return Err(crate::err!(Git {
                message: format!(
                    "git fetch failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            }));
        }

        // Reset to origin/HEAD or origin/{branch}
        let remote_ref = branch
            .map(|b| format!("origin/{}", b))
            .unwrap_or_else(|| "origin/HEAD".to_string());

        let output = tokio::process::Command::new("git")
            .args(["reset", "--hard", &remote_ref])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| crate::err!(Git {
                message: format!("Failed to reset: {}", e),
            }))?;

        if !output.status.success() {
            // Try with FETCH_HEAD if origin/HEAD fails
            let output = tokio::process::Command::new("git")
                .args(["reset", "--hard", "FETCH_HEAD"])
                .current_dir(repo_path)
                .output()
                .await
                .map_err(|e| crate::err!(Git {
                    message: format!("Failed to reset to FETCH_HEAD: {}", e),
                }))?;

            if !output.status.success() {
                return Err(crate::err!(Git {
                    message: format!(
                        "git reset failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                }));
            }
        }

        // Get the new HEAD SHA
        self.get_head_sha(repo_path).await
    }

    /// Clean old cache entries that exceed the maximum size.
    pub async fn cleanup_old_entries(&self, max_entries: usize) -> Result<usize> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut entries: Vec<(PathBuf, u64)> = Vec::new();

        // Collect all cache entries with their last accessed time
        let mut dir = tokio::fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| MonPhareError::io(&self.cache_dir, e, file!(), line!()))?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| {
            MonPhareError::io(&self.cache_dir, e, file!(), line!())
        })? {
            let path = entry.path();
            if path.is_dir() {
                let meta_path = path.join(".monphare-cache");
                if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
                    if let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&content) {
                        entries.push((path, cache_entry.last_accessed));
                    }
                }
            }
        }

        // Sort by last accessed (oldest first)
        entries.sort_by_key(|(_, accessed)| *accessed);

        // Remove oldest entries until we're under the limit
        let mut removed = 0;
        while entries.len() > max_entries {
            if let Some((path, _)) = entries.first() {
                tracing::info!(path = %path.display(), "Removing old cache entry");
                if tokio::fs::remove_dir_all(path).await.is_ok() {
                    removed += 1;
                }
                entries.remove(0);
            } else {
                break;
            }
        }

        Ok(removed)
    }
}

/// Extract a readable repository name from a URL.
fn extract_repo_name(url: &str) -> String {
    // Remove common prefixes and suffixes
    let url = url
        .trim_end_matches('/')
        .trim_end_matches(".git");

    // Try to extract org/repo from various URL formats
    if let Some(rest) = url.strip_prefix("git@") {
        // git@github.com:org/repo
        if let Some((_host, path)) = rest.split_once(':') {
            return path.replace('/', "-");
        }
    }

    // https://github.com/org/repo or similar
    if let Some(idx) = url.find("://") {
        let path_part = &url[idx + 3..];
        if let Some(slash_idx) = path_part.find('/') {
            let path = &path_part[slash_idx + 1..];
            // Handle Azure DevOps special paths
            let path = path
                .replace("/_git/", "-")
                .replace("/", "-")
                .replace("_", "-");
            return path;
        }
    }

    // Fallback: use hash of URL
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("repo-{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_github() {
        assert_eq!(
            extract_repo_name("https://github.com/hashicorp/terraform"),
            "hashicorp-terraform"
        );
        assert_eq!(
            extract_repo_name("https://github.com/hashicorp/terraform.git"),
            "hashicorp-terraform"
        );
    }

    #[test]
    fn test_extract_repo_name_gitlab() {
        assert_eq!(
            extract_repo_name("https://gitlab.com/org/project"),
            "org-project"
        );
    }

    #[test]
    fn test_extract_repo_name_azure_devops() {
        assert_eq!(
            extract_repo_name("https://dev.azure.com/org/project/_git/repo"),
            "org-project-repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:hashicorp/terraform.git"),
            "hashicorp-terraform"
        );
    }

    #[test]
    fn test_cache_key_uniqueness() {
        let options = CacheOptions::default();
        let manager = CacheManager::new(&options);

        let key1 = manager.cache_key("https://github.com/org/repo1");
        let key2 = manager.cache_key("https://github.com/org/repo2");

        assert_ne!(key1, key2);
        assert!(key1.starts_with("org-repo1-"));
        assert!(key2.starts_with("org-repo2-"));
    }
}
