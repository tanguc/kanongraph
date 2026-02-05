//! Terraform policy and best practice analysis.
//!
//! This module implements policy checks and best practice analysis for
//! Terraform version constraints, including missing constraints, risky patterns,
//! and overly broad constraints.

use crate::analyzer::deprecation;
use crate::analyzer::patterns::{PatternChecker, RiskyPattern};
use crate::config::Config;
use crate::error::Result;
use crate::graph::DependencyGraph;
use crate::types::{
    AnalysisResult, AnalysisSummary, Finding, FindingCategory, Location, ModuleRef, ProviderRef,
    RuntimeRef, Severity,
};
use std::collections::HashMap;

/// Analyzer for Terraform policy and best practice checks.
///
/// # Analysis Phases
///
/// The analyzer performs the following checks:
///
/// ## Phase 1: Missing Constraints
///
/// Detects modules and providers without version constraints.
/// Version pinning is a best practice to ensure reproducible deployments.
///
/// ## Phase 2: Risky Patterns
///
/// Identifies problematic constraint patterns:
/// - Wildcard constraints (`*`)
/// - Pre-release versions (`1.0.0-beta`)
/// - Exact versions (prevents patch updates)
/// - Missing upper bounds (allows breaking changes)
///
/// ## Phase 3: Broad Constraints
///
/// Flags overly permissive constraints like `>= 0.0.0` that provide
/// no meaningful version control.
///
/// ## Phase 4: Deprecations
///
/// Checks for deprecated module/provider versions based on configuration
/// or inline CLI rules.
///
/// # Example
///
/// ```rust,no_run
/// use monphare::analyzer::Analyzer;
/// use monphare::Config;
///
/// let config = Config::default();
/// let analyzer = Analyzer::new(&config);
/// ```
pub struct Analyzer {
    _config: Config,
    pattern_checker: PatternChecker,
}

impl Analyzer {
    /// Create a new constraint analyzer.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            _config: config.clone(),
            pattern_checker: PatternChecker::new(config),
        }
    }

    /// Analyze modules and providers for policy violations and best practice issues.
    ///
    /// # Errors
    ///
    /// Returns an error if analysis fails.
    pub fn analyze(
        &self,
        _graph: &DependencyGraph,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
        runtimes: &[RuntimeRef],
    ) -> Result<AnalysisResult> {
        tracing::debug!(
            modules = modules.len(),
            providers = providers.len(),
            runtimes = runtimes.len(),
            "Starting policy analysis"
        );
        let mut findings = Vec::new();

        // Phase 1: Check for missing constraints
        tracing::debug!("Phase 1: Checking for missing constraints");
        let missing = self.check_missing_constraints(modules, providers);
        tracing::debug!(
            missing_constraints = missing.len(),
            "Missing constraints found"
        );
        findings.extend(missing);

        // Phase 2: Check for risky patterns
        tracing::debug!("Phase 2: Checking for risky patterns");
        let risky = self.check_risky_patterns(modules, providers);
        tracing::debug!(risky_patterns = risky.len(), "Risky patterns found");
        findings.extend(risky);

        // Phase 3: Check for broad constraints
        tracing::debug!("Phase 3: Checking for broad constraints");
        let broad = self.check_broad_constraints(modules, providers);
        tracing::debug!(broad_constraints = broad.len(), "Broad constraints found");
        findings.extend(broad);

        tracing::debug!("Checking deprecations");
        let deprecation_analyzer = deprecation::DeprecationAnalyzer::new(&self._config);
        let deprecations = deprecation_analyzer.analyze(modules, providers, runtimes);
        tracing::debug!(
            deprecated_modules = deprecations.modules.len(),
            deprecated_providers = deprecations.providers.len(),
            deprecated_runtimes = deprecations.runtimes.len(),
            "Deprecation analysis complete"
        );

        // Build summary
        tracing::debug!("Building analysis summary");
        let summary = self.build_summary(modules, providers, &findings);
        let error_count = summary
            .findings_by_severity
            .get("ERROR")
            .copied()
            .unwrap_or(0)
            + summary
                .findings_by_severity
                .get("CRITICAL")
                .copied()
                .unwrap_or(0);
        let warning_count = summary
            .findings_by_severity
            .get("WARNING")
            .copied()
            .unwrap_or(0);
        tracing::debug!(
            total_findings = findings.len(),
            errors = error_count,
            warnings = warning_count,
            "Analysis complete"
        );

        Ok(AnalysisResult {
            findings,
            summary,
            timestamp: Some(chrono::Utc::now()),
            deprecations,
        })
    }

    /// Check for missing version constraints.
    fn check_missing_constraints(
        &self,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check modules
        for module in modules {
            if module.source.is_local() {
                tracing::trace!(module = %module.name, "Local module, skipping constraint check");
                continue;
            }
            if module.version_constraint.is_none() && !module.source.is_local() {
                findings.push(Finding {
                    code: "missing-version".to_string(),
                    severity: Severity::Error,
                    message: format!("Module '{}' has no version constraint", module.name),
                    description: Some(
                        "Modules without version constraints may unexpectedly update \
                         to incompatible versions. Always specify a version constraint."
                            .to_string(),
                    ),
                    location: Some(Location {
                        file: module.file_path.clone(),
                        line: module.line_number,
                        column: None,
                        repository: module.repository.clone(),
                    }),
                    related_locations: vec![],
                    suggestion: Some(
                        "Add a version constraint, e.g., version = \"~> 1.0\"".to_string(),
                    ),
                    category: FindingCategory::MissingConstraint,
                });
            }
        }

        // Check providers
        for provider in providers {
            if provider.version_constraint.is_none() {
                findings.push(Finding {
                    code: "missing-version".to_string(),
                    severity: Severity::Error,
                    message: format!("Provider '{}' has no version constraint", provider.name),
                    description: Some(
                        "Providers without version constraints may unexpectedly update \
                         to incompatible versions. Always specify a version constraint."
                            .to_string(),
                    ),
                    location: Some(Location {
                        file: provider.file_path.clone(),
                        line: provider.line_number,
                        column: None,
                        repository: provider.repository.clone(),
                    }),
                    related_locations: vec![],
                    suggestion: Some(
                        "Add a version constraint, e.g., version = \">= 4.0, < 6.0\"".to_string(),
                    ),
                    category: FindingCategory::MissingConstraint,
                });
            }
        }

        findings
    }

    /// Check for risky constraint patterns.
    fn check_risky_patterns(
        &self,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check modules
        for module in modules {
            if module.source.is_local() {
                tracing::trace!(module = %module.name, "Local module, skipping risky pattern check");
                continue;
            }
            if let Some(constraint) = &module.version_constraint {
                for pattern in self.pattern_checker.check(&constraint.raw) {
                    findings.push(self.pattern_to_finding(
                        pattern,
                        &module.name,
                        &module.file_path,
                        module.line_number,
                        module.repository.as_deref(),
                    ));
                }
            }
        }

        // Check providers
        for provider in providers {
            if let Some(constraint) = &provider.version_constraint {
                for pattern in self.pattern_checker.check(&constraint.raw) {
                    findings.push(self.pattern_to_finding(
                        pattern,
                        &provider.name,
                        &provider.file_path,
                        provider.line_number,
                        provider.repository.as_deref(),
                    ));
                }
            }
        }

        findings
    }

    /// Check for overly broad constraints.
    fn check_broad_constraints(
        &self,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check modules
        for module in modules {
            if module.source.is_local() {
                tracing::trace!(module = %module.name, "Local module, skipping broad constraint check");
                continue;
            }
            if let Some(constraint) = &module.version_constraint {
                if constraint.is_overly_broad() {
                    findings.push(Finding {
                        code: "broad-constraint".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "Module '{}' has overly broad constraint: {}",
                            module.name, constraint.raw
                        ),
                        description: Some(
                            "Overly broad constraints like '>= 0.0.0' effectively allow \
                             any version and don't provide meaningful protection."
                                .to_string(),
                        ),
                        location: Some(Location {
                            file: module.file_path.clone(),
                            line: module.line_number,
                            column: None,
                            repository: module.repository.clone(),
                        }),
                        related_locations: vec![],
                        suggestion: Some(
                            "Use a more specific constraint like '~> 1.0' or '>= 1.0, < 2.0'"
                                .to_string(),
                        ),
                        category: FindingCategory::BroadConstraint,
                    });
                }
            }
        }

        // Check providers
        for provider in providers {
            if let Some(constraint) = &provider.version_constraint {
                if constraint.is_overly_broad() {
                    findings.push(Finding {
                        code: "broad-constraint".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "Provider '{}' has overly broad constraint: {}",
                            provider.name, constraint.raw
                        ),
                        description: Some(
                            "Overly broad constraints like '>= 0.0.0' effectively allow \
                             any version and don't provide meaningful protection."
                                .to_string(),
                        ),
                        location: Some(Location {
                            file: provider.file_path.clone(),
                            line: provider.line_number,
                            column: None,
                            repository: provider.repository.clone(),
                        }),
                        related_locations: vec![],
                        suggestion: Some(
                            "Use a more specific constraint like '>= 4.0, < 6.0'".to_string(),
                        ),
                        category: FindingCategory::BroadConstraint,
                    });
                }
            }
        }

        findings
    }

    /// Convert a risky pattern to a finding.
    fn pattern_to_finding(
        &self,
        pattern: RiskyPattern,
        name: &str,
        file: &std::path::Path,
        line: usize,
        repo: Option<&str>,
    ) -> Finding {
        let (code, severity, message, description, suggestion) = match pattern {
            RiskyPattern::Wildcard => (
                "wildcard-constraint",
                Severity::Warning,
                format!("'{name}' uses wildcard version constraint"),
                "Wildcard constraints like '*' allow any version and should be avoided.",
                "Replace with a specific constraint like '~> 1.0'",
            ),
            RiskyPattern::PreRelease => (
                "prerelease-version",
                Severity::Info,
                format!("'{name}' uses pre-release version"),
                "Pre-release versions may be unstable and are not recommended for production.",
                "Consider using a stable release version",
            ),
            RiskyPattern::ExactVersion => (
                "exact-version",
                Severity::Info,
                format!("'{name}' uses exact version constraint"),
                "Exact version constraints prevent automatic patch updates.",
                "Consider using '~> X.Y.0' to allow patch updates",
            ),
            RiskyPattern::NoUpperBound => (
                "no-upper-bound",
                Severity::Warning,
                format!("'{name}' has no upper bound on version"),
                "Constraints without upper bounds may allow breaking changes.",
                "Add an upper bound, e.g., '>= 1.0, < 2.0'",
            ),
        };

        Finding {
            code: code.to_string(),
            severity,
            message,
            description: Some(description.to_string()),
            location: Some(Location {
                file: file.to_path_buf(),
                line,
                column: None,
                repository: repo.map(String::from),
            }),
            related_locations: vec![],
            suggestion: Some(suggestion.to_string()),
            category: FindingCategory::BestPractice,
        }
    }

    /// Build analysis summary.
    fn build_summary(
        &self,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
        findings: &[Finding],
    ) -> AnalysisSummary {
        let mut findings_by_severity: HashMap<String, usize> = HashMap::new();
        let mut findings_by_category: HashMap<String, usize> = HashMap::new();

        for finding in findings {
            *findings_by_severity
                .entry(finding.severity.to_string())
                .or_insert(0) += 1;
            *findings_by_category
                .entry(finding.category.to_string())
                .or_insert(0) += 1;
        }

        // Count unique sources
        let unique_module_sources: std::collections::HashSet<_> =
            modules.iter().map(|m| m.source.canonical_id()).collect();
        let unique_provider_sources: std::collections::HashSet<_> =
            providers.iter().map(|p| p.qualified_source()).collect();

        AnalysisSummary {
            total_modules: modules.len(),
            total_providers: providers.len(),
            total_files: 0, // Set by caller
            unique_module_sources: unique_module_sources.len(),
            unique_provider_sources: unique_provider_sources.len(),
            findings_by_severity,
            findings_by_category,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::DeprecationRef;
    use crate::graph::GraphBuilder;
    use crate::types::{ModuleSource, RuntimeSource};
    use crate::{Constraint, VersionRange};
    use std::path::PathBuf;

    fn create_module(
        name: &str,
        source_name: &str,
        version: Option<&str>,
        repo: &str,
    ) -> ModuleRef {
        ModuleRef {
            name: name.to_string(),
            source: ModuleSource::Registry {
                hostname: "registry.terraform.io".to_string(),
                namespace: "terraform-aws-modules".to_string(),
                name: source_name.to_string(),
                provider: "aws".to_string(),
            },
            version_constraint: version.map(|v| Constraint::parse(v).unwrap()),
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: Some(repo.to_string()),
            attributes: Default::default(),
        }
    }

    fn create_provider(name: &str, version: Option<&str>, repo: &str) -> ProviderRef {
        ProviderRef {
            name: name.to_string(),
            source: Some(format!("hashicorp/{name}")),
            version_constraint: version.map(|v| Constraint::parse(v).unwrap()),
            file_path: PathBuf::from("versions.tf"),
            line_number: 1,
            repository: Some(repo.to_string()),
        }
    }

    #[test]
    fn test_detect_missing_constraint() {
        let modules = vec![create_module("vpc", "vpc", None, "repo-a")];
        let providers = vec![];

        let graph = GraphBuilder::new()
            .build(&modules, &providers, &[])
            .unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        let missing: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == FindingCategory::MissingConstraint)
            .collect();

        assert!(!missing.is_empty(), "Should detect missing constraint");
    }

    #[test]
    fn test_detect_broad_constraint() {
        let modules = vec![create_module("vpc", "vpc", Some(">= 0.0.0"), "repo-a")];
        let providers = vec![];

        let graph = GraphBuilder::new()
            .build(&modules, &providers, &[])
            .unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        let broad: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == FindingCategory::BroadConstraint)
            .collect();

        assert!(!broad.is_empty(), "Should detect broad constraint");
    }

    #[test]
    fn test_detect_runtime_deprecation() {
        let runtimes = vec![RuntimeRef {
            name: "terraform".to_string(),
            version: Constraint::parse("0.12.0").unwrap(),
            source: RuntimeSource::Terraform,
            file_path: PathBuf::from("main.tf"),
            line_number: 1,
            repository: Some("test".to_string()),
        }];
        let modules = vec![];
        let providers = vec![];

        let graph = GraphBuilder::new()
            .build(&modules, &providers, &runtimes)
            .unwrap();
        let config = Config::default();
        let mut config = config.clone();
        config.deprecations.runtime = HashMap::from([(
            "terraform".to_string(),
            vec![DeprecationRef {
                version: Some("<= 0.13.0".to_string()),
                git_ref: None,
                reason: "Legacy Terraform version, migrate to v0.13.1 or later".to_string(),
                severity: Severity::Error.to_string(),
                replacement: ">= 0.13.1".to_string(),
            }],
        )]);
        let analyzer = Analyzer::new(&config);

        let result = analyzer
            .analyze(&graph, &modules, &providers, &runtimes)
            .unwrap();

        let deprecations: Vec<_> = result.deprecations.runtimes.iter().collect();

        assert!(
            !deprecations.is_empty(),
            "Should detect runtime deprecation"
        );
        assert_eq!(deprecations.len(), 1);
        assert_eq!(deprecations[0].name, "terraform");
        if let VersionRange::Exact(version) = deprecations[0].version.ranges.first().unwrap() {
            assert_eq!(version.to_string(), "0.12.0");
        } else {
            panic!("Expected exact version");
        }
    }

    #[test]
    fn test_analysis_summary() {
        let modules = vec![
            create_module("vpc", "vpc", Some("~> 5.0"), "repo-a"),
            create_module("eks", "eks", Some("~> 19.0"), "repo-a"),
        ];
        let providers = vec![create_provider("aws", Some(">= 4.0"), "repo-a")];

        let graph = GraphBuilder::new()
            .build(&modules, &providers, &[])
            .unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        assert_eq!(result.summary.total_modules, 2);
        assert_eq!(result.summary.total_providers, 1);
        assert_eq!(result.summary.unique_module_sources, 2);
        assert_eq!(result.summary.unique_provider_sources, 1);
    }
}
