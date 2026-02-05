//! Plain text report generator with clean, table-based CLI output.
//! One unified table per repository showing all issues.

use crate::config::Config;
use crate::error::Result;
use crate::reporter::ReportGenerator;
use crate::types::{Finding, ScanResult, ScanWarning, Severity};
use colored::Colorize;
use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Table};
use std::collections::HashMap;

/// Text report generator for CLI output.
pub struct TextReporter {
    use_colors: bool,
    #[allow(dead_code)]
    verbose: bool,
    strict_mode: bool,
}

impl TextReporter {
    /// Create a new text reporter.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            use_colors: config.output.colored,
            verbose: config.output.verbose,
            strict_mode: config.scan.strict_mode,
        }
    }
}

impl ReportGenerator for TextReporter {
    fn generate(&self, result: &ScanResult) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str(&self.format_header(result));

        // Scan warnings (unparseable files, etc.)
        if !result.warnings.is_empty() {
            output.push_str(&self.format_scan_warnings(result));
        }

        // Issues grouped by repository
        if !result.analysis.findings.is_empty() {
            output.push_str(&self.format_issues_by_repo(result));
        } else {
            output.push_str(&self.format_no_issues());
        }

        // Footer
        output.push_str(&self.format_footer(result));

        Ok(output)
    }
}

impl TextReporter {
    /// Format the header with status and summary.
    fn format_header(&self, result: &ScanResult) -> String {
        let errors = count_by_severity(result, |s| {
            matches!(s, Severity::Error | Severity::Critical)
        });
        let warnings = count_by_severity(result, |s| matches!(s, Severity::Warning));

        let status = if errors > 0 || (warnings > 0 && self.strict_mode) {
            if self.use_colors {
                "FAILED".red().bold().to_string()
            } else {
                "FAILED".to_string()
            }
        } else if warnings > 0 {
            if self.use_colors {
                "PASSED".yellow().to_string()
            } else {
                "PASSED".to_string()
            }
        } else if self.use_colors {
            "PASSED".green().bold().to_string()
        } else {
            "PASSED".to_string()
        };

        let version = env!("CARGO_PKG_VERSION");

        let errors_str = if self.use_colors && errors > 0 {
            format!("{errors} errors").red().bold().to_string()
        } else {
            format!("{errors} errors")
        };
        let warnings_str = if self.use_colors && warnings > 0 {
            format!("{warnings} warnings").yellow().bold().to_string()
        } else {
            format!("{warnings} warnings")
        };

        format!(
            "\nMonPhare v{version}  [{status}]  {errors_str}, {warnings_str}\n\
             Scanned: {} files, {} modules, {} providers\n\n",
            result.files_scanned.len(),
            result.modules.len(),
            result.providers.len()
        )
    }

    /// Format issues grouped by repository, one table per repo.
    fn format_issues_by_repo(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        // Group findings by repository
        let mut by_repo: HashMap<String, Vec<&Finding>> = HashMap::new();
        for finding in &result.analysis.findings {
            let repo = finding
                .location
                .as_ref()
                .and_then(|l| l.repository.clone())
                .unwrap_or_else(|| "unknown".to_string());
            by_repo.entry(repo).or_default().push(finding);
        }

        // Sort repos: most errors first
        let mut repos: Vec<_> = by_repo.keys().cloned().collect();
        repos.sort_by(|a, b| {
            let err_a = by_repo[a]
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
                .count();
            let err_b = by_repo[b]
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
                .count();
            err_b.cmp(&err_a).then(a.cmp(b))
        });

        for repo in repos {
            let findings = &by_repo[&repo];

            // Count by severity for this repo
            let repo_errors = findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
                .count();
            let repo_warnings = findings
                .iter()
                .filter(|f| f.severity == Severity::Warning)
                .count();

            // Repo header
            let repo_title = if self.use_colors {
                format!("─ {repo} ").cyan().bold().to_string()
            } else {
                format!("─ {repo} ")
            };

            let counts = format_counts(repo_errors, repo_warnings, self.use_colors);

            output.push_str(&format!("┌{repo_title}{counts}\n"));

            // Build table
            let mut table = Table::new();
            table
                .load_preset(presets::UTF8_FULL_CONDENSED)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("Sev").add_attribute(Attribute::Bold),
                    Cell::new("Resource").add_attribute(Attribute::Bold),
                    Cell::new("Issue").add_attribute(Attribute::Bold),
                    Cell::new("Current").add_attribute(Attribute::Bold),
                    Cell::new("File").add_attribute(Attribute::Bold),
                ]);

            // Sort findings: errors first, then warnings, then info
            let mut sorted_findings = findings.clone();
            sorted_findings.sort_by(|a, b| b.severity.cmp(&a.severity));

            for finding in sorted_findings {
                let (sev_text, sev_color) = match finding.severity {
                    Severity::Critical => ("CRIT", Color::Red),
                    Severity::Error => ("ERR", Color::Red),
                    Severity::Warning => ("WARN", Color::Yellow),
                    Severity::Info => ("INFO", Color::Cyan),
                };

                let sev_cell = if self.use_colors {
                    Cell::new(sev_text)
                        .fg(sev_color)
                        .add_attribute(Attribute::Bold)
                } else {
                    Cell::new(sev_text)
                };

                // Extract resource name from message (e.g., "Module 'vpc' has no...")
                let resource = extract_resource_name(&finding.message, &finding.category);

                // Short issue description
                let issue = short_issue_description(&finding.code, &finding.category);

                // Current value (version constraint if available)
                let current = extract_current_value(&finding.message);

                // File path (relative to repo - strip any temp/clone prefix)
                let file = finding
                    .location
                    .as_ref()
                    .map(|l| {
                        let path_str = l.file.to_string_lossy();
                        let relative_path =
                            extract_relative_path(&path_str, l.repository.as_deref());
                        format!("{}:{}", relative_path, l.line)
                    })
                    .unwrap_or_else(|| "-".to_string());

                table.add_row(vec![
                    sev_cell,
                    Cell::new(&resource),
                    Cell::new(&issue),
                    Cell::new(&current),
                    Cell::new(&file),
                ]);
            }

            output.push_str(&table.to_string());
            output.push_str("\n\n");
        }

        output
    }

    /// Format message when there are no issues.
    fn format_no_issues(&self) -> String {
        if self.use_colors {
            "No issues found.\n\n".green().to_string()
        } else {
            "No issues found.\n\n".to_string()
        }
    }

    /// Format scan warnings (e.g., unparseable files).
    fn format_scan_warnings(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        let header = if self.use_colors {
            format!(
                "  {} {}\n\n",
                "SCAN WARNINGS".yellow().bold(),
                format!("({} skipped)", result.warnings.len()).dimmed()
            )
        } else {
            format!("  SCAN WARNINGS ({} skipped)\n\n", result.warnings.len())
        };
        output.push_str(&header);

        // Group warnings by repository
        let mut by_repo: HashMap<String, Vec<&ScanWarning>> = HashMap::new();
        for warning in &result.warnings {
            let repo = warning
                .repository
                .clone()
                .unwrap_or_else(|| "local".to_string());
            by_repo.entry(repo).or_default().push(warning);
        }

        for (repo, warnings) in by_repo {
            let repo_header = if self.use_colors {
                format!("  {} ({})\n", repo.yellow(), warnings.len())
            } else {
                format!("  {} ({})\n", repo, warnings.len())
            };
            output.push_str(&repo_header);

            for warning in warnings {
                let file = extract_relative_path(
                    &warning.file.display().to_string(),
                    warning.repository.as_deref(),
                );
                let line_info = warning.line.map_or(String::new(), |l| format!(":{l}"));

                let msg = if self.use_colors {
                    format!(
                        "    {} {}{}: {}\n",
                        "SKIP".yellow(),
                        file.dimmed(),
                        line_info.dimmed(),
                        warning.message
                    )
                } else {
                    format!("    SKIP {}{}: {}\n", file, line_info, warning.message)
                };
                output.push_str(&msg);
            }
            output.push('\n');
        }

        output
    }

    /// Format the footer.
    fn format_footer(&self, result: &ScanResult) -> String {
        let errors = count_by_severity(result, |s| {
            matches!(s, Severity::Error | Severity::Critical)
        });
        let warnings = count_by_severity(result, |s| matches!(s, Severity::Warning));

        if errors > 0 {
            if self.use_colors {
                "Fix errors to pass.\n\n".red().to_string()
            } else {
                "Fix errors to pass.\n\n".to_string()
            }
        } else if warnings > 0 && self.strict_mode {
            if self.use_colors {
                "Strict mode: fix warnings to pass.\n\n"
                    .yellow()
                    .to_string()
            } else {
                "Strict mode: fix warnings to pass.\n\n".to_string()
            }
        } else if warnings > 0 {
            if self.use_colors {
                "Passed with warnings.\n\n".yellow().to_string()
            } else {
                "Passed with warnings.\n\n".to_string()
            }
        } else {
            String::new()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

/// Count findings by severity.
fn count_by_severity<F>(result: &ScanResult, predicate: F) -> usize
where
    F: Fn(&Severity) -> bool,
{
    result
        .analysis
        .findings
        .iter()
        .filter(|f| predicate(&f.severity))
        .count()
}

/// Format error/warning counts.
fn format_counts(errors: usize, warnings: usize, use_colors: bool) -> String {
    let mut parts = Vec::new();

    if errors > 0 {
        let s = format!("{} error{}", errors, if errors == 1 { "" } else { "s" });
        if use_colors {
            parts.push(s.red().to_string());
        } else {
            parts.push(s);
        }
    }

    if warnings > 0 {
        let s = format!(
            "{} warning{}",
            warnings,
            if warnings == 1 { "" } else { "s" }
        );
        if use_colors {
            parts.push(s.yellow().to_string());
        } else {
            parts.push(s);
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("({})", parts.join(", "))
    }
}

/// Extract resource name from finding message.
/// E.g., "Module 'vpc' has no version" -> "module.vpc"
fn extract_resource_name(message: &str, _category: &crate::types::FindingCategory) -> String {
    // Try to extract quoted name
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            let name = &message[start + 1..start + 1 + end];

            // Determine prefix based on message content (case-insensitive search)
            let msg_lower = message.to_lowercase();
            let prefix = if msg_lower.contains("module") {
                "module"
            } else if msg_lower.contains("provider") {
                "provider"
            } else {
                "resource"
            };

            return format!("{prefix}.{name}");
        }
    }

    "unknown".to_string()
}

/// Extract relative path inside the repository.
/// Strips temp clone paths like "/tmp/xyz/repo-name/..." to just the path after repo name.
fn extract_relative_path(full_path: &str, repo_name: Option<&str>) -> String {
    // If we have a repo name, try to find it in the path and take everything after
    if let Some(repo) = repo_name {
        // Look for the repo name in the path
        if let Some(idx) = full_path.find(repo) {
            let after_repo = &full_path[idx + repo.len()..];
            // Strip leading slash
            let relative = after_repo.trim_start_matches('/');
            if !relative.is_empty() {
                return relative.to_string();
            }
        }
    }

    // Fallback: try common patterns for temp directories
    // Pattern: /tmp/.../repo-name/actual/path.tf or /var/folders/.../repo-name/...
    let parts: Vec<&str> = full_path.split('/').collect();

    // Look for common temp dir indicators and skip past them
    for (i, part) in parts.iter().enumerate() {
        // Skip if this looks like a temp/clone directory marker
        if *part == "tmp" || *part == "var" || part.starts_with("T") || part.contains("monphare") {
            continue;
        }
        // If we find something that looks like a repo name (contains hyphen or starts with letter)
        // and there's more path after it, return from here
        if i > 0
            && parts.len() > i + 1
            && (part.contains('-')
                || part
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false))
        {
            // Check if next part looks like a real path (not another temp marker)
            let remaining: Vec<&str> = parts[i + 1..].to_vec();
            if !remaining.is_empty() && !remaining[0].is_empty() {
                return remaining.join("/");
            }
        }
    }

    // Last resort: just return the last 2-3 components
    if parts.len() > 3 {
        parts[parts.len() - 3..].join("/")
    } else {
        full_path.to_string()
    }
}

/// Get a short issue description from code/category.
fn short_issue_description(code: &str, category: &crate::types::FindingCategory) -> String {
    match code {
        "missing-version" => "No version".to_string(),
        "wildcard-constraint" => "Wildcard (*)".to_string(),
        "broad-constraint" => "Too broad".to_string(),
        "prerelease-version" => "Pre-release".to_string(),
        "exact-version" => "Exact version".to_string(),
        "no-upper-bound" => "No upper bound".to_string(),
        _ => category.to_string(),
    }
}

/// Extract current value from message (if any).
/// E.g., "...has overly broad constraint: >= 0.0.0" -> ">= 0.0.0"
fn extract_current_value(message: &str) -> String {
    // Look for patterns like ": >= x.x.x" or "constraint: ~> x.x"
    if let Some(idx) = message.rfind(": ") {
        let value = message[idx + 2..].trim();
        if !value.is_empty() && (value.contains('.') || value.starts_with('*')) {
            return shorten_str(value, 15);
        }
    }

    // Look for patterns in parentheses
    if let Some(start) = message.rfind('(') {
        if let Some(end) = message.rfind(')') {
            if end > start {
                let value = &message[start + 1..end];
                if value.contains('.') {
                    return shorten_str(value, 15);
                }
            }
        }
    }

    "-".to_string()
}

/// Shorten a string to max length.
fn shorten_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Shorten a file path intelligently.
#[allow(dead_code)]
fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        return shorten_str(path, max_len);
    }

    // Keep last 2-3 parts
    let keep = parts.len().min(3);
    let short = parts[parts.len() - keep..].join("/");

    if short.len() + 4 <= max_len {
        format!(".../{short}")
    } else {
        shorten_str(&short, max_len)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

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
            files_scanned: vec![PathBuf::from("main.tf")],
            graph: Default::default(),
            analysis: AnalysisResult::default(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn test_text_report_generation() {
        let result = create_test_result();
        let config = Config::default();
        let reporter = TextReporter::new(&config);

        let text = reporter.generate(&result).unwrap();

        assert!(text.contains("MonPhare"));
        assert!(text.contains("PASSED"));
        assert!(text.contains("No issues found"));
    }

    #[test]
    fn test_extract_resource_name() {
        let category = crate::types::FindingCategory::MissingConstraint;
        assert_eq!(
            extract_resource_name("Module 'vpc' has no version constraint", &category),
            "module.vpc"
        );
        assert_eq!(
            extract_resource_name("Provider 'aws' has no version constraint", &category),
            "provider.aws"
        );
    }

    #[test]
    fn test_short_issue_description() {
        let category = crate::types::FindingCategory::MissingConstraint;
        assert_eq!(
            short_issue_description("missing-version", &category),
            "No version"
        );
        assert_eq!(
            short_issue_description("no-upper-bound", &category),
            "No upper bound"
        );
    }

    #[test]
    fn test_extract_current_value() {
        assert_eq!(
            extract_current_value("Module has overly broad constraint: >= 0.0.0"),
            ">= 0.0.0"
        );
        assert_eq!(
            extract_current_value("Module has no version constraint"),
            "-"
        );
    }

    #[test]
    fn test_shorten_path() {
        assert_eq!(shorten_path("short.tf", 20), "short.tf");
        let shortened = shorten_path("/very/long/path/to/some/terraform/file.tf", 30);
        assert!(shortened.contains("file.tf"));
    }

    #[test]
    fn test_extract_relative_path() {
        // With repo name
        assert_eq!(
            extract_relative_path("/tmp/abc123/my-repo/modules/vpc/main.tf", Some("my-repo")),
            "modules/vpc/main.tf"
        );

        // Repo name at different position
        assert_eq!(
            extract_relative_path(
                "/var/folders/xyz/my-repo/terraform/main.tf",
                Some("my-repo")
            ),
            "terraform/main.tf"
        );
    }
}
