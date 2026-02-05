//! JSON report generator with clean, well-organized machine-readable output.
//!
//! Designed for easy parsing by CI/CD tools, dashboards, and automation scripts.
//! Structure prioritizes: status first, then grouped findings, then inventory.

use crate::config::Config;
use crate::error::Result;
use crate::reporter::ReportGenerator;
use crate::types::{ScanResult, ScanWarning, Severity};
use serde::Serialize;
use std::collections::HashMap;

/// JSON report generator.
pub struct JsonReporter {
    /// Whether to pretty-print the output
    pretty: bool,
}

impl JsonReporter {
    /// Create a new JSON reporter.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            pretty: config.output.pretty,
        }
    }
}

impl ReportGenerator for JsonReporter {
    fn generate(&self, result: &ScanResult) -> Result<String> {
        let report = JsonReport::from(result);

        let json = if self.pretty {
            serde_json::to_string_pretty(&report)
        } else {
            serde_json::to_string(&report)
        };

        json.map_err(|e| {
            crate::err!(ReportGeneration {
                message: format!("Failed to serialize JSON report: {e}"),
            })
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// JSON REPORT STRUCTURE
// ═══════════════════════════════════════════════════════════════════════════════

/// Top-level JSON report structure.
///
/// Designed for easy consumption:
/// 1. Check `status` first for pass/fail
/// 2. Look at `summary` for quick counts
/// 3. Dive into `findings` grouped by repo/file for details
/// 4. Reference `inventory` for full module/provider lists
#[derive(Debug, Serialize)]
pub struct JsonReport {
    /// Report metadata (version, timestamp, etc.)
    pub meta: ReportMeta,

    /// Overall status - check this first!
    pub status: ReportStatus,

    /// Quick summary counts
    pub summary: ReportSummary,

    /// Scan warnings (e.g., unparseable files) - check if items were skipped
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scan_warnings: Vec<JsonScanWarning>,

    /// Findings grouped by repository for easy navigation
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<RepoFindings>,

    /// Full inventory of modules and providers
    pub inventory: Inventory,
}

impl From<&ScanResult> for JsonReport {
    fn from(result: &ScanResult) -> Self {
        let errors = result
            .analysis
            .findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
            .count();
        let warnings = result
            .analysis
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let infos = result
            .analysis
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count();

        // Group findings by repository
        let findings = group_findings_by_repo(result);

        Self {
            meta: ReportMeta {
                tool: "monphare".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                files_scanned: result.files_scanned.len(),
            },
            status: ReportStatus {
                passed: !result.analysis.has_errors(),
                exit_code: if result.analysis.has_errors() {
                    2
                } else if result.analysis.has_warnings() {
                    1
                } else {
                    0
                },
                message: if result.analysis.has_errors() {
                    "Errors found - action required".to_string()
                } else if result.analysis.has_warnings() {
                    "Passed with warnings".to_string()
                } else {
                    "All checks passed".to_string()
                },
            },
            summary: ReportSummary {
                total_findings: result.analysis.findings.len(),
                errors,
                warnings,
                infos,
                modules: ModuleSummary {
                    total: result.modules.len(),
                    with_issues: result
                        .modules
                        .iter()
                        .filter(|m| {
                            let pattern = format!("'{}'", m.name);
                            result
                                .analysis
                                .findings
                                .iter()
                                .any(|f| f.message.contains(&pattern))
                        })
                        .count(),
                    local: result
                        .modules
                        .iter()
                        .filter(|m| m.source.is_local())
                        .count(),
                },
                providers: ProviderSummary {
                    total: result.providers.len(),
                    with_issues: result
                        .providers
                        .iter()
                        .filter(|p| {
                            let pattern = format!("'{}'", p.name);
                            result
                                .analysis
                                .findings
                                .iter()
                                .any(|f| f.message.contains(&pattern))
                        })
                        .count(),
                },
                repositories: result
                    .modules
                    .iter()
                    .filter_map(|m| m.repository.as_ref())
                    .chain(
                        result
                            .providers
                            .iter()
                            .filter_map(|p| p.repository.as_ref()),
                    )
                    .collect::<std::collections::HashSet<_>>()
                    .len(),
            },
            scan_warnings: result.warnings.iter().map(JsonScanWarning::from).collect(),
            findings,
            inventory: Inventory {
                modules: result.modules.iter().map(JsonModule::from).collect(),
                providers: result.providers.iter().map(JsonProvider::from).collect(),
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// METADATA & STATUS
// ═══════════════════════════════════════════════════════════════════════════════

/// Report metadata.
#[derive(Debug, Serialize)]
pub struct ReportMeta {
    /// Tool name
    pub tool: String,
    /// MonPhare version
    pub version: String,
    /// Report generation timestamp (RFC 3339)
    pub timestamp: String,
    /// Number of files scanned
    pub files_scanned: usize,
}

/// Overall report status - check this first for pass/fail.
#[derive(Debug, Serialize)]
pub struct ReportStatus {
    /// Whether the scan passed (no errors)
    pub passed: bool,
    /// Suggested exit code (0=pass, 1=warnings, 2=errors)
    pub exit_code: u8,
    /// Human-readable status message
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCAN WARNINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// Scan warning for items that were skipped (e.g., unparseable constraints).
#[derive(Debug, Serialize)]
pub struct JsonScanWarning {
    /// Warning code (e.g., "unparseable-constraint")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// File where the issue occurred
    pub file: String,
    /// Line number (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Repository name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl From<&ScanWarning> for JsonScanWarning {
    fn from(warning: &ScanWarning) -> Self {
        Self {
            code: warning.code.clone(),
            message: warning.message.clone(),
            file: warning.file.display().to_string(),
            line: warning.line,
            repository: warning.repository.clone(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUMMARY
// ═══════════════════════════════════════════════════════════════════════════════

/// Quick summary counts.
#[derive(Debug, Serialize)]
pub struct ReportSummary {
    /// Total findings across all severities
    pub total_findings: usize,
    /// Error count (action required)
    pub errors: usize,
    /// Warning count
    pub warnings: usize,
    /// Info count
    pub infos: usize,
    /// Module statistics
    pub modules: ModuleSummary,
    /// Provider statistics
    pub providers: ProviderSummary,
    /// Number of unique repositories scanned
    pub repositories: usize,
}

/// Module summary statistics.
#[derive(Debug, Serialize)]
pub struct ModuleSummary {
    /// Total modules found
    pub total: usize,
    /// Modules with issues
    pub with_issues: usize,
    /// Local modules (excluded from version checks)
    pub local: usize,
}

/// Provider summary statistics.
#[derive(Debug, Serialize)]
pub struct ProviderSummary {
    /// Total providers found
    pub total: usize,
    /// Providers with issues
    pub with_issues: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// FINDINGS (GROUPED BY REPOSITORY)
// ═══════════════════════════════════════════════════════════════════════════════

/// Findings grouped by repository.
#[derive(Debug, Serialize)]
pub struct RepoFindings {
    /// Repository name
    pub repository: String,
    /// Error count in this repo
    pub errors: usize,
    /// Warning count in this repo
    pub warnings: usize,
    /// Findings grouped by file within this repo
    pub files: Vec<FileFindings>,
}

/// Findings grouped by file.
#[derive(Debug, Serialize)]
pub struct FileFindings {
    /// File path (relative to repo root)
    pub path: String,
    /// Findings in this file
    pub findings: Vec<JsonFinding>,
}

/// A single finding with all relevant details.
#[derive(Debug, Serialize)]
pub struct JsonFinding {
    /// Finding code (e.g., "missing-version")
    pub code: String,
    /// Severity level
    pub severity: String,
    /// Category
    pub category: String,
    /// Line number in the file
    pub line: usize,
    /// Short, actionable message
    pub message: String,
    /// Detailed description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Suggested fix (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Related locations for context (e.g., conflicting constraints)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<RelatedLocation>,
}

/// A related location (for cross-references).
#[derive(Debug, Serialize)]
pub struct RelatedLocation {
    /// Repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// File path
    pub path: String,
    /// Line number
    pub line: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// INVENTORY
// ═══════════════════════════════════════════════════════════════════════════════

/// Full inventory of modules and providers.
#[derive(Debug, Serialize)]
pub struct Inventory {
    /// All modules found
    pub modules: Vec<JsonModule>,
    /// All providers found
    pub providers: Vec<JsonProvider>,
}

/// JSON representation of a module.
#[derive(Debug, Serialize)]
pub struct JsonModule {
    /// Module name (the label in `module "name" {}`)
    pub name: String,
    /// Module source (registry, git, local, etc.)
    pub source: JsonModuleSource,
    /// Version constraint (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Location where this module is defined
    pub location: JsonLocation,
    /// Whether this module has issues
    pub has_issues: bool,
}

/// Module source details.
#[derive(Debug, Serialize)]
pub struct JsonModuleSource {
    /// Source type (registry, git, local, s3, etc.)
    #[serde(rename = "type")]
    pub source_type: String,
    /// Canonical source identifier
    pub canonical: String,
    /// Whether this is a local module
    pub is_local: bool,
}

impl From<&crate::types::ModuleRef> for JsonModule {
    fn from(module: &crate::types::ModuleRef) -> Self {
        let source_type = match &module.source {
            crate::types::ModuleSource::Registry { .. } => "registry",
            crate::types::ModuleSource::Git { .. } => "git",
            crate::types::ModuleSource::Local { .. } => "local",
            crate::types::ModuleSource::Http { .. } => "http",
            crate::types::ModuleSource::S3 { .. } => "s3",
            crate::types::ModuleSource::Gcs { .. } => "gcs",
            crate::types::ModuleSource::Unknown(_) => "unknown",
        };

        Self {
            name: module.name.clone(),
            source: JsonModuleSource {
                source_type: source_type.to_string(),
                canonical: module.source.canonical_id(),
                is_local: module.source.is_local(),
            },
            version: module.version_constraint.as_ref().map(|c| c.raw.clone()),
            location: JsonLocation {
                repository: module.repository.clone(),
                path: module.file_path.to_string_lossy().to_string(),
                line: module.line_number,
            },
            has_issues: false, // Will be set during report generation
        }
    }
}

/// JSON representation of a provider.
#[derive(Debug, Serialize)]
pub struct JsonProvider {
    /// Provider name (e.g., "aws")
    pub name: String,
    /// Provider source (e.g., "hashicorp/aws")
    pub source: String,
    /// Version constraint (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Location where this provider is required
    pub location: JsonLocation,
    /// Whether this provider has issues
    pub has_issues: bool,
}

impl From<&crate::types::ProviderRef> for JsonProvider {
    fn from(provider: &crate::types::ProviderRef) -> Self {
        Self {
            name: provider.name.clone(),
            source: provider.qualified_source(),
            version: provider.version_constraint.as_ref().map(|c| c.raw.clone()),
            location: JsonLocation {
                repository: provider.repository.clone(),
                path: provider.file_path.to_string_lossy().to_string(),
                line: provider.line_number,
            },
            has_issues: false, // Will be set during report generation
        }
    }
}

/// Location information.
#[derive(Debug, Serialize)]
pub struct JsonLocation {
    /// Repository name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// File path
    pub path: String,
    /// Line number
    pub line: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Group findings by repository, then by file.
fn group_findings_by_repo(result: &ScanResult) -> Vec<RepoFindings> {
    let mut by_repo: HashMap<String, HashMap<String, Vec<JsonFinding>>> = HashMap::new();

    for finding in &result.analysis.findings {
        let (repo, file, line) = if let Some(loc) = &finding.location {
            (
                loc.repository
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                loc.file.to_string_lossy().to_string(),
                loc.line,
            )
        } else {
            ("unknown".to_string(), "unknown".to_string(), 0)
        };

        let json_finding = JsonFinding {
            code: finding.code.clone(),
            severity: finding.severity.to_string().to_lowercase(),
            category: finding.category.to_string(),
            line,
            message: finding.message.clone(),
            description: finding.description.clone(),
            suggestion: finding.suggestion.clone(),
            related: finding
                .related_locations
                .iter()
                .map(|loc| RelatedLocation {
                    repository: loc.repository.clone(),
                    path: loc.file.to_string_lossy().to_string(),
                    line: loc.line,
                })
                .collect(),
        };

        by_repo
            .entry(repo)
            .or_default()
            .entry(file)
            .or_default()
            .push(json_finding);
    }

    let mut repos: Vec<RepoFindings> = by_repo
        .into_iter()
        .map(|(repo, files)| {
            let file_findings: Vec<FileFindings> = files
                .into_iter()
                .map(|(path, findings)| FileFindings { path, findings })
                .collect();

            let errors = file_findings
                .iter()
                .flat_map(|f| &f.findings)
                .filter(|f| f.severity == "error" || f.severity == "critical")
                .count();
            let warnings = file_findings
                .iter()
                .flat_map(|f| &f.findings)
                .filter(|f| f.severity == "warning")
                .count();

            RepoFindings {
                repository: repo,
                errors,
                warnings,
                files: file_findings,
            }
        })
        .collect();

    // Sort by error count descending
    repos.sort_by(|a, b| b.errors.cmp(&a.errors).then(b.warnings.cmp(&a.warnings)));

    repos
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AnalysisResult, Constraint, ModuleRef, ModuleSource, ProviderRef, RuntimeRef, RuntimeSource,
    };
    use std::path::PathBuf;

    fn create_test_result() -> ScanResult {
        ScanResult {
            modules: vec![ModuleRef {
                name: "vpc".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "vpc".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: None,
                file_path: PathBuf::from("main.tf"),
                line_number: 1,
                repository: Some("test-repo".to_string()),
                attributes: Default::default(),
            }],
            providers: vec![ProviderRef {
                name: "aws".to_string(),
                source: Some("hashicorp/aws".to_string()),
                version_constraint: None,
                file_path: PathBuf::from("versions.tf"),
                line_number: 1,
                repository: Some("test-repo".to_string()),
            }],
            runtimes: vec![RuntimeRef {
                name: "terraform".to_string(),
                version: Constraint::parse("1.0.0").unwrap(),
                source: RuntimeSource::Terraform,
                file_path: PathBuf::from("main.tf"),
                line_number: 1,
                repository: Some("test-repo".to_string()),
            }],
            files_scanned: vec![PathBuf::from("main.tf"), PathBuf::from("versions.tf")],
            graph: Default::default(),
            analysis: AnalysisResult::default(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn test_json_report_generation() {
        let result = create_test_result();
        let config = Config::default();
        let reporter = JsonReporter::new(&config);

        let json = reporter.generate(&result).unwrap();

        // Parse to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["status"]["passed"].as_bool().unwrap());
        assert_eq!(parsed["status"]["exit_code"].as_u64().unwrap(), 0);
        assert!(parsed["meta"]["version"].is_string());
        assert_eq!(parsed["summary"]["modules"]["total"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_json_report_pretty() {
        let result = create_test_result();
        let mut config = Config::default();
        config.output.pretty = true;

        let reporter = JsonReporter::new(&config);
        let json = reporter.generate(&result).unwrap();

        // Pretty output should have newlines
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_json_report_structure() {
        let result = create_test_result();
        let config = Config::default();
        let reporter = JsonReporter::new(&config);

        let json = reporter.generate(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Check top-level structure
        assert!(parsed["meta"].is_object());
        assert!(parsed["status"].is_object());
        assert!(parsed["summary"].is_object());
        assert!(parsed["inventory"].is_object());

        // Check inventory structure
        assert!(parsed["inventory"]["modules"].is_array());
        assert!(parsed["inventory"]["providers"].is_array());
    }
}
