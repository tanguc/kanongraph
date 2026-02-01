//! JSON report generator.

use crate::config::Config;
use crate::error::{MonPhareError, Result};
use crate::reporter::ReportGenerator;
use crate::types::ScanResult;
use serde::Serialize;

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

        json.map_err(|e| crate::err!(ReportGeneration {
            message: format!("Failed to serialize JSON report: {e}"),
        }))
    }
}

/// JSON report structure.
#[derive(Debug, Serialize)]
pub struct JsonReport {
    /// Report metadata
    pub metadata: ReportMetadata,
    /// Summary statistics
    pub summary: ReportSummary,
    /// All findings
    pub findings: Vec<JsonFinding>,
    /// Module details
    pub modules: Vec<JsonModule>,
    /// Provider details
    pub providers: Vec<JsonProvider>,
}

impl From<&ScanResult> for JsonReport {
    fn from(result: &ScanResult) -> Self {
        Self {
            metadata: ReportMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                files_scanned: result.files_scanned.len(),
            },
            summary: ReportSummary {
                total_modules: result.modules.len(),
                total_providers: result.providers.len(),
                total_findings: result.analysis.findings.len(),
                findings_by_severity: result
                    .analysis
                    .summary
                    .findings_by_severity
                    .clone(),
                findings_by_category: result
                    .analysis
                    .summary
                    .findings_by_category
                    .clone(),
                has_errors: result.analysis.has_errors(),
                has_warnings: result.analysis.has_warnings(),
            },
            findings: result
                .analysis
                .findings
                .iter()
                .map(JsonFinding::from)
                .collect(),
            modules: result.modules.iter().map(JsonModule::from).collect(),
            providers: result.providers.iter().map(JsonProvider::from).collect(),
        }
    }
}

/// Report metadata.
#[derive(Debug, Serialize)]
pub struct ReportMetadata {
    /// MonPhare version
    pub version: String,
    /// Report generation timestamp
    pub timestamp: String,
    /// Number of files scanned
    pub files_scanned: usize,
}

/// Report summary.
#[derive(Debug, Serialize)]
pub struct ReportSummary {
    /// Total modules found
    pub total_modules: usize,
    /// Total providers found
    pub total_providers: usize,
    /// Total findings
    pub total_findings: usize,
    /// Findings grouped by severity
    pub findings_by_severity: std::collections::HashMap<String, usize>,
    /// Findings grouped by category
    pub findings_by_category: std::collections::HashMap<String, usize>,
    /// Whether there are error-level findings
    pub has_errors: bool,
    /// Whether there are warning-level findings
    pub has_warnings: bool,
}

/// JSON representation of a finding.
#[derive(Debug, Serialize)]
pub struct JsonFinding {
    /// Finding code
    pub code: String,
    /// Severity level
    pub severity: String,
    /// Short message
    pub message: String,
    /// Detailed description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Primary location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<JsonLocation>,
    /// Related locations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_locations: Vec<JsonLocation>,
    /// Suggested fix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Finding category
    pub category: String,
}

impl From<&crate::types::Finding> for JsonFinding {
    fn from(finding: &crate::types::Finding) -> Self {
        Self {
            code: finding.code.clone(),
            severity: finding.severity.to_string(),
            message: finding.message.clone(),
            description: finding.description.clone(),
            location: finding.location.as_ref().map(JsonLocation::from),
            related_locations: finding
                .related_locations
                .iter()
                .map(JsonLocation::from)
                .collect(),
            suggestion: finding.suggestion.clone(),
            category: finding.category.to_string(),
        }
    }
}

/// JSON representation of a location.
#[derive(Debug, Serialize)]
pub struct JsonLocation {
    /// File path
    pub file: String,
    /// Line number
    pub line: usize,
    /// Column number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Repository name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl From<&crate::types::Location> for JsonLocation {
    fn from(loc: &crate::types::Location) -> Self {
        Self {
            file: loc.file.to_string_lossy().to_string(),
            line: loc.line,
            column: loc.column,
            repository: loc.repository.clone(),
        }
    }
}

/// JSON representation of a module.
#[derive(Debug, Serialize)]
pub struct JsonModule {
    /// Module name
    pub name: String,
    /// Module source
    pub source: String,
    /// Version constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_constraint: Option<String>,
    /// File path
    pub file_path: String,
    /// Line number
    pub line_number: usize,
    /// Repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl From<&crate::types::ModuleRef> for JsonModule {
    fn from(module: &crate::types::ModuleRef) -> Self {
        Self {
            name: module.name.clone(),
            source: module.source.canonical_id(),
            version_constraint: module.version_constraint.as_ref().map(|c| c.raw.clone()),
            file_path: module.file_path.to_string_lossy().to_string(),
            line_number: module.line_number,
            repository: module.repository.clone(),
        }
    }
}

/// JSON representation of a provider.
#[derive(Debug, Serialize)]
pub struct JsonProvider {
    /// Provider name
    pub name: String,
    /// Provider source
    pub source: String,
    /// Version constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_constraint: Option<String>,
    /// File path
    pub file_path: String,
    /// Line number
    pub line_number: usize,
    /// Repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl From<&crate::types::ProviderRef> for JsonProvider {
    fn from(provider: &crate::types::ProviderRef) -> Self {
        Self {
            name: provider.name.clone(),
            source: provider.qualified_source(),
            version_constraint: provider.version_constraint.as_ref().map(|c| c.raw.clone()),
            file_path: provider.file_path.to_string_lossy().to_string(),
            line_number: provider.line_number,
            repository: provider.repository.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;
    use crate::{Constraint, VersionRange, types::{AnalysisResult, ModuleRef, ModuleSource, ProviderRef, RuntimeRef, RuntimeSource}};
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
                repository: Some("test".to_string()),
                attributes: Default::default(),
            }],
            providers: vec![ProviderRef {
                name: "aws".to_string(),
                source: Some("hashicorp/aws".to_string()),
                version_constraint: None,
                file_path: PathBuf::from("versions.tf"),
                line_number: 1,
                repository: Some("test".to_string()),
            }],
            runtimes: vec![RuntimeRef {
                name: "terraform".to_string(),
                version: Constraint::parse("1.0.0").unwrap(),
                source: RuntimeSource::Terraform,
                file_path: PathBuf::from("main.tf"),
                line_number: 1,
                repository: Some("test".to_string()),
            }],
            files_scanned: vec![PathBuf::from("main.tf"), PathBuf::from("versions.tf")],
            graph: Default::default(),
            analysis: AnalysisResult::default(),
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

        assert!(parsed["metadata"]["version"].is_string());
        assert!(parsed["summary"]["total_modules"].as_u64().unwrap() == 1);
        assert!(parsed["modules"].is_array());
        assert!(parsed["providers"].is_array());
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
}

