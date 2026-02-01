//! VCS API clients for bulk repository discovery.
//!
//! This module provides HTTP clients for different VCS platforms
//! to discover repositories under organizations/groups.

use crate::error::{MonPhareError, Result, ResultExt};
use crate::error::MonPhareError::VcsApi;
use crate::vcs::{VcsClient, VcsPlatform, VcsRepository};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use std::{fs, process};
use dirs;
use base64::engine::{Engine as _, general_purpose::STANDARD};


/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_concurrent: usize,
    pub delay_ms: u64,
    pub max_retries: usize,
    pub backoff_multiplier: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            delay_ms: 1000, // 1 second
            max_retries: 3,
            backoff_multiplier: 2.0,
        }
    }
}

/// HTTP client wrapper with rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitedClient {
    client: Client,
    config: RateLimitConfig,
}

impl RateLimitedClient {
    pub fn new(config: RateLimitConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("monphare/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    pub async fn get(&self, url: &str, token: Option<&str>) -> crate::Result<reqwest::Response> {
        self.request(reqwest::Method::GET, url, token).await
    }

    async fn request(&self, method: reqwest::Method, url: &str, token: Option<&str>) -> crate::Result<reqwest::Response> {
        let mut attempts = 0;
        let mut delay = self.config.delay_ms;

        loop {
            attempts += 1;
            let mut request = self.client.request(method.clone(), url);
            if let Some(t) = token {
                request = request.header("Authorization", format!("token {}", t)); // GitHub
            }

            let response = request.send().await.map_err(|e| crate::err!(VcsApi {
                platform: "unknown".to_string(), // Placeholder, will be updated by specific clients
                message: format!("HTTP request failed for {}: {}", url, e),
            }))?;

            if response.status().is_success() {
                return Ok(response);
            }

            let status = response.status();
            if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempts <= self.config.max_retries {
                    tracing::warn!("Request to {} failed with status {} (attempt {}/{}), retrying in {}ms",
                                   url, status, attempts, self.config.max_retries, delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    delay = (delay as f64 * self.config.backoff_multiplier) as u64;
                    continue;
                }
            }

            return Err(crate::err!(VcsApi {
                platform: "unknown".to_string(),
                message: format!("HTTP request failed for {}: Status {}", url, status),
            }));
        }
    }
}

impl Default for RateLimitedClient {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

/// GitHub API client
pub struct GitHubClient {
    client: CachedRateLimitedClient,
    api_base_url: String,
}

impl GitHubClient {
    pub fn new(client: CachedRateLimitedClient) -> Self {
        Self {
            client,
            api_base_url: "https://api.github.com".to_string(),
        }
    }

    #[cfg(test)]
    fn with_api_base_url(mut self, api_base_url: String) -> Self {
        self.api_base_url = api_base_url.trim_end_matches('/').to_string();
        self
    }

    pub fn with_cache_disabled(mut self) -> Self {
        self.client.cache_mut().enabled = false;
        self
    }
}

#[async_trait]
impl VcsClient for GitHubClient {
    fn platform(&self) -> VcsPlatform {
        VcsPlatform::GitHub
    }

    async fn discover_repositories(
        &self,
        org: &str,
        token: &str,
    ) -> crate::Result<Vec<VcsRepository>> {
        // Check cache first
        if let Some(cached_repos) = self.client.cache().load("github", org)? {
            tracing::debug!("Using cached repository list for GitHub org {}", org);
            return Ok(cached_repos);
        }

        tracing::debug!("Fetching repository list for GitHub org {}", org);
        let mut repos = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url = format!(
                "{}/orgs/{}/repos?page={}&per_page={}",
                self.api_base_url, org, page, per_page
            );

            let mut request = self.client.client().client().request(reqwest::Method::GET, &url);
            request = request.header("Authorization", format!("token {}", token));

            let response = request.send().await.map_err(|e| crate::err!(VcsApi {
                platform: "github".to_string(),
                message: format!("Failed to fetch GitHub repositories: {}", e),
            }))?;

            if !response.status().is_success() {
                return Err(crate::err!(VcsApi {
                    platform: "github".to_string(),
                    message: format!("GitHub API error: Status {}", response.status()),
                }));
            }

            let page_repos: Vec<GitHubRepository> = response.json().await.map_err(|e| crate::err!(VcsApi {
                platform: "github".to_string(),
                message: format!("Failed to parse GitHub repositories: {}", e),
            }))?;

            let page_len = page_repos.len();
            if page_repos.is_empty() {
                break;
            }

            repos.extend(page_repos.into_iter().map(|r| VcsRepository {
                name: r.name,
                clone_url: r.clone_url,
                default_branch: r.default_branch,
                archived: r.archived,
                fork: r.fork,
                platform_id: r.id.to_string(),
            }));

            page += 1;

            // Rate limiting
            if page_len < per_page {
                break;
            }
            tokio::time::sleep(Duration::from_millis(self.client.client().config().delay_ms)).await;
        }

        // Cache the results
        if self.client.cache().enabled {
            if let Err(e) = self.client.cache().save("github", org, &repos) {
                tracing::warn!("Failed to cache repository list: {}", e);
            }
        }

        Ok(repos)
    }
}

/// GitHub repository API response structure.
#[derive(Debug, Deserialize)]
struct GitHubRepository {
    id: u64,
    name: String,
    clone_url: String,
    default_branch: String,
    archived: bool,
    fork: bool,
}

/// GitLab API client
pub struct GitLabClient {
    client: CachedRateLimitedClient,
}

impl GitLabClient {
    pub fn new(client: CachedRateLimitedClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl VcsClient for GitLabClient {
    fn platform(&self) -> VcsPlatform {
        VcsPlatform::GitLab
    }

    async fn discover_repositories(
        &self,
        org: &str,
        token: &str,
    ) -> crate::Result<Vec<VcsRepository>> {
        // Check cache first
        if let Some(cached_repos) = self.client.cache().load("gitlab", org)? {
            tracing::debug!("Using cached repository list for GitLab group {}", org);
            return Ok(cached_repos);
        }

        tracing::debug!("Fetching repository list for GitLab group {}", org);
        let mut repos = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url = format!(
                "https://gitlab.com/api/v4/groups/{}/projects?page={}&per_page={}&include_subgroups=true",
                org, page, per_page
            );

            let mut request = self.client.client().client().request(reqwest::Method::GET, &url);
            request = request.header("PRIVATE-TOKEN", token);

            let response = request.send().await.map_err(|e| crate::err!(VcsApi {
                platform: "gitlab".to_string(),
                message: format!("Failed to fetch GitLab projects: {}", e),
            }))?;

            if !response.status().is_success() {
                return Err(crate::err!(VcsApi {
                    platform: "gitlab".to_string(),
                    message: format!("GitLab API error: Status {}", response.status()),
                }));
            }

            let page_repos: Vec<GitLabProject> = response.json().await.map_err(|e| crate::err!(VcsApi {
                platform: "gitlab".to_string(),
                message: format!("Failed to parse GitLab projects: {}", e),
            }))?;

            let page_len = page_repos.len();
            if page_repos.is_empty() {
                break;
            }

            repos.extend(page_repos.into_iter().map(|p| VcsRepository {
                name: p.path_with_namespace.clone(),
                clone_url: p.http_url_to_repo,
                default_branch: p.default_branch.unwrap_or_else(|| "main".to_string()),
                archived: p.archived,
                fork: p.forked_from_project.is_some(),
                platform_id: p.id.to_string(),
            }));

            page += 1;

            // Rate limiting
            if page_len < per_page {
                break;
            }
            tokio::time::sleep(Duration::from_millis(self.client.client().config().delay_ms)).await;
        }

        // Cache the results
        if self.client.cache().enabled {
            if let Err(e) = self.client.cache().save("gitlab", org, &repos) {
                tracing::warn!("Failed to cache repository list: {}", e);
            }
        }

        Ok(repos)
    }
}

/// GitLab project API response structure.
#[derive(Debug, Deserialize)]
struct GitLabProject {
    id: u64,
    path_with_namespace: String,
    http_url_to_repo: String,
    default_branch: Option<String>,
    archived: bool,
    forked_from_project: Option<serde_json::Value>, // Presence indicates a fork
}

/// Azure DevOps API client
pub struct AzureDevOpsClient {
    client: CachedRateLimitedClient,
}

impl AzureDevOpsClient {
    pub fn new(client: CachedRateLimitedClient) -> Self {
        Self { client }
    }

    /// List all projects in an organization
    async fn list_projects(&self, organization: &str, token: Option<&str>) -> crate::Result<Vec<String>> {
        let mut projects = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut url = format!(
                "https://dev.azure.com/{}/_apis/projects?api-version=7.1&$top=100",
                organization
            );
            if let Some(ct) = &continuation_token {
                url.push_str(&format!("&continuationToken={}", ct));
            }

            let mut request = self.client.client().client().request(reqwest::Method::GET, &url);
            if let Some(t) = token {
                request = request.header("Authorization", format!("Basic {}", STANDARD.encode(format!(":{}", t))));
            }

            let response = request.send().await.map_err(|e| crate::err!(VcsApi {
                platform: "ado".to_string(),
                message: format!("Failed to fetch Azure DevOps projects for {}: {}", organization, e),
            }))?;

            let next_continuation = response.headers()
                .get("x-ms-continuationtoken")
                .and_then(|h| h.to_str().ok())
                .map(String::from);

            if !response.status().is_success() {
                return Err(crate::err!(VcsApi {
                    platform: "ado".to_string(),
                    message: format!("Azure DevOps API error listing projects: Status {}", response.status()),
                }));
            }
            use azure_devops_rust_api::core::models::TeamProjectReferenceList;

            let ado_response: TeamProjectReferenceList = response.json().await.map_err(|e| crate::err!(VcsApi {
                platform: "ado".to_string(),
                message: format!("Failed to parse Azure DevOps projects: {}", e),
            }))?;

            if ado_response.value.is_empty() {
                break;
            }

            tracing::debug!("Azure DevOps projects response: {:?}", ado_response);
            process::exit(1);

            projects.extend(ado_response.value.into_iter().map(|p| p.name));

            continuation_token = next_continuation;
            if continuation_token.is_none() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(self.client.client().config().delay_ms)).await;
        }

        Ok(projects)
    }

    /// List all repositories in a specific project
    async fn list_project_repos(&self, organization: &str, project: &str, token: &str) -> crate::Result<Vec<VcsRepository>> {
        let mut repos = Vec::new();
        let mut continuation_token: Option<String> = None;
        tracing::debug!("Listing project repositories for {}/{}", organization, project);

        loop {
            let mut url = format!(
                "https://dev.azure.com/{}/{}/_apis/git/repositories?api-version=7.1&$top=100",
                organization, project
            );
            if let Some(ct) = &continuation_token {
                url.push_str(&format!("&continuationToken={}", ct));
            }

            let mut request = self.client.client().client().request(reqwest::Method::GET, &url);
            tracing::debug!("Request URL: {} and token content: {:?}", url, token);
            request = request.header("Authorization", format!("Basic {}", STANDARD.encode(format!(":{}", token))));

            let response = request.send().await.map_err(|e| crate::err!(VcsApi {
                platform: "ado".to_string(),
                message: format!("Failed to fetch Azure DevOps repositories for {}/{}: {}", organization, project, e),
            }))?;

            let next_continuation = response.headers()
                .get("x-ms-continuationtoken")
                .and_then(|h| h.to_str().ok())
                .map(String::from);

            if !response.status().is_success() {
                let status = response.status();
                return Err(crate::err!(VcsApi {
                    platform: "ado".to_string(),
                    message: format!("Azure DevOps API error: Status {}", status),
                }));
            }

            // Get response text first for better error messages
            let response_text = response.text().await.map_err(|e| crate::err!(VcsApi {
                platform: "ado".to_string(),
                message: format!("[src/vcs_clients.rs:440] Failed to read Azure DevOps response: {}", e),
            }))?;

            use azure_devops_rust_api::git::models::GitRepositoryList;

            let ado_response: GitRepositoryList = serde_json::from_str(&response_text).map_err(|e| {
                let preview = if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text.clone()
                };
                crate::err!(VcsApi {
                    platform: "ado".to_string(),
                    message: format!(
                        "[src/vcs_clients.rs:448] Failed to parse Azure DevOps repositories JSON: {}\nResponse body:\n{}",
                        e, preview
                    ),
                })
            })?;

            tracing::debug!("Azure DevOps project repositories response: {:?}", ado_response);
            if ado_response.value.is_empty() {
                break;
            }

            repos.extend(ado_response.value.into_iter().map(|r| VcsRepository {
                name: format!("{}/{}", project, r.name),
                clone_url: r.web_url.unwrap_or("UNKNOWN-NOT-FOUND".to_string()),
                default_branch: r.default_branch.map(|s| s.replace("refs/heads/", "")).unwrap_or_else(|| "main".to_string()),
                archived: r.is_disabled.unwrap_or(false),
                fork: r.is_fork.unwrap_or(false),
                platform_id: r.id,
            }));

            continuation_token = next_continuation;
            if continuation_token.is_none() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(self.client.client().config().delay_ms)).await;
        }

        Ok(repos)
    }
}

#[async_trait]
impl VcsClient for AzureDevOpsClient {
    fn platform(&self) -> VcsPlatform {
        VcsPlatform::AzureDevOps
    }

    async fn discover_repositories(
        &self,
        org: &str,
        token: &str,
    ) -> crate::Result<Vec<VcsRepository>> {
        tracing::debug!(org = %org, "Discovering repositories for Azure DevOps organization");
        
        // Check cache first
        if let Some(cached_repos) = self.client.cache().load("ado", org)? {
            tracing::debug!("Using cached repository list for Azure DevOps {}", org);
            return Ok(cached_repos);
        }

        let parts: Vec<&str> = org.split('/').collect();
        let mut repos = Vec::new();

        match parts.len() {
            // org only - scan all projects
            1 => {
                let organization = parts[0];
                tracing::info!("Discovering all projects in Azure DevOps organization: {}", organization);

                let projects = self.list_projects(organization, Some(token)).await?;
                tracing::info!("Found {} projects in organization {}", projects.len(), organization);

                for project in projects {
                    tracing::debug!("Fetching repositories from project: {}/{}", organization, project);
                    match self.list_project_repos(organization, &project, token).await {
                        Ok(project_repos) => {
                            tracing::debug!("Found {} repos in {}/{}", project_repos.len(), organization, project);
                            repos.extend(project_repos);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to list repos in {}/{}: {}", organization, project, e);
                            // Continue with other projects
                        }
                    }
                }
            }
            // org/project - scan single project
            2 => {
                let organization = parts[0];
                let project = parts[1];
                tracing::debug!("Fetching repositories from Azure DevOps project: {}/{}", organization, project);
                repos = self.list_project_repos(organization, project, token).await?;
            }
            _ => {
                return Err(crate::err!(VcsApi {
                    platform: "ado".to_string(),
                    message: format!("Invalid Azure DevOps format: {}. Expected 'organization' or 'organization/project'", org),
                }));
            }
        }

        tracing::info!("Discovered {} total repositories in Azure DevOps {}", repos.len(), org);


        crate::wait_for_user_input().await;

        // Cache the results
        if self.client.cache().enabled {
            if let Err(e) = self.client.cache().save("ado", org, &repos) {
                tracing::warn!("Failed to cache repository list: {}", e);
            }
        }

        Ok(repos)
    }
}


/// Bitbucket API client
pub struct BitbucketClient {
    client: CachedRateLimitedClient,
}

impl BitbucketClient {
    pub fn new(client: CachedRateLimitedClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl VcsClient for BitbucketClient {
    fn platform(&self) -> VcsPlatform {
        VcsPlatform::Bitbucket
    }

    async fn discover_repositories(
        &self,
        org: &str, // Bitbucket uses "workspace" instead of org/group
        token: &str,
    ) -> crate::Result<Vec<VcsRepository>> {
        // Check cache first
        if let Some(cached_repos) = self.client.cache().load("bitbucket", org)? {
            tracing::debug!("Using cached repository list for Bitbucket workspace {}", org);
            return Ok(cached_repos);
        }

        tracing::debug!("Fetching repository list for Bitbucket workspace {}", org);
        let mut repos = Vec::new();
        let mut next_page_url: Option<String> = Some(format!(
            "https://api.bitbucket.org/2.0/repositories/{}?pagelen=100",
            org
        ));

        while let Some(url) = next_page_url.take() {
            let mut request = self.client.client().client().request(reqwest::Method::GET, &url);
    
            // Bitbucket uses x-token-auth as username for app passwords
            request = request.basic_auth("x-token-auth", Some(token));
            let response = request.send().await.map_err(|e| crate::err!(VcsApi {
                platform: "bitbucket".to_string(),
                message: format!("Failed to fetch Bitbucket repositories for {}: {}", org, e),
            }))?;

            if !response.status().is_success() {
                return Err(crate::err!(VcsApi {
                    platform: "bitbucket".to_string(),
                    message: format!("Bitbucket API error: Status {}", response.status()),
                }));
            }

            let bitbucket_response: BitbucketRepositoriesResponse = response.json().await.map_err(|e| crate::err!(VcsApi {
                platform: "bitbucket".to_string(),
                message: format!("Failed to parse Bitbucket repositories: {}", e),
            }))?;

            if bitbucket_response.values.is_empty() {
                break;
            }

            repos.extend(bitbucket_response.values.into_iter().filter_map(|r| {
                // Find HTTPS clone link
                let clone_link = r.links.clone.into_iter().find(|link| link.name == "https")?.href;
                Some(VcsRepository {
                    name: r.full_name,
                    clone_url: clone_link,
                    default_branch: r.mainbranch.map_or_else(|| "main".to_string(), |b| b.name),
                    archived: false, // Bitbucket doesn't have direct "archived" flag
                    fork: r.parent.is_some(),
                    platform_id: r.uuid,
                })
            }));

            next_page_url = bitbucket_response.next;

            if next_page_url.is_some() {
                // Rate limiting
                tokio::time::sleep(Duration::from_millis(self.client.client().config().delay_ms)).await;
            }
        }

        // Cache the results
        if self.client.cache().enabled {
            if let Err(e) = self.client.cache().save("bitbucket", org, &repos) {
                tracing::warn!("Failed to cache repository list: {}", e);
            }
        }

        Ok(repos)
    }
}

/// Bitbucket repository API response structure.
#[derive(Debug, Deserialize)]
struct BitbucketRepository {
    uuid: String,
    full_name: String,
    is_private: bool,
    mainbranch: Option<BitbucketBranch>,
    parent: Option<serde_json::Value>, // Presence indicates a fork
    links: BitbucketLinks,
}

#[derive(Debug, Deserialize)]
struct BitbucketBranch {
    name: String,
}

#[derive(Debug, Deserialize)]
struct BitbucketLinks {
    #[serde(rename = "clone")]
    clone: Vec<BitbucketCloneLink>,
}

#[derive(Debug, Deserialize)]
struct BitbucketCloneLink {
    href: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct BitbucketRepositoriesResponse {
    size: u64,
    page: u64,
    pagelen: u64,
    next: Option<String>,
    values: Vec<BitbucketRepository>,
}

/// Repository cache for VCS API responses.
///
/// Caches repository lists to reduce API calls and improve performance.
#[derive(Debug, Clone)]
pub struct RepoCache {
    cache_dir: PathBuf,
    ttl: Duration,
    enabled: bool,
}

impl Default for RepoCache {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("./.cache"))
                .join("monphare")
                .join("repo_cache"),
            ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            enabled: true,
        }
    }
}

impl RepoCache {
    pub fn new(cache_dir: PathBuf, ttl: Duration, enabled: bool) -> Self {
        Self {
            cache_dir,
            ttl,
            enabled,
        }
    }

    /// Load cached repositories for an organization.
    pub fn load(&self, platform: &str, org: &str) -> crate::Result<Option<Vec<VcsRepository>>> {
        if !self.enabled {
            return Ok(None);
        }

        let cache_file = self.cache_dir.join(format!("{}-{}.json", platform, org));
        if !cache_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&cache_file)?;
        let cached: CachedRepositories = serde_json::from_str(&content)?;

        // Handle potential clock issues gracefully
        let elapsed = cached.cached_at.elapsed().unwrap_or(self.ttl);

        if elapsed < self.ttl {
            tracing::debug!("Cache hit for {}/{}", platform, org);
            Ok(Some(cached.repos))
        } else {
            tracing::debug!("Cache expired for {}/{}", platform, org);
            // Optionally remove expired cache file
            if let Err(e) = std::fs::remove_file(&cache_file) {
                tracing::warn!("Failed to remove expired cache file {}: {}", cache_file.display(), e);
            }
            Ok(None)
        }
    }

    /// Save repositories to cache.
    pub fn save(&self, platform: &str, org: &str, repos: &[VcsRepository]) -> crate::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let cache_file = self.cache_dir.join(format!("{}-{}.json", platform, org));
        std::fs::create_dir_all(&self.cache_dir)?;

        let cached = CachedRepositories {
            cached_at: SystemTime::now(),
            repos: repos.to_vec(),
        };

        let content = serde_json::to_string_pretty(&cached)?;
        std::fs::write(&cache_file, content)?;

        tracing::debug!("Repositories cached for {}/{} at {}", platform, org, cache_file.display());
        Ok(())
    }

    /// Clear all cached repositories.
    pub fn clear(&self) -> crate::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if self.cache_dir.exists() {
            for entry in std::fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    std::fs::remove_file(&path)?;
                }
            }
        }
        tracing::info!("Cleared repository cache at {}", self.cache_dir.display());
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("cache_files".to_string(), 0);
        stats.insert("cache_size_bytes".to_string(), 0);

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.filter_map(std::result::Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    *stats.get_mut("cache_files").unwrap() += 1;
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        *stats.get_mut("cache_size_bytes").unwrap() += metadata.len() as usize;
                    }
                }
            }
        }
        stats
    }
}

/// Internal struct for caching repositories with a timestamp.
#[derive(Debug, Serialize, Deserialize)]
struct CachedRepositories {
    cached_at: SystemTime,
    repos: Vec<VcsRepository>,
}

/// HTTP client wrapper with rate limiting AND caching
#[derive(Debug, Clone)]
pub struct CachedRateLimitedClient {
    client: RateLimitedClient,
    cache: RepoCache,
}

impl CachedRateLimitedClient {
    pub fn new(client: RateLimitedClient, cache: RepoCache) -> Self {
        Self { client, cache }
    }

    pub fn client(&self) -> &RateLimitedClient {
        &self.client
    }

    pub fn cache(&self) -> &RepoCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut RepoCache {
        &mut self.cache
    }
}

impl Default for CachedRateLimitedClient {
    fn default() -> Self {
        Self::new(RateLimitedClient::default(), RepoCache::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};
    use serde_json::json;

    // Helper to setup mock server for GitHub
    async fn setup_github_mock_server(org: &str, repos: Vec<&str>) -> MockServer {
        let mock_server = MockServer::start().await;
        let mut response_repos = Vec::new();
        for (i, repo_name) in repos.iter().enumerate() {
            response_repos.push(json!({
                "id": i + 1,
                "name": repo_name,
                "clone_url": format!("https://github.com/{}/{}", org, repo_name),
                "default_branch": "main",
                "archived": false,
                "fork": false,
            }));
        }

        Mock::given(method("GET"))
            .and(path(format!("/orgs/{}/repos", org)))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_repos))
            .mount(&mock_server)
            .await;
        mock_server
    }

    #[tokio::test]
    async fn test_github_client_discover_repositories() -> crate::Result<()> {
        let org = "test-org";
        let repos = vec!["repo1", "repo2"];
        let mock_server = setup_github_mock_server(org, repos.clone()).await;

        let client = GitHubClient::new(CachedRateLimitedClient::new(
            RateLimitedClient::new(RateLimitConfig::default()),
            RepoCache::new(PathBuf::from("./test_cache"), Duration::from_secs(3600), true),
        ))
        .with_api_base_url(mock_server.uri());

        // Ensure cache is cleared before test
        let _ = client.client.cache().clear();

        let discovered_repos = client.discover_repositories(org, "test-token").await?;
        assert_eq!(discovered_repos.len(), repos.len());
        assert_eq!(discovered_repos[0].name, repos[0]);
        assert_eq!(discovered_repos[1].name, repos[1]);

        // Test caching: second call should not hit the mock server
        let discovered_repos_cached = client.discover_repositories(org, "test-token").await?;
        assert_eq!(discovered_repos_cached.len(), repos.len());

        Ok(())
    }
}

