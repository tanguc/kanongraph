//! Constraint conflict detection.
//!
//! This module implements the core conflict detection algorithm for
//! version constraints across modules and providers.

use crate::analyzer::deprecation;
use crate::analyzer::patterns::{PatternChecker, RiskyPattern};
use crate::config::Config;
use crate::error::Result;
use crate::graph::DependencyGraph;
use crate::types::{
    AnalysisResult, AnalysisSummary, Constraint, Finding, FindingCategory, Location, ModuleRef, ProviderRef, RuntimeRef, Severity
};
use std::collections::HashMap;

/// Analyzer for detecting version constraint conflicts and issues.
///
/// # Algorithm Overview
///
/// The conflict detection algorithm works in several phases:
///
/// ## Phase 1: Grouping
///
/// Group all modules/providers by their canonical source identifier.
/// This creates clusters of items that should be compatible.
///
/// ```text
/// terraform-aws-modules/vpc/aws:
///   - repo-a/main.tf: ~> 5.0
///   - repo-b/main.tf: ~> 4.0
///   - repo-c/main.tf: >= 3.0, < 5.0
/// ```
///
/// ## Phase 2: Pairwise Comparison
///
/// For each group, compare all pairs of constraints to find conflicts.
///
/// ```text
/// Compare: ~> 5.0 vs ~> 4.0
///   - ~> 5.0 allows: 5.0.0 - 5.x.x
///   - ~> 4.0 allows: 4.0.0 - 4.x.x
///   - Overlap: NONE → CONFLICT!
/// ```
///
/// ## Phase 3: Severity Classification
///
/// Classify conflicts by severity based on:
/// - Whether constraints have any overlap
/// - How narrow the overlap is
/// - Whether the modules are in the same repository
///
/// # Example
///
/// ```rust,no_run
/// use driftops::analyzer::Analyzer;
/// use driftops::Config;
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

    /// Analyze modules and providers for conflicts and issues.
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
            "Starting analysis"
        );
        let mut findings = Vec::new();

        // Phase 1: Detect module constraint conflicts
        tracing::debug!("Phase 1: Detecting module constraint conflicts");
        let module_conflicts = self.detect_module_conflicts(modules);
        tracing::debug!(module_conflicts = module_conflicts.len(), "Module conflicts detected");
        findings.extend(module_conflicts);

        // Phase 2: Detect provider constraint conflicts
        tracing::debug!("Phase 2: Detecting provider constraint conflicts");
        let provider_conflicts = self.detect_provider_conflicts(providers);
        tracing::debug!(provider_conflicts = provider_conflicts.len(), "Provider conflicts detected");
        findings.extend(provider_conflicts);

        // Phase 3: Check for missing constraints
        tracing::debug!("Phase 3: Checking for missing constraints");
        let missing = self.check_missing_constraints(modules, providers);
        tracing::debug!(missing_constraints = missing.len(), "Missing constraints found");
        findings.extend(missing);

        // Phase 4: Check for risky patterns
        tracing::debug!("Phase 4: Checking for risky patterns");
        let risky = self.check_risky_patterns(modules, providers);
        tracing::debug!(risky_patterns = risky.len(), "Risky patterns found");
        findings.extend(risky);

        // Phase 5: Check for broad constraints
        tracing::debug!("Phase 5: Checking for broad constraints");
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
        let error_count = summary.findings_by_severity.get("ERROR").copied().unwrap_or(0)
            + summary.findings_by_severity.get("CRITICAL").copied().unwrap_or(0);
        let warning_count = summary.findings_by_severity.get("WARNING").copied().unwrap_or(0);
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

    /// Detect conflicts between module version constraints.
    fn detect_module_conflicts(&self, modules: &[ModuleRef]) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Group modules by source
        let grouped = self.group_modules_by_source(modules);
        tracing::debug!(module_groups = grouped.len(), "Grouped modules by source for conflict detection");

        // Check each group for conflicts
        for (source, module_refs) in grouped {
            if module_refs.len() < 2 {
                tracing::debug!(
                    source = %source,
                    count = module_refs.len(),
                    "Skipping source with less than 2 modules"
                );
                continue;
            }

            tracing::debug!(
                source = %source,
                count = module_refs.len(),
                "Checking module conflicts for source"
            );

            // Compare all pairs
            let mut pairs_checked = 0;
            for i in 0..module_refs.len() {
                for j in (i + 1)..module_refs.len() {
                    pairs_checked += 1;
                    let m1 = module_refs[i];
                    let m2 = module_refs[j];

                    if let Some(finding) = self.check_constraint_conflict(
                        &source,
                        m1.version_constraint.as_ref(),
                        m2.version_constraint.as_ref(),
                        &m1.file_path,
                        m1.line_number,
                        m1.repository.as_deref(),
                        &m2.file_path,
                        m2.line_number,
                        m2.repository.as_deref(),
                    ) {
                        tracing::debug!(
                            source = %source,
                            constraint1 = %m1.version_constraint.as_ref().map(|c| c.raw.as_str()).unwrap_or("none"),
                            constraint2 = %m2.version_constraint.as_ref().map(|c| c.raw.as_str()).unwrap_or("none"),
                            "Found module constraint conflict"
                        );
                        findings.push(finding);
                    }
                }
            }
            tracing::debug!(
                source = %source,
                pairs_checked = pairs_checked,
                conflicts_found = findings.len(),
                "Completed conflict checking for source"
            );
        }

        findings
    }

    /// Detect conflicts between provider version constraints.
    fn detect_provider_conflicts(&self, providers: &[ProviderRef]) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Group providers by source
        let grouped = self.group_providers_by_source(providers);
        tracing::debug!(provider_groups = grouped.len(), "Grouped providers by source for conflict detection");

        // Check each group for conflicts
        for (source, provider_refs) in grouped {
            if provider_refs.len() < 2 {
                tracing::debug!(
                    source = %source,
                    count = provider_refs.len(),
                    "Skipping source with less than 2 providers"
                );
                continue;
            }

            tracing::debug!(
                source = %source,
                count = provider_refs.len(),
                "Checking provider conflicts for source"
            );

            // Compare all pairs
            let mut pairs_checked = 0;
            for i in 0..provider_refs.len() {
                for j in (i + 1)..provider_refs.len() {
                    pairs_checked += 1;
                    let p1 = provider_refs[i];
                    let p2 = provider_refs[j];

                    if let Some(finding) = self.check_constraint_conflict(
                        &source,
                        p1.version_constraint.as_ref(),
                        p2.version_constraint.as_ref(),
                        &p1.file_path,
                        p1.line_number,
                        p1.repository.as_deref(),
                        &p2.file_path,
                        p2.line_number,
                        p2.repository.as_deref(),
                    ) {
                        tracing::debug!(
                            source = %source,
                            constraint1 = %p1.version_constraint.as_ref().map(|c| c.raw.as_str()).unwrap_or("none"),
                            constraint2 = %p2.version_constraint.as_ref().map(|c| c.raw.as_str()).unwrap_or("none"),
                            "Found provider constraint conflict"
                        );
                        findings.push(finding);
                    }
                }
            }
            tracing::debug!(
                source = %source,
                pairs_checked = pairs_checked,
                "Completed conflict checking for provider source"
            );
        }

        findings
    }

    /// Check if two constraints conflict.
    #[allow(clippy::too_many_arguments)]
    fn check_constraint_conflict(
        &self,
        source: &str,
        constraint1: Option<&Constraint>,
        constraint2: Option<&Constraint>,
        file1: &std::path::Path,
        line1: usize,
        repo1: Option<&str>,
        file2: &std::path::Path,
        line2: usize,
        repo2: Option<&str>,
    ) -> Option<Finding> {
        // If either has no constraint, no conflict (but might be a separate issue)
        let c1 = constraint1?;
        let c2 = constraint2?;

        tracing::debug!(
            source = %source,
            constraint1 = %c1.raw,
            constraint2 = %c2.raw,
            file1 = %file1.display(),
            file2 = %file2.display(),
            "Checking constraint conflict"
        );

        // Check for conflict
        if !c1.conflicts_with(c2) {
            tracing::debug!(
                source = %source,
                constraint1 = %c1.raw,
                constraint2 = %c2.raw,
                "Constraints do not conflict"
            );
            return None;
        }

        tracing::debug!(
            source = %source,
            constraint1 = %c1.raw,
            constraint2 = %c2.raw,
            repo1 = ?repo1,
            repo2 = ?repo2,
            "Constraints conflict detected"
        );

        // Determine severity based on context
        let severity = if repo1 == repo2 {
            // Same repo: more severe
            tracing::debug!(source = %source, "Same repository conflict, using Error severity");
            Severity::Error
        } else {
            // Different repos: might be intentional
            tracing::debug!(source = %source, "Different repository conflict, using Warning severity");
            Severity::Warning
        };

        let location1 = Location {
            file: file1.to_path_buf(),
            line: line1,
            column: None,
            repository: repo1.map(String::from),
        };

        let location2 = Location {
            file: file2.to_path_buf(),
            line: line2,
            column: None,
            repository: repo2.map(String::from),
        };

        Some(Finding {
            code: "DRIFT001".to_string(),
            severity,
            message: format!(
                "Version constraint conflict for '{source}': '{}' vs '{}'",
                c1.raw, c2.raw
            ),
            description: Some(format!(
                "The constraints '{}' and '{}' have no overlapping versions. \
                 This will cause Terraform to fail when both modules are used together.",
                c1.raw, c2.raw
            )),
            location: Some(location1),
            related_locations: vec![location2],
            suggestion: Some(format!(
                "Consider aligning the constraints. A compatible range might be: '{}'",
                suggest_compatible_constraint(c1, c2)
            )),
            category: FindingCategory::ConstraintConflict,
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
            if module.version_constraint.is_none() && !module.source.is_local() {
                findings.push(Finding {
                    code: "DRIFT002".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Module '{}' has no version constraint",
                        module.name
                    ),
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
                    suggestion: Some(format!(
                        "Add a version constraint, e.g., version = \"~> 1.0\""
                    )),
                    category: FindingCategory::MissingConstraint,
                });
            }
        }

        // Check providers
        for provider in providers {
            if provider.version_constraint.is_none() {
                findings.push(Finding {
                    code: "DRIFT002".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Provider '{}' has no version constraint",
                        provider.name
                    ),
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
                    suggestion: Some(format!(
                        "Add a version constraint, e.g., version = \">= 4.0, < 6.0\""
                    )),
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
            if let Some(constraint) = &module.version_constraint {
                if constraint.is_overly_broad() {
                    findings.push(Finding {
                        code: "DRIFT004".to_string(),
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
                        code: "DRIFT004".to_string(),
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
                            "Use a more specific constraint like '>= 4.0, < 6.0'"
                                .to_string(),
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
                "DRIFT003",
                Severity::Warning,
                format!("'{name}' uses wildcard version constraint"),
                "Wildcard constraints like '*' allow any version and should be avoided.",
                "Replace with a specific constraint like '~> 1.0'",
            ),
            RiskyPattern::PreRelease => (
                "DRIFT005",
                Severity::Info,
                format!("'{name}' uses pre-release version"),
                "Pre-release versions may be unstable and are not recommended for production.",
                "Consider using a stable release version",
            ),
            RiskyPattern::ExactVersion => (
                "DRIFT006",
                Severity::Info,
                format!("'{name}' uses exact version constraint"),
                "Exact version constraints prevent automatic patch updates.",
                "Consider using '~> X.Y.0' to allow patch updates",
            ),
            RiskyPattern::NoUpperBound => (
                "DRIFT007",
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

    /// Group modules by their canonical source.
    fn group_modules_by_source<'a>(
        &self,
        modules: &'a [ModuleRef],
    ) -> HashMap<String, Vec<&'a ModuleRef>> {
        let mut grouped: HashMap<String, Vec<&ModuleRef>> = HashMap::new();
        for module in modules {
            let key = module.source.canonical_id();
            grouped.entry(key).or_default().push(module);
        }
        grouped
    }

    /// Group providers by their canonical source.
    fn group_providers_by_source<'a>(
        &self,
        providers: &'a [ProviderRef],
    ) -> HashMap<String, Vec<&'a ProviderRef>> {
        let mut grouped: HashMap<String, Vec<&ProviderRef>> = HashMap::new();
        for provider in providers {
            let key = provider.qualified_source();
            grouped.entry(key).or_default().push(provider);
        }
        grouped
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

/// Suggest a compatible constraint given two conflicting constraints.
fn suggest_compatible_constraint(c1: &Constraint, c2: &Constraint) -> String {
    // This is a simplified suggestion; a full implementation would
    // compute the intersection of the constraint ranges
    
    // For now, suggest the more restrictive of the two
    if c1.raw.contains("~>") && c2.raw.contains("~>") {
        // Both pessimistic: suggest the higher one
        return format!("{} (align both to this)", c1.raw);
    }

    // Default suggestion
    "align both constraints to the same range".to_string()
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;
    use crate::VersionRange;
    use crate::config::DeprecationRef;
    use crate::graph::GraphBuilder;
    use crate::types::{ModuleSource, RuntimeSource};
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
    fn test_detect_module_conflict() {
        let modules = vec![
            create_module("vpc1", "vpc", Some(">= 5.0"), "repo-a"),
            create_module("vpc2", "vpc", Some("<= 4.5"), "repo-b"),
        ];
        let providers = vec![];

        let graph = GraphBuilder::new().build(&modules, &providers, &[]).unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        // Should find the conflict
        let conflicts: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == FindingCategory::ConstraintConflict)
            .collect();

        assert!(!conflicts.is_empty(), "Should detect constraint conflict");
    }

    #[test]
    fn test_detect_missing_constraint() {
        let modules = vec![create_module("vpc", "vpc", None, "repo-a")];
        let providers = vec![];

        let graph = GraphBuilder::new().build(&modules, &providers, &[]).unwrap();
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
    fn test_no_conflict_when_compatible() {
        let modules = vec![
            create_module("vpc1", "vpc", Some(">= 4.0"), "repo-a"),
            create_module("vpc2", "vpc", Some("<= 5.0"), "repo-b"),
        ];
        let providers = vec![];

        let graph = GraphBuilder::new().build(&modules, &providers, &[]).unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        let conflicts: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == FindingCategory::ConstraintConflict)
            .collect();

        assert!(conflicts.is_empty(), "Should not detect conflict for compatible constraints");
    }

    #[test]
    fn test_detect_broad_constraint() {
        let modules = vec![create_module("vpc", "vpc", Some(">= 0.0.0"), "repo-a")];
        let providers = vec![];

        let graph = GraphBuilder::new().build(&modules, &providers, &[]).unwrap();
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
    fn test_provider_conflict() {
        let modules = vec![];
        let providers = vec![
            create_provider("aws", Some(">= 5.0"), "repo-a"),
            create_provider("aws", Some("<= 4.0"), "repo-b"),
        ];

        let graph = GraphBuilder::new().build(&modules, &providers, &[]).unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        let conflicts: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == FindingCategory::ConstraintConflict)
            .collect();

        assert!(!conflicts.is_empty(), "Should detect provider conflict");
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

        let graph = GraphBuilder::new().build(&modules, &providers, &runtimes).unwrap();
        let config = Config::default();
        let mut config = config.clone();
        config.deprecations.runtime = HashMap::from([("terraform".to_string(), vec![DeprecationRef {
            version: Some("<= 0.13.0".to_string()),
            git_ref: None,
            reason: "Legacy Terraform version, migrate to v0.13.1 or later".to_string(),
            severity: Severity::Error.to_string(),
            replacement: ">= 0.13.1".to_string(),
        }])]);
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &runtimes).unwrap();

        let deprecations: Vec<_> = result
            .deprecations
            .runtimes
            .iter()
            .collect();

        assert!(!deprecations.is_empty(), "Should detect runtime deprecation");
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

        let graph = GraphBuilder::new().build(&modules, &providers, &[]).unwrap();
        let config = Config::default();
        let analyzer = Analyzer::new(&config);

        let result = analyzer.analyze(&graph, &modules, &providers, &[]).unwrap();

        assert_eq!(result.summary.total_modules, 2);
        assert_eq!(result.summary.total_providers, 1);
        assert_eq!(result.summary.unique_module_sources, 2);
        assert_eq!(result.summary.unique_provider_sources, 1);
    }


}

