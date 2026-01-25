//! Core data types used throughout DriftOps.
//!
//! This module defines the fundamental data structures for representing:
//! - Terraform/OpenTofu modules and providers
//! - Version constraints and ranges
//! - Analysis results and findings
//! - Report formats and severity levels

use crate::graph::DependencyGraph;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::path::PathBuf;

/// Represents a Terraform/OpenTofu module reference.
///
/// A module reference captures all information about a module block
/// in a Terraform configuration, including its source and version constraints.
///
/// # Example HCL
///
/// ```hcl
/// module "vpc" {
///   source  = "terraform-aws-modules/vpc/aws"
///   version = "~> 5.0"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleRef {
    /// The name/label of the module block (e.g., "vpc")
    pub name: String,

    /// The source of the module (registry path, Git URL, or local path)
    pub source: ModuleSource,

    /// Version constraint, if specified
    pub version_constraint: Option<Constraint>,

    /// File where this module is defined
    pub file_path: PathBuf,

    /// Line number in the file
    pub line_number: usize,

    /// The repository/project this module belongs to
    pub repository: Option<String>,

    /// Additional attributes from the module block
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

/// Represents a runtime environment (Terraform or OpenTofu) and its version.
///
/// Used to record which version of the runtime a configuration targets,
/// enabling version-based analysis such as deprecation checks.
///
/// # Fields
/// - `name`: The name of the runtime ("terraform" or "opentofu").
/// - `version`: The version string (e.g., "1.4.6").
///
/// # Example
/// ```rust,no_run
/// use driftops::types::{RuntimeRef, RuntimeSource};
/// use driftops::Constraint;
/// use std::path::PathBuf;
///
/// let runtime = RuntimeRef {
///     name: "terraform".to_string(),
///     version: Constraint::parse("1.4.6").unwrap(),
///     source: RuntimeSource::Terraform,
///     file_path: PathBuf::from("main.tf"),
///     line_number: 1,
///     repository: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRef {
    /// The name of the runtime ("terraform" or "opentofu")
    pub name: String,
    /// The version of the runtime (e.g., "1.4.6")
    pub version: Constraint,
    /// The source of the runtime
    pub source: RuntimeSource,
    /// File where this runtime is defined
    pub file_path: PathBuf,
    /// Line number in the file
    pub line_number: usize,
    /// The repository/project this runtime belongs to
    pub repository: Option<String>,
}

/// Represents the source of a runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeSource {
    /// Terraform
    Terraform,
    /// OpenTofu
    OpenTofu,
}

impl std::fmt::Display for RuntimeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terraform => write!(f, "terraform"),
            Self::OpenTofu => write!(f, "opentofu"),
        }
    }
}

/// Represents the source of a Terraform module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ModuleSource {
    /// Terraform Registry module (e.g., "hashicorp/consul/aws")
    Registry {
        /// Registry hostname (default: registry.terraform.io)
        hostname: String,
        /// Namespace (e.g., "hashicorp")
        namespace: String,
        /// Module name (e.g., "consul")
        name: String,
        /// Provider (e.g., "aws")
        provider: String,
    },

    /// Git repository source
    Git {
        /// Hostname "ssh.dev.azure.com/v3/lpl-sources/Terraform/mod-azure-gov-security-ips"
        host: String,
        /// Repository URL
        url: String,
        /// Git ref (branch, tag, or commit)
        ref_: Option<String>,
        /// Subdirectory within the repository
        subdir: Option<String>,
    },

    /// Local file path
    Local {
        /// Path to the module (relative or absolute)
        path: String,
    },

    /// HTTP/HTTPS URL
    Http {
        /// URL to the module archive
        url: String,
    },

    /// S3 bucket source
    S3 {
        /// Bucket name
        bucket: String,
        /// Object key
        key: String,
        /// AWS region
        region: Option<String>,
    },

    /// GCS bucket source
    Gcs {
        /// Bucket name
        bucket: String,
        /// Object path
        path: String,
    },

    /// Unknown or unparseable source
    Unknown(String),
}

impl ModuleSource {
    /// Returns a canonical identifier for this source.
    #[must_use]
    pub fn canonical_id(&self) -> String {
        match self {
            Self::Registry {
                hostname,
                namespace,
                name,
                provider,
            } => format!("{hostname}/{namespace}/{name}/{provider}"),
            Self::Git { host,url, ref_, subdir } => {
                let mut id = host.clone();
                if let Some(r) = ref_ {
                    id.push_str(&format!("?ref={r}"));
                }
                if let Some(s) = subdir {
                    if s.is_empty() {
                        tracing::warn!("Subdirectory is empty for Git source {url}, this is not allowed, ignoring...");
                    } else {
                        id.push_str(&format!("//{s}"));
                    }
                }
                id
            }
            Self::Local { path } => format!("local://{path}"),
            Self::Http { url } => url.clone(),
            Self::S3 { bucket, key, .. } => format!("s3://{bucket}/{key}"),
            Self::Gcs { bucket, path } => format!("gcs://{bucket}/{path}"),
            Self::Unknown(s) => s.clone(),
        }
    }

    /// Returns true if this is a local module source.
    #[must_use]
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Local { .. })
    }

    /// Returns true if this is a registry module source.
    #[must_use]
    pub const fn is_registry(&self) -> bool {
        matches!(self, Self::Registry { .. })
    }
}

/// Represents a Terraform provider requirement.
///
/// # Example HCL
///
/// ```hcl
/// terraform {
///   required_providers {
///     aws = {
///       source  = "hashicorp/aws"
///       version = ">= 4.0, < 6.0"
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRef {
    /// Local name of the provider (e.g., "aws")
    pub name: String,

    /// Provider source (e.g., "hashicorp/aws")
    pub source: Option<String>,

    /// Version constraint
    pub version_constraint: Option<Constraint>,

    /// File where this provider is required
    pub file_path: PathBuf,

    /// Line number in the file
    pub line_number: usize,

    /// The repository/project this provider requirement belongs to
    pub repository: Option<String>,
}

impl ProviderRef {
    /// Returns the fully qualified provider source.
    ///
    /// If no explicit source is provided, assumes the default namespace.
    #[must_use]
    pub fn qualified_source(&self) -> String {
        self.source
            .clone()
            .unwrap_or_else(|| format!("hashicorp/{}", self.name))
    }
}

/// Represents a version constraint expression.
///
/// Supports Terraform's constraint syntax:
/// - `= 1.0.0` - Exact version
/// - `!= 1.0.0` - Not equal
/// - `> 1.0.0`, `>= 1.0.0` - Greater than
/// - `< 1.0.0`, `<= 1.0.0` - Less than
/// - `~> 1.0` - Pessimistic constraint (allows rightmost version component to increment)
/// - `>= 1.0, < 2.0` - Multiple constraints (AND)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraint {
    /// The raw constraint string as written in HCL
    pub raw: String,

    /// Parsed version ranges
    pub ranges: Vec<VersionRange>,
}

impl Constraint {
    /// Parse a constraint string into a `Constraint`.
    ///
    /// # Errors
    ///
    /// Returns an error if the constraint string is invalid.
    pub fn parse(s: &str) -> Result<Self, crate::error::DriftOpsError> {
        let ranges = parse_constraint_string(s)?;
        Ok(Self {
            raw: s.to_string(),
            ranges,
        })
    }

    /// Check if this constraint is satisfied by a given version.
    #[must_use]
    pub fn is_satisfied_by(&self, version: &semver::Version) -> bool {
        self.ranges.iter().all(|range| range.contains(version))
    }

    /// Check if this constraint conflicts with another constraint.
    ///
    /// Two constraints conflict if there is no version that satisfies both.
    #[must_use]
    pub fn conflicts_with(&self, other: &Self) -> bool {
        // Simple heuristic: check if the ranges have any overlap
        // This is a simplified check; a full implementation would need
        // to compute the intersection of all constraint ranges
        !self.has_overlap_with(other)
    }

    /// Check if there's any version that could satisfy both constraints.
    #[must_use]
    pub fn has_overlap_with(&self, other: &Self) -> bool {
        // Get the effective bounds for both constraints
        let self_bounds = self.effective_bounds();
        let other_bounds = other.effective_bounds();

        // Check if the ranges overlap
        match (self_bounds, other_bounds) {
            (Some((self_min, self_max)), Some((other_min, other_max))) => {
                // Ranges overlap if neither is entirely before the other
                !(self_max < other_min || other_max < self_min)
            }
            // If we can't determine bounds, assume they might overlap
            _ => true,
        }
    }

    /// Get the effective minimum and maximum bounds of this constraint.
    fn effective_bounds(&self) -> Option<(semver::Version, semver::Version)> {
        let mut min = semver::Version::new(0, 0, 0);
        let mut max = semver::Version::new(u64::MAX, u64::MAX, u64::MAX);

        for range in &self.ranges {
            match range {
                VersionRange::Exact(v) => {
                    min = v.clone();
                    max = v.clone();
                }
                VersionRange::GreaterThan(v) => {
                    if *v >= min {
                        min = next_version(v);
                    }
                }
                VersionRange::GreaterThanOrEqual(v) => {
                    if *v > min {
                        min = v.clone();
                    }
                }
                VersionRange::LessThan(v) => {
                    if *v <= max {
                        max = prev_version(v);
                    }
                }
                VersionRange::LessThanOrEqual(v) => {
                    if *v < max {
                        max = v.clone();
                    }
                }
                VersionRange::NotEqual(_) => {
                    // NotEqual doesn't affect bounds directly
                }
                VersionRange::Pessimistic {version, parts} => {
                    // ~> X.Y allows X.Y.* but not X.(Y+1)
                    if *version > min {
                        min = version.clone();
                    }
                    let upper = pessimistic_upper_bound(version, *parts);
                    if upper < max {
                        max = upper;
                    }
                }
            }
        }

        if min <= max {
            Some((min, max))
        } else {
            None
        }
    }

    /// Returns true if this constraint allows any version (no constraint).
    #[must_use]
    pub fn is_unconstrained(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Returns true if this constraint is overly broad (e.g., ">= 0.0.0").
    #[must_use]
    pub fn is_overly_broad(&self) -> bool {
        if self.ranges.len() == 1 {
            if let Some(VersionRange::GreaterThanOrEqual(v)) = self.ranges.first() {
                return v.major == 0 && v.minor == 0 && v.patch == 0;
            }
        }
        false
    }
}

/// Represents a single version range component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionRange {
    /// Exact version match: `= X.Y.Z`
    Exact(semver::Version),
    /// Greater than: `> X.Y.Z`
    GreaterThan(semver::Version),
    /// Greater than or equal: `>= X.Y.Z`
    GreaterThanOrEqual(semver::Version),
    /// Less than: `< X.Y.Z`
    LessThan(semver::Version),
    /// Less than or equal: `<= X.Y.Z`
    LessThanOrEqual(semver::Version),
    /// Not equal: `!= X.Y.Z`
    NotEqual(semver::Version),
    /// Pessimistic constraint: `~> X.Y`
    Pessimistic {
        /// The version specified in the constraint
        version: semver::Version,
        /// Number of version components specified (1=X, 2=X.Y, 3=X.Y.Z)
        parts: usize,
    },
}

impl VersionRange {
    /// Check if a version satisfies this range.
    #[must_use]
    pub fn contains(&self, version: &semver::Version) -> bool {
        match self {
            Self::Exact(v) => version == v,
            Self::GreaterThan(v) => version > v,
            Self::GreaterThanOrEqual(v) => version >= v,
            Self::LessThan(v) => version < v,
            Self::LessThanOrEqual(v) => version <= v,
            Self::NotEqual(v) => version != v,
            Self::Pessimistic { version: v, parts } => {
                // ~> X.Y.Z allows >= X.Y.Z and < X.(Y+1).0
                // ~> X.Y allows >= X.Y.0 and < X.(Y+1).0
                let upper = pessimistic_upper_bound(v, *parts);
                version >= v && version < &upper
            }
        }
    }
}

/// Parse a constraint string into version ranges.
fn parse_constraint_string(s: &str) -> Result<Vec<VersionRange>, crate::error::DriftOpsError> {
    let mut ranges = Vec::new();

    // Split on comma for multiple constraints
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let range = parse_single_constraint(part)?;
        ranges.push(range);
    }

    Ok(ranges)
}

/// Parse a single constraint expression.
fn parse_single_constraint(s: &str) -> Result<VersionRange, crate::error::DriftOpsError> {
    let s = s.trim();

    // Pessimistic constraint
    if let Some(version_str) = s.strip_prefix("~>") {
        let version_str = version_str.trim();
        let version = parse_version(version_str)?;
        // Count dots in the original version string to determine parts
        let parts = version_str.matches('.').count() + 1;
        return Ok(VersionRange::Pessimistic { version, parts });
    }

    // Not equal
    if let Some(version_str) = s.strip_prefix("!=") {
        let version = parse_version(version_str.trim())?;
        return Ok(VersionRange::NotEqual(version));
    }

    // Greater than or equal
    if let Some(version_str) = s.strip_prefix(">=") {
        let version = parse_version(version_str.trim())?;
        return Ok(VersionRange::GreaterThanOrEqual(version));
    }

    // Less than or equal
    if let Some(version_str) = s.strip_prefix("<=") {
        let version = parse_version(version_str.trim())?;
        return Ok(VersionRange::LessThanOrEqual(version));
    }

    // Greater than
    if let Some(version_str) = s.strip_prefix('>') {
        let version = parse_version(version_str.trim())?;
        return Ok(VersionRange::GreaterThan(version));
    }

    // Less than
    if let Some(version_str) = s.strip_prefix('<') {
        let version = parse_version(version_str.trim())?;
        return Ok(VersionRange::LessThan(version));
    }

    // Exact (with or without = prefix)
    let version_str = s.strip_prefix('=').unwrap_or(s).trim();
    let version = parse_version(version_str)?;
    Ok(VersionRange::Exact(version))
}

/// Parse a version string, handling incomplete versions.
fn parse_version(s: &str) -> Result<semver::Version, crate::error::DriftOpsError> {
    // Handle versions like "1.0" by appending ".0"
    let normalized = match s.matches('.').count() {
        0 => format!("{s}.0.0"),
        1 => format!("{s}.0"),
        _ => s.to_string(),
    };

    // Remove any 'v' prefix
    let normalized = normalized.strip_prefix('v').unwrap_or(&normalized);

    semver::Version::parse(normalized).map_err(|e| {
        crate::error::DriftOpsError::VersionParse {
            version: s.to_string(),
            source: e,
        }
    })
}

/// Calculate the next version (for > constraint).
fn next_version(v: &semver::Version) -> semver::Version {
    semver::Version::new(v.major, v.minor, v.patch + 1)
}

/// Calculate the previous version (for < constraint).
fn prev_version(v: &semver::Version) -> semver::Version {
    if v.patch > 0 {
        semver::Version::new(v.major, v.minor, v.patch - 1)
    } else if v.minor > 0 {
        semver::Version::new(v.major, v.minor - 1, u64::MAX)
    } else if v.major > 0 {
        semver::Version::new(v.major - 1, u64::MAX, u64::MAX)
    } else {
        semver::Version::new(0, 0, 0)
    }
}

/// Calculate the upper bound for a pessimistic constraint.
fn pessimistic_upper_bound(v: &semver::Version, parts: usize) -> semver::Version {
    // ~> X.Y.Z allows < X.(Y+1).0
    // ~> X.Y allows < X.(Y+1).0

    match parts {
        3 => semver::Version::new(v.major, v.minor + 1, 0),
        2 => semver::Version::new(v.major + 1, 0, 0),
        _ => semver::Version::new(u64::MAX, u64::MAX, u64::MAX), // means infinite upper bound at semver eg X
    }
}
/// Report output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
pub enum ReportFormat {
    /// JSON format
    #[default]
    Json,
    /// Plain text format
    Text,
    /// Self-contained HTML report
    Html,
}

/// Graph output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
pub enum GraphFormat {
    /// DOT format (Graphviz)
    #[default]
    Dot,
    /// JSON format
    Json,
    /// Mermaid diagram format
    Mermaid,
}

/// Severity level for findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational finding
    Info,
    /// Warning - potential issue
    Warning,
    /// Error - definite problem
    Error,
    /// Critical - severe issue requiring immediate attention
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Result of scanning and analyzing Terraform/OpenTofu files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// All discovered module references
    pub modules: Vec<ModuleRef>,

    /// All discovered provider requirements
    pub providers: Vec<ProviderRef>,

    /// All discovered runtime environments
    pub runtimes: Vec<RuntimeRef>,

    /// List of files that were scanned
    pub files_scanned: Vec<PathBuf>,

    /// The dependency graph
    #[serde(skip)]
    pub graph: DependencyGraph,

    /// Analysis results
    pub analysis: AnalysisResult,
}

impl Default for ScanResult {
    fn default() -> Self {
        Self {
            modules: Vec::new(),
            providers: Vec::new(),
            runtimes: Vec::new(),
            files_scanned: Vec::new(),
            graph: DependencyGraph::new(),
            analysis: AnalysisResult::default(),
        }
    }
}

impl ScanResult {
    /// Merge another scan result into this one.
    pub fn merge(&mut self, other: Self) {
        self.modules.extend(other.modules);
        self.providers.extend(other.providers);
        self.files_scanned.extend(other.files_scanned);
        self.graph.merge(other.graph);
        self.analysis.merge(other.analysis);
    }

    /// Generate a report in the specified format.
    ///
    /// # Errors
    ///
    /// Returns an error if report generation fails.
    pub fn generate_report(&self, format: ReportFormat) -> crate::Result<String> {
        let config = crate::Config::default();
        let reporter = crate::reporter::Reporter::new(&config);
        reporter.generate(self, format)
    }
}

/// Results from the constraint analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Individual findings from the analysis
    pub findings: Vec<Finding>,

    /// Summary statistics
    pub summary: AnalysisSummary,

    /// Deprecation results
    pub deprecations: DeprecationResult,

    /// Timestamp of the analysis
    pub timestamp: Option<DateTime<Utc>>,
}

impl AnalysisResult {
    /// Check if there are any error-level findings.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f.severity, Severity::Error | Severity::Critical))
    }

    /// Check if there are any warning-level findings.
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f.severity, Severity::Warning))
    }

    /// Get findings filtered by severity.
    #[must_use]
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect()
    }

    /// Merge another analysis result into this one.
    pub fn merge(&mut self, other: Self) {
        self.findings.extend(other.findings);
        self.summary.merge(other.summary);
    }
}

/// A single finding from the analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier for this type of finding
    pub code: String,

    /// Severity level
    pub severity: Severity,

    /// Human-readable message
    pub message: String,

    /// Detailed description
    pub description: Option<String>,

    /// File location
    pub location: Option<Location>,

    /// Related locations (e.g., conflicting constraints)
    #[serde(default)]
    pub related_locations: Vec<Location>,

    /// Suggested fix
    pub suggestion: Option<String>,

    /// Category of the finding
    pub category: FindingCategory,
}

/// Location in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// File path
    pub file: PathBuf,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based, optional)
    pub column: Option<usize>,
    /// Repository name (optional)
    pub repository: Option<String>,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(repo) = &self.repository {
            write!(f, "{repo}:")?;
        }
        write!(f, "{}:{}", self.file.display(), self.line)?;
        if let Some(col) = self.column {
            write!(f, ":{col}")?;
        }
        Ok(())
    }
}

/// Category of findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingCategory {
    /// Version constraint conflict
    ConstraintConflict,
    /// Missing version constraint
    MissingConstraint,
    /// Overly broad constraint
    BroadConstraint,
    /// Deprecated module/provider
    Deprecated,
    /// Outdated version
    Outdated,
    /// Security concern
    Security,
    /// Best practice violation
    BestPractice,
    /// Configuration issue
    Configuration,
}

impl std::fmt::Display for FindingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstraintConflict => write!(f, "Constraint Conflict"),
            Self::MissingConstraint => write!(f, "Missing Constraint"),
            Self::BroadConstraint => write!(f, "Broad Constraint"),
            Self::Deprecated => write!(f, "Deprecated"),
            Self::Outdated => write!(f, "Outdated"),
            Self::Security => write!(f, "Security"),
            Self::BestPractice => write!(f, "Best Practice"),
            Self::Configuration => write!(f, "Configuration"),
        }
    }
}

/// Summary statistics from analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Total number of modules found
    pub total_modules: usize,
    /// Total number of providers found
    pub total_providers: usize,
    /// Total number of files scanned
    pub total_files: usize,
    /// Number of unique module sources
    pub unique_module_sources: usize,
    /// Number of unique provider sources
    pub unique_provider_sources: usize,
    /// Counts by severity
    pub findings_by_severity: HashMap<String, usize>,
    /// Counts by category
    pub findings_by_category: HashMap<String, usize>,
}

/// Deprecation results. Used to track deprecated runtimes, modules, and providers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeprecationResult {
    /// Runtimes found
    pub runtimes: Vec<RuntimeRef>,

    /// Modules found
    pub modules: Vec<ModuleRef>,

    /// Providers found
    pub providers: Vec<ProviderRef>,

    /// Number of unique runtime sources
    pub unique_runtime_sources: HashSet<String>,

    /// Number of unique module sources
    pub unique_module_sources: HashSet<String>,

    /// Number of unique provider sources
    pub unique_provider_sources: HashSet<String>,
}

impl AnalysisSummary {
    /// Merge another summary into this one.
    pub fn merge(&mut self, other: Self) {
        self.total_modules += other.total_modules;
        self.total_providers += other.total_providers;
        self.total_files += other.total_files;
        // Note: unique counts need recalculation after merge
        for (k, v) in other.findings_by_severity {
            *self.findings_by_severity.entry(k).or_insert(0) += v;
        }
        for (k, v) in other.findings_by_category {
            *self.findings_by_category.entry(k).or_insert(0) += v;
        }
    }
}

/// VCS platform-agnostic identifier for tracking module ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VcsIdentifier {
    /// Canonical format: "vcs:platform:namespace/path"
    /// Examples:
    /// - "vcs:github:hashicorp/terraform"
    /// - "vcs:gitlab:gitlab-org/gitlab"
    /// - "vcs:ado:myorg/myproject/myrepo"
    /// - "vcs:bitbucket:atlassian/bitbucket"
    /// - "vcs:local" (for local filesystem modules)
    pub canonical: String,
    /// Parsed namespace components
    pub components: Vec<String>,
}

impl VcsIdentifier {
    /// Create a VCS identifier from a URL and platform.
    #[must_use]
    pub fn from_url(url: &str, platform: &str) -> Option<Self> {
        let components = Self::parse_namespace_from_url(url, platform)?;
        let canonical = format!("vcs:{}:{}", platform, components.join("/"));
        Some(Self {
            canonical,
            components,
        })
    }

    /// Create a local VCS identifier for filesystem modules.
    #[must_use]
    pub fn local() -> Self {
        Self {
            canonical: "vcs:local".to_string(),
            components: vec!["local".to_string()],
        }
    }

    /// Parse namespace components from a VCS URL.
    fn parse_namespace_from_url(url: &str, platform: &str) -> Option<Vec<String>> {
        match platform {
            "github" => Self::parse_github_url(url),
            "gitlab" => Self::parse_gitlab_url(url),
            "ado" => Self::parse_azure_devops_url(url),
            "bitbucket" => Self::parse_bitbucket_url(url),
            _ => None,
        }
    }

    /// Parse GitHub URL: https://github.com/owner/repo -> ["owner", "repo"]
    fn parse_github_url(url: &str) -> Option<Vec<String>> {
        let url = url.trim_end_matches(".git");
        let parts: Vec<&str> = url.split('/').collect();

        // Find "github.com" in the URL
        let github_idx = parts.iter().position(|&p| p == "github.com")?;

        if parts.len() > github_idx + 2 {
            Some(vec![
                parts[github_idx + 1].to_string(),
                parts[github_idx + 2].to_string(),
            ])
        } else {
            None
        }
    }

    /// Parse GitLab URL: https://gitlab.com/group/subgroup/repo -> ["group", "subgroup", "repo"]
    fn parse_gitlab_url(url: &str) -> Option<Vec<String>> {
        let url = url.trim_end_matches(".git");
        let parts: Vec<&str> = url.split('/').collect();

        let gitlab_idx = parts.iter().position(|&p| p == "gitlab.com")?;

        if parts.len() > gitlab_idx + 1 {
            let mut components = Vec::new();
            for &part in &parts[gitlab_idx + 1..] {
                if !part.is_empty() {
                    components.push(part.to_string());
                }
            }
            Some(components)
        } else {
            None
        }
    }

    /// Parse Azure DevOps URL: https://dev.azure.com/org/project/_git/repo -> ["org", "project", "repo"]
    fn parse_azure_devops_url(url: &str) -> Option<Vec<String>> {
        let url = url.trim_end_matches(".git");
        let parts: Vec<&str> = url.split('/').collect();

        let dev_idx = parts.iter().position(|&p| p == "dev.azure.com")?;

        if parts.len() > dev_idx + 3 && parts.get(dev_idx + 3) == Some(&"_git") {
            Some(vec![
                parts[dev_idx + 1].to_string(),
                parts[dev_idx + 2].to_string(),
                parts[dev_idx + 4].to_string(),
            ])
        } else {
            None
        }
    }

    /// Parse Bitbucket URL: https://bitbucket.org/workspace/repo -> ["workspace", "repo"]
    fn parse_bitbucket_url(url: &str) -> Option<Vec<String>> {
        let url = url.trim_end_matches(".git");
        let parts: Vec<&str> = url.split('/').collect();

        let bitbucket_idx = parts.iter().position(|&p| p == "bitbucket.org")?;

        if parts.len() > bitbucket_idx + 2 {
            Some(vec![
                parts[bitbucket_idx + 1].to_string(),
                parts[bitbucket_idx + 2].to_string(),
            ])
        } else {
            None
        }
    }
}
/// Parsed HCL file contents.
#[derive(Debug, Clone, Default)]
pub struct ParsedHcl {
    /// Module references found in this file
    pub modules: Vec<ModuleRef>,
    /// Provider requirements found in this file
    pub providers: Vec<ProviderRef>,
    /// Files that were parsed
    pub files: Vec<PathBuf>,

    /// Runtimes found in this file
    pub runtimes: Vec<RuntimeRef>,
}

impl ParsedHcl {
    /// Merge another parsed result into this one.
    pub fn merge(&mut self, other: Self) {
        self.modules.extend(other.modules);
        self.providers.extend(other.providers);
        self.files.extend(other.files);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_parse_exact() {
        let c = Constraint::parse("1.0.0").unwrap();
        assert_eq!(c.ranges.len(), 1);
        assert!(matches!(&c.ranges[0], VersionRange::Exact(v) if v.major == 1));
    }

    #[test]
    fn test_constraint_parse_pessimistic() {
        let c = Constraint::parse("~> 1.0").unwrap();
        assert_eq!(c.ranges.len(), 1);

        if let VersionRange::Pessimistic { version, parts} = &c.ranges[0] {
            assert_eq!(version.major, 1);
            assert_eq!(version.minor, 0);
            assert_eq!(version.patch, 0);
            assert_eq!(*parts, 2);
        } else {
            panic!("Expected pessimistic version range");
        }
    }

    #[test]
    fn test_constraint_parse_pessimistic_3_parts() {
        let c = Constraint::parse("~> 1.3.59").unwrap();
        assert_eq!(c.ranges.len(), 1);
        if let VersionRange::Pessimistic { version, parts} = &c.ranges[0] {
            assert_eq!(version.major, 1);
            assert_eq!(version.minor, 3);
            assert_eq!(version.patch, 59);
            assert_eq!(*parts, 3);
        } else {
            panic!("Expected pessimistic version range");
        }
    }

    #[test]
    fn test_constraint_parse_multiple() {
        let c = Constraint::parse(">= 1.0, < 2.0").unwrap();
        assert_eq!(c.ranges.len(), 2);
    }

    #[test]
    fn test_constraint_satisfaction() {
        let c = Constraint::parse(">= 1.0.0, < 2.0.0").unwrap();
        assert!(c.is_satisfied_by(&semver::Version::new(1, 5, 0)));
        assert!(!c.is_satisfied_by(&semver::Version::new(0, 9, 0)));
        assert!(!c.is_satisfied_by(&semver::Version::new(2, 0, 0)));
    }

    #[test]
    fn test_constraint_conflict_detection() {
        let c1 = Constraint::parse(">= 5.0.0").unwrap();
        let c2 = Constraint::parse("<= 4.5.0").unwrap();
        assert!(c1.conflicts_with(&c2));
    }

    #[test]
    fn test_constraint_no_conflict() {
        let c1 = Constraint::parse(">= 1.0.0").unwrap();
        let c2 = Constraint::parse("<= 2.0.0").unwrap();
        assert!(!c1.conflicts_with(&c2));
    }

    #[test]
    fn test_pessimistic_constraint() {
        let c = Constraint::parse("~> 1.2.0").unwrap();
        assert!(c.is_satisfied_by(&semver::Version::new(1, 2, 5)));
        assert!(!c.is_satisfied_by(&semver::Version::new(1, 3, 0)));
    }

    #[test]
    fn test_module_source_canonical_id() {
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
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }
}

