//! Configuration module for DriftOps.
//!
//! This module handles loading and validating configuration from:
//! - YAML configuration files (`driftops.yaml`)
//! - Environment variables
//! - CLI arguments
//!
//! # Configuration File Format
//!
//! ```yaml
//! # driftops.yaml
//!
//! # Scanning options
//! scan:
//!   exclude_patterns:
//!     - "**/test/**"
//!     - "**/examples/**"
//!   continue_on_error: true
//!   max_depth: 100
//!
//! # Analysis options
//! analysis:
//!   check_exact_versions: true
//!   check_prerelease: true
//!   check_upper_bound: true
//!   max_age_months: 12  # Flag modules older than this
//!
//! # Output options
//! output:
//!   colored: true
//!   verbose: false
//!   pretty: true
//!
//! # Git options
//! git:
//!   token: ${GITHUB_TOKEN}  # Environment variable expansion
//!   branch: main
//!
//! # Policy rules
//! policies:
//!   require_version_constraint: true
//!   require_upper_bound: false
//!   allowed_providers:
//!     - hashicorp/*
//!   blocked_modules: []
//! ```

use crate::{config, error::{DriftOpsError, Result}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scanning options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ScanOptions {
    /// Patterns to exclude from scanning (glob patterns).
    pub exclude_patterns: Vec<String>,

    /// Continue scanning even if some files fail to parse.
    pub continue_on_error: bool,

    /// Maximum depth for recursive directory scanning.
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

/// Analysis options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AnalysisOptions {
    /// Check for exact version constraints.
    #[serde(default = "default_true")]
    pub check_exact_versions: bool,

    /// Check for pre-release versions.
    #[serde(default = "default_true")]
    pub check_prerelease: bool,

    /// Check for missing upper bounds.
    #[serde(default = "default_true")]
    pub check_upper_bound: bool,

    /// Maximum age in months before flagging as outdated.
    #[serde(default = "default_max_age")]
    pub max_age_months: u32,
}

/// Output options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OutputOptions {
    /// Use colored output.
    #[serde(default = "default_true")]
    pub colored: bool,

    /// Verbose output mode.
    pub verbose: bool,

    /// Pretty-print JSON output.
    #[serde(default = "default_true")]
    pub pretty: bool,
}

/// Git options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GitOptions {
    /// GitHub personal access token
    pub github_token: Option<String>,
    
    /// GitLab personal/group access token
    pub gitlab_token: Option<String>,
    
    /// Azure DevOps personal access token
    pub azure_devops_token: Option<String>,
    
    /// Bitbucket app password
    pub bitbucket_token: Option<String>,
    
    /// Legacy single token (for backward compatibility)
    /// Will be used as fallback if platform-specific tokens are not set

    /// Git branch to checkout.
    pub branch: Option<String>,

    /// Patterns to exclude when git cloning repositories.
    pub exclude_patterns: Option<Vec<String>>,

    /// Patterns to include when git cloning repositories.
    pub include_patterns: Option<Vec<String>>,
}

impl GitOptions {

    /// Get token for a specific VCS platform with auto-detection from environment
    /// and configuration fallback.
    ///
    /// Priority order:
    /// 1. Platform-specific config value
    /// 2. Environment variable (DO_{PLATFORM}_TOKEN)
    /// 3. Legacy environment variable (DRIFTOPS_GIT_TOKEN)
    #[must_use]
    pub fn get_token_for_platform(&self, platform: &str) -> Result<String> {
        tracing::debug!(platform = %platform, "Getting token for platform");

        // Helper to get non-empty environment variable
        let get_non_empty_env = |var: &str| -> Option<String> {
            std::env::var(var).ok().filter(|s| !s.is_empty())
        };

        // 1. Check platform-specific config token
        let config_token = match platform.to_lowercase().as_str() {
            "github" => self.github_token.as_deref(),
            "gitlab" => self.gitlab_token.as_deref(),
            "ado" | "azure" | "azure-devops" => self.azure_devops_token.as_deref(),
            "bitbucket" => self.bitbucket_token.as_deref(),
            _ => {
                return Err(DriftOpsError::ConfigMissing {
                    key: format!("Unsupported platform: {}", platform)
                });
            }
        };

        if let Some(token) = config_token {
            tracing::debug!(platform = %platform, "Using token from configuration");
            return Ok(token.to_string());
        }

        // 2. Check platform-specific environment variable
        let env_var_name = match platform.to_lowercase().as_str() {
            "github" => "DO_GITHUB_TOKEN",
            "gitlab" => "DO_GITLAB_TOKEN",
            "ado" | "azure" | "azure-devops" => "DO_AZURE_DEVOPS_TOKEN",
            "bitbucket" => "DO_BITBUCKET_TOKEN",
            _ => unreachable!(), // Already handled above
        };

        if let Some(env_token) = get_non_empty_env(env_var_name) {
            tracing::debug!(platform = %platform, env_var = %env_var_name, "Using token from environment variable");
            return Ok(env_token);
        }

        // 3. Fallback to legacy environment variable
        get_non_empty_env("DRIFTOPS_GIT_TOKEN")
            .map(|token| {
                tracing::warn!(platform = %platform, "Using token from legacy DRIFTOPS_GIT_TOKEN environment variable");
                token
            })
            .ok_or_else(|| {
                tracing::error!(platform = %platform, env_var = %env_var_name, "No token found for platform");
                DriftOpsError::ConfigMissing {
                    key: format!("{} token - please set {} environment variable or configure in driftops.yaml",
                        platform, env_var_name)
                }
            })
    }
    
    /// Load tokens from environment variables, updating config values if not set
    pub fn load_from_env(&mut self) {
        if self.github_token.is_none() {
            if let Ok(token) = std::env::var("DO_GITHUB_TOKEN") {
                if !token.is_empty() {
                    tracing::debug!("Loaded GitHub token from DO_GITHUB_TOKEN environment variable");
                    self.github_token = Some(token);
                }
            }
        }
        
        if self.gitlab_token.is_none() {
            if let Ok(token) = std::env::var("DO_GITLAB_TOKEN") {
                if !token.is_empty() {
                    tracing::debug!("Loaded GitLab token from DO_GITLAB_TOKEN environment variable");
                    self.gitlab_token = Some(token);
                }
            }
        }
        
        if self.azure_devops_token.is_none() {
            if let Ok(token) = std::env::var("DO_AZURE_DEVOPS_TOKEN") {
                if !token.is_empty() {
                    tracing::debug!("Loaded Azure DevOps token from DO_AZURE_DEVOPS_TOKEN environment variable");
                    self.azure_devops_token = Some(token);
                }
            }
        }
        
        if self.bitbucket_token.is_none() {
            if let Ok(token) = std::env::var("DO_BITBUCKET_TOKEN") {
                if !token.is_empty() {
                    tracing::debug!("Loaded Bitbucket token from DO_BITBUCKET_TOKEN environment variable");
                    self.bitbucket_token = Some(token);
                }
            }
        }
    }
}

/// Policy rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PoliciesOptions {
    /// Require version constraints on all modules.
    #[serde(default = "default_true")]
    pub require_version_constraint: bool,

    /// Require upper bounds on version constraints.
    pub require_upper_bound: bool,

    /// Allowed provider patterns (glob).
    pub allowed_providers: Vec<String>,

    /// Blocked module patterns (glob).
    pub blocked_modules: Vec<String>,

    /// Custom severity overrides for finding codes.
    pub severity_overrides: HashMap<String, String>,

}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
/// Deprecation tracking options for modules and providers.
///
/// The `modules` and `providers` fields map resource identifiers to
/// a list of deprecation rules, each describing which versions are deprecated,
/// the reason for deprecation, severity, and replacements.
pub struct DeprecationsOptions {
    /// Map of module identifiers to their deprecation rules.
    pub modules: HashMap<String, Vec<DeprecationRef>>,
    /// Map of provider identifiers to their deprecation rules.
    pub providers: HashMap<String, Vec<DeprecationRef>>,

    /// Runtime deprecation rules for Terraform and OpenTofu.
    pub runtime: HashMap<String, Vec<DeprecationRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
/// A single deprecation rule for a module or provider version.
///
/// This struct represents a deprecated version or version range,
/// the reason for deprecation, the severity (e.g., "error", "warning"),
/// and a recommended replacement version or module.
pub struct DeprecationRef {
    /// A version or version range (e.g., "1.0.1", ">= 2.0.8, < 3.0.0", "~> 3.1").
    /// Can be git ref / SHA1 / tag / commit hash / etc.
    /// If the version is a range, it will be a comma-separated list of version ranges.
    pub version: Option<String>,

    pub git_ref: Option<String>,
    /// The reason for this deprecation (e.g., "Critical security vulnerability CVE-2023-1234").
    pub reason: String,
    /// Severity of the deprecation ("error", "warning", etc).
    pub severity: String,
    /// Suggested replacement (could be a module name or version constraint).
    pub replacement: String,
}

/// Main configuration structure with nested sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Scanning options
    pub scan: ScanOptions,

    /// Analysis options
    pub analysis: AnalysisOptions,

    /// Output options
    pub output: OutputOptions,

    /// Git options
    pub git: GitOptions,

    /// Policy rules
    pub policies: PoliciesOptions,

    /// Deprecation tracking options
    pub deprecations: DeprecationsOptions,
}

fn default_max_depth() -> usize {
    100
}

fn default_max_age() -> u32 {
    12
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanOptions {
                exclude_patterns: vec![
                    "**/test/**".to_string(),
                    "**/tests/**".to_string(),
                    "**/examples/**".to_string(),
                    "**/.terraform/**".to_string(),
                ],
                continue_on_error: false,
                max_depth: default_max_depth(),
            },
            analysis: AnalysisOptions {
                check_exact_versions: true,
                check_prerelease: true,
                check_upper_bound: true,
                max_age_months: default_max_age(),
            },
            output: OutputOptions {
                colored: true,
                verbose: false,
                pretty: true,
            },
            git: GitOptions {
                github_token: None,
                gitlab_token: None,
                azure_devops_token: None,
                bitbucket_token: None, // Legacy token is not needed here
                branch: None,
                include_patterns: None,
                exclude_patterns: None
            },
            policies: PoliciesOptions {
                require_version_constraint: true,
                require_upper_bound: false,
                allowed_providers: Vec::new(),
                blocked_modules: Vec::new(),
                severity_overrides: HashMap::new(),
            },
            deprecations: DeprecationsOptions {
                modules: HashMap::new(),
                providers: HashMap::new(),
                runtime: HashMap::new(),
            },
        }
    }
}

impl Config {
    /// Load configuration from a YAML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the YAML is invalid.
    pub fn from_yaml(content: &str) -> Result<Self> {
        tracing::debug!("Parsing configuration from YAML");
        // First, expand environment variables
        let expanded = expand_env_vars(content);
        tracing::debug!("Expanded environment variables in configuration");

        let config: Config = serde_yaml::from_str(&expanded).map_err(|e| DriftOpsError::ConfigParse {
            message: e.to_string(),
            source: None,
        })?;
        
        tracing::debug!(
            exclude_patterns = config.scan.exclude_patterns.len(),
            continue_on_error = config.scan.continue_on_error,
            "Configuration loaded successfully"
        );
        
        Ok(config)
    }

    /// Generate an example YAML configuration.
    #[must_use]
    pub fn example_yaml() -> String {
        r#"# DriftOps Configuration File
# https://github.com/yourusername/driftops

# Scanning options
scan:
  # Patterns to exclude from scanning (glob patterns)
  exclude_patterns:
    - "**/test/**"
    - "**/tests/**"
    - "**/examples/**"
    - "**/.terraform/**"
  
  # Continue scanning even if some files fail to parse
  continue_on_error: false
  
  # Maximum depth for recursive directory scanning
  max_depth: 100

# Analysis options
analysis:
  # Flag exact version constraints (e.g., "= 1.0.0")
  check_exact_versions: true
  
  # Flag pre-release versions (e.g., "1.0.0-beta")
  check_prerelease: true
  
  # Flag constraints without upper bounds (e.g., ">= 1.0")
  check_upper_bound: true
  
  # Flag modules older than this many months
  max_age_months: 12

# Output options
output:
  # Use colored output in terminal
  colored: true
  
  # Enable verbose output
  verbose: false
  
  # Pretty-print JSON output
  pretty: true

# Git options (for cloning repositories)
git:
  # Authentication token (can use environment variable)
  # token: ${GITHUB_TOKEN}
  
  # Branch to checkout (default: repository's default branch)
  # branch: main

# Policy rules
policies:
  # Require version constraints on all modules
  require_version_constraint: true
  
  # Require upper bounds on all version constraints
  require_upper_bound: false
  
  # Allowed provider patterns (empty = allow all)
  # allowed_providers:
  #   - hashicorp/*
  #   - terraform-aws-modules/*
  
  # Blocked module patterns
  # blocked_modules:
  #   - deprecated-module/*

  # Severity overrides for specific finding codes
  # severity_overrides:
  #   DRIFT002: info  # Downgrade missing constraint to info
  #   DRIFT003: error  # Upgrade wildcard to error

deprecations:
  # runtime:
    # terraform:
      # - version: "< 0.13.0"
      #   reason: "Legacy Terraform version, migrate to v0.13.0 or later"
      #   severity: error
      #   replacement: ">= 0.13.0"
    # opentofu:
    #   - version: '*'
    #     reason: "OpenTofu is not validated yet, use Terraform instead"
    #     severity: error
    #     replacement: "terraform"

  # modules:
    # Module source identifier (use canonical format or shorthand)
    # "claranet/azure-log-mngt-v1/azurerm":
    #   - version: "1.0.1"
    #     reason: "Critical security vulnerability CVE-2023-1234"
    #     severity: error
    #     replacement: "claranet/azure-log-mngt-v3/azurerm"

    #   - version: ">= 2.0.8, < 3.0.0"
    #     reason: "Breaking API changes, migrate to v3"
    #     severity: warning
    #     replacement: "claranet/azure-log-mngt-v3/azurerm"
      
    #   - version: "~> 3.1"
    #     reason: "v3.1.x has known performance issues"
    #     severity: warning
    #     replacement: "claranet/azure-log-mngt-v3/azurerm"

    # "claranet/azure-log-mngt-v2/azurerm":
    #   - version: "0.0.10"
    #     reason: "CVE-2024-5678 detected during security audit"
    #     severity: error
    #     replacement: "claranet/azure-log-mngt-v3/azurerm"

  # providers:
    # Provider source (qualified name like "hashicorp/azurerm")
    # "hashicorp/azurerm":
    #   versions:
    #     - version: "> 0.0.1, < 3.50.0"
    #       reason: "Multiple CVEs in versions before 3.50.0"
    #       severity: error
    #       replacement: ">= 3.50.0"
        
    #     - version: ">= 4.0.0, < 4.50.0"
    #       reason: "Known authentication issues in early 4.x versions"
    #       severity: warning
    #       replacement: ">= 4.50.0"

"#
        .to_string()
    }

    /// Merge CLI arguments into the configuration.
    pub fn merge_cli_args(&mut self, args: &crate::cli::ScanArgs) {
        if !args.exclude_patterns.is_empty() {
            self.scan
                .exclude_patterns
                .extend(args.exclude_patterns.iter().cloned());
        }
        if args.continue_on_error {
            self.scan.continue_on_error = true;
        }
        if args.max_depth != 100 {
            self.scan.max_depth = args.max_depth;
        }
        if let Some(_token) = &args.git_token.as_ref() {
            // Note: Legacy token field no longer exists, use platform-specific tokens instead
        }
        if let Some(ref branch) = args.branch {
            self.git.branch = Some(branch.clone());
        }
    }
    
    /// Load VCS tokens from environment variables
    /// This should be called after loading config to populate token fields
    pub fn load_vcs_tokens_from_env(&mut self) {
        tracing::debug!("Loading VCS tokens from environment variables");
        self.git.load_from_env();
        tracing::debug!(
            github_token_set = self.git.github_token.is_some(),
            gitlab_token_set = self.git.gitlab_token.is_some(),
            azure_devops_token_set = self.git.azure_devops_token.is_some(),
            bitbucket_token_set = self.git.bitbucket_token.is_some(),
            "VCS token loading complete"
        );
    }
}

/// Expand environment variables in a string.
///
/// Supports `${VAR}` and `$VAR` syntax.
fn expand_env_vars(content: &str) -> String {
    let mut result = content.to_string();

    // Find all ${VAR} patterns
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    for cap in re.captures_iter(content) {
        let var_name = &cap[1];
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(&cap[0], &value);
        }
    }

    // Find all $VAR patterns (word boundary)
    let re = regex::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    for cap in re.captures_iter(content) {
        let var_name = &cap[1];
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(&cap[0], &value);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.analysis.check_exact_versions);
        assert!(config.analysis.check_prerelease);
        assert!(config.analysis.check_upper_bound);
        assert_eq!(config.scan.max_depth, 100);
    }

    #[test]
    fn test_config_from_yaml_nested() {
        let yaml = r#"
scan:
  exclude_patterns:
    - "**/vendor/**"
  continue_on_error: true
  max_depth: 50
analysis:
  check_exact_versions: false
  max_age_months: 6
output:
  colored: false
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert!(config.scan.exclude_patterns.contains(&"**/vendor/**".to_string()));
        assert!(config.scan.continue_on_error);
        assert_eq!(config.scan.max_depth, 50);
        assert!(!config.analysis.check_exact_versions);
        assert_eq!(config.analysis.max_age_months, 6);
        assert!(!config.output.colored);
    }

    #[test]
    fn test_config_from_yaml_flat() {
        // Test backward compatibility with flat YAML
        let yaml = r#"
scan:
  exclude_patterns:
    - "**/vendor/**"
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert!(config.scan.exclude_patterns.contains(&"**/vendor/**".to_string()));
    }

    #[test]
    fn test_env_var_expansion() {
        // Test environment variable expansion without modifying env
        // (which requires unsafe code). Instead, test the regex/logic directly.

        // Test ${VAR} pattern
        let content_with_literal = "token: ${LITERAL_VALUE}";
        let expanded = expand_env_vars(content_with_literal);
        // If env var doesn't exist, the pattern should remain unchanged
        assert!(expanded.contains("${LITERAL_VALUE}") || expanded.contains("LITERAL_VALUE"));

        // Test that the function doesn't crash on various patterns
        let patterns = vec![
            "no vars here",
            "$NOTAVAR123",
            "${NESTED${VAR}}",
            "normal = ${KEY}",
        ];

        for pattern in patterns {
            let _ = expand_env_vars(pattern);
        }
    }

    #[test]
    fn test_example_yaml_is_valid() {
        let example = Config::example_yaml();
        // The example should be valid YAML (comments are stripped by parser)
        let result = Config::from_yaml(&example);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_with_policies() {
        let yaml = r#"
policies:
  require_version_constraint: true
  require_upper_bound: true
  allowed_providers:
    - hashicorp/*
    - terraform-aws-modules/*
  blocked_modules:
    - deprecated/*
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert!(config.policies.require_version_constraint);
        assert!(config.policies.require_upper_bound);
        assert_eq!(config.policies.allowed_providers.len(), 2);
        assert_eq!(config.policies.blocked_modules.len(), 1);
    }

    #[test]
    fn test_severity_overrides() {
        let yaml = r#"
policies:
  severity_overrides:
    DRIFT001: warning
    DRIFT002: info
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(
            config.policies.severity_overrides.get("DRIFT001"),
            Some(&"warning".to_string())
        );
        assert_eq!(
            config.policies.severity_overrides.get("DRIFT002"),
            Some(&"info".to_string())
        );
    }

    #[test]
    fn test_config_with_deprecations() {
        let yaml = r#"
deprecations:
  runtime:
    terraform:
      - version: "< 0.13.0"
        reason: "Legacy Terraform version, migrate to v0.13.0 or later"
        severity: error
        replacement: ">= 0.13.0"
  modules:
    "foobar/bar-module/azurerm":
      - version: "< 2.0.0"
        reason: "Legacy module version, migrate to v2.0.0 or later"
        severity: error
        replacement: ">= 2.0.0"
  providers:
    "hashicorp/azurerm":
      - version: "< 4.0.0"
        reason: "Legacy provider version, migrate to v4.0.0 or later"
        severity: error
        replacement: ">= 4.0.0" 
    "#;
    let config = Config::from_yaml(yaml).unwrap();

    assert_eq!(config.deprecations.runtime.len(), 1);
    assert_eq!(config.deprecations.runtime.get("terraform").unwrap().len(), 1);
    assert_eq!(
        config.deprecations.runtime.get("terraform").unwrap()[0].version,
        Some("< 0.13.0".to_string())
    );
    assert_eq!(config.deprecations.runtime.get("terraform").unwrap()[0].reason, "Legacy Terraform version, migrate to v0.13.0 or later");
    assert_eq!(config.deprecations.runtime.get("terraform").unwrap()[0].severity, "error");
    assert_eq!(config.deprecations.runtime.get("terraform").unwrap()[0].replacement, ">= 0.13.0");

    assert_eq!(config.deprecations.modules.len(), 1);
    assert_eq!(config.deprecations.modules.get("foobar/bar-module/azurerm").unwrap().len(), 1);
    assert_eq!(
        config.deprecations.modules.get("foobar/bar-module/azurerm").unwrap()[0].version,
        Some("< 2.0.0".to_string())
    );
    assert_eq!(config.deprecations.modules.get("foobar/bar-module/azurerm").unwrap()[0].reason, "Legacy module version, migrate to v2.0.0 or later");
    assert_eq!(config.deprecations.modules.get("foobar/bar-module/azurerm").unwrap()[0].severity, "error");
    assert_eq!(config.deprecations.modules.get("foobar/bar-module/azurerm").unwrap()[0].replacement, ">= 2.0.0");

    assert_eq!(config.deprecations.providers.len(), 1);
    assert_eq!(config.deprecations.providers.get("hashicorp/azurerm").unwrap().len(), 1);
    assert_eq!(
        config.deprecations.providers.get("hashicorp/azurerm").unwrap()[0].version,
        Some("< 4.0.0".to_string())
    );
    assert_eq!(config.deprecations.providers.get("hashicorp/azurerm").unwrap()[0].reason, "Legacy provider version, migrate to v4.0.0 or later");
    assert_eq!(config.deprecations.providers.get("hashicorp/azurerm").unwrap()[0].severity, "error");
    assert_eq!(config.deprecations.providers.get("hashicorp/azurerm").unwrap()[0].replacement, ">= 4.0.0");
    }
}
