//! Plain text report generator.

use crate::config::Config;
use crate::error::Result;
use crate::reporter::ReportGenerator;
use crate::types::{ScanResult, Severity};
use colored::Colorize;
use comfy_table::{Cell, Color, ContentArrangement, Table};

/// Text report generator for CLI output.
pub struct TextReporter {
    /// Whether to use colors
    use_colors: bool,
    /// Whether to show verbose output
    verbose: bool,

    /// strict mode
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
        output.push_str(&self.format_header());
        output.push('\n');

        // Summary
        output.push_str(&self.format_summary(result));
        output.push('\n');

        // Findings
        if !result.analysis.findings.is_empty() {
            output.push_str(&self.format_findings(result));
            output.push('\n');
        }

        // Module table (handles verbose logic internally)
        if !result.modules.is_empty() {
            let modules_output = self.format_modules(result);
            if !modules_output.is_empty() {
                output.push_str(&modules_output);
                output.push('\n');
            }
        }

        // Provider table (handles verbose logic internally)
        if !result.providers.is_empty() {
            let providers_output = self.format_providers(result);
            if !providers_output.is_empty() {
                output.push_str(&providers_output);
                output.push('\n');
            }
        }

        // Footer
        output.push_str(&self.format_footer(result));

        Ok(output)
    }
}

impl TextReporter {
    /// Format the report header.
    fn format_header(&self) -> String {
        let title = "MonPhare Analysis";
        let version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

        if self.use_colors {
            format!(
                "\n{} {} {}\n{}\n",
                title.bright_white().bold(),
                version.dimmed(),
                format!("({})", timestamp).dimmed(),
                "=".repeat(80).bright_blue(),
            )
        } else {
            format!(
                "\n{} {} ({})\n{}\n",
                title,
                version,
                timestamp,
                "=".repeat(80),
            )
        }
    }

    /// Format the summary section.
    fn format_summary(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        let section_title = if self.use_colors {
            "Summary".bright_cyan().bold().to_string()
        } else {
            "Summary".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));
        output.push_str(&"-".repeat(80));
        output.push('\n');

        // findings by severity
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

        // format the findings summary with colors
        if self.use_colors {
            output.push_str(&format!(
                "  {} {} | {} {} | {} {}\n",
                errors.to_string().red().bold(),
                if errors == 1 { "Error" } else { "Errors" },
                warnings.to_string().yellow().bold(),
                if warnings == 1 { "Warning" } else { "Warnings" },
                infos.to_string().blue(),
                if infos == 1 { "Info" } else { "Infos" }
            ));
        } else {
            output.push_str(&format!(
                "  {} {} | {} {} | {} {}\n",
                errors,
                if errors == 1 { "Error" } else { "Errors" },
                warnings,
                if warnings == 1 { "Warning" } else { "Warnings" },
                infos,
                if infos == 1 { "Info" } else { "Infos" }
            ));
        }

        output.push_str(&format!(
            "  {} files | {} modules | {} providers\n",
            result.files_scanned.len(),
            result.modules.len(),
            result.providers.len()
        ));

        output
    }

    /// Format the findings section.
    fn format_findings(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        let section_title = if self.use_colors {
            "Findings".bright_cyan().bold().to_string()
        } else {
            "Findings".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));
        output.push_str(&"-".repeat(80));
        output.push('\n');

        // sort findings by severity (critical first)
        let mut findings = result.analysis.findings.clone();
        findings.sort_by(|a, b| b.severity.cmp(&a.severity));

        for finding in &findings {
            output.push_str(&self.format_finding(finding));
            output.push('\n');
        }

        output
    }

    /// Format a single finding.
    fn format_finding(&self, finding: &crate::types::Finding) -> String {
        let severity_str = match finding.severity {
            Severity::Critical => {
                if self.use_colors {
                    "CRITICAL".red().bold().to_string()
                } else {
                    "CRITICAL".to_string()
                }
            }
            Severity::Error => {
                if self.use_colors {
                    "ERROR".red().to_string()
                } else {
                    "ERROR".to_string()
                }
            }
            Severity::Warning => {
                if self.use_colors {
                    "WARNING".yellow().to_string()
                } else {
                    "WARNING".to_string()
                }
            }
            Severity::Info => {
                if self.use_colors {
                    "INFO".blue().to_string()
                } else {
                    "INFO".to_string()
                }
            }
        };

        let mut output = format!(
            "\n  [{severity_str}] {} ({})\n",
            finding.message, finding.code
        );

        if let Some(loc) = &finding.location {
            let loc_str = if self.use_colors {
                format!("    → {}", loc).dimmed().to_string()
            } else {
                format!("    → {loc}")
            };
            output.push_str(&loc_str);
            output.push('\n');
        }

        if let Some(desc) = &finding.description {
            let desc_str = if self.use_colors {
                format!("    {desc}").dimmed().to_string()
            } else {
                format!("    {desc}")
            };
            output.push_str(&desc_str);
            output.push('\n');
        }

        if let Some(suggestion) = &finding.suggestion {
            let sugg_str = if self.use_colors {
                format!("    💡 {suggestion}").green().to_string()
            } else {
                format!("    Suggestion: {suggestion}")
            };
            output.push_str(&sugg_str);
            output.push('\n');
        }

        output
    }

    /// Format the modules table.
    fn format_modules(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        // filter modules with issues (check if they appear in findings)
        let modules_with_issues: Vec<_> = result.modules.iter()
            .filter(|m| {
                // check if any finding mentions this module by name (in quotes)
                let pattern = format!("'{}'", m.name);
                result.analysis.findings.iter().any(|f| f.message.contains(&pattern))
            })
            .collect();

        let total_modules = result.modules.len();
        let issues_count = modules_with_issues.len();
        let passing_count = total_modules - issues_count;

        // skip section if nothing to show
        if !self.verbose && modules_with_issues.is_empty() {
            return String::new();
        }

        let section_title = if self.use_colors {
            "Modules".bright_cyan().bold().to_string()
        } else {
            "Modules".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));
        output.push_str(&"-".repeat(80));
        output.push('\n');

        // show summary if not in verbose mode and there are passing modules
        if !self.verbose && passing_count > 0 {
            let summary = format!("{} issues, {} passing", issues_count, passing_count);
            if self.use_colors {
                output.push_str(&format!("  {} (use -vv to show all)\n\n", summary.dimmed()));
            } else {
                output.push_str(&format!("  {} (use -vv to show all)\n\n", summary));
            }
        }

        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_BORDERS_ONLY)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Source", "Version", "File"]);

        // iterate based on verbose mode
        if self.verbose {
            for module in &result.modules {
                self.add_module_row(&mut table, module);
            }
        } else {
            for module in modules_with_issues {
                self.add_module_row(&mut table, module);
            }
        }

        output.push_str(&table.to_string());
        output.push('\n');

        output
    }

    /// Add a module row to the table
    fn add_module_row(&self, table: &mut Table, module: &crate::types::ModuleRef) {
        let version = module
            .version_constraint
            .as_ref()
            .map(|c| c.raw.clone())
            .unwrap_or_else(|| "MISSING".to_string());

        // color the version based on status
        let version_cell = if module.version_constraint.is_none() {
            if self.use_colors {
                Cell::new(&version).fg(Color::Red)
            } else {
                Cell::new(&version)
            }
        } else if self.use_colors {
            Cell::new(&version).fg(Color::Green)
        } else {
            Cell::new(&version)
        };

        // format source with module name
        let source_display = format!("{} ({})",
            truncate(&module.source.canonical_id(), 45),
            &module.name
        );

        // show last 2 directories + filename for context
        let file_display = get_contextual_path(&module.file_path, 3);

        table.add_row(vec![
            Cell::new(&source_display),
            version_cell,
            Cell::new(&file_display),
        ]);
    }

    /// Format the providers table.
    fn format_providers(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        // filter providers with issues (check if they appear in findings)
        let providers_with_issues: Vec<_> = result.providers.iter()
            .filter(|p| {
                // check if any finding mentions this provider by name (in quotes)
                let pattern = format!("'{}'", p.name);
                result.analysis.findings.iter().any(|f| f.message.contains(&pattern))
            })
            .collect();

        let total_providers = result.providers.len();
        let issues_count = providers_with_issues.len();
        let passing_count = total_providers - issues_count;

        // skip section if nothing to show
        if !self.verbose && providers_with_issues.is_empty() {
            return String::new();
        }

        let section_title = if self.use_colors {
            "Providers".bright_cyan().bold().to_string()
        } else {
            "Providers".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));
        output.push_str(&"-".repeat(80));
        output.push('\n');

        // show summary if not in verbose mode and there are passing providers
        if !self.verbose && passing_count > 0 {
            let summary = format!("{} issues, {} passing", issues_count, passing_count);
            if self.use_colors {
                output.push_str(&format!("  {} (use -vv to show all)\n\n", summary.dimmed()));
            } else {
                output.push_str(&format!("  {} (use -vv to show all)\n\n", summary));
            }
        }

        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_BORDERS_ONLY)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Provider", "Version", "File"]);

        // iterate based on verbose mode
        if self.verbose {
            for provider in &result.providers {
                self.add_provider_row(&mut table, provider);
            }
        } else {
            for provider in providers_with_issues {
                self.add_provider_row(&mut table, provider);
            }
        }

        output.push_str(&table.to_string());
        output.push('\n');

        output
    }

    /// Add a provider row to the table
    fn add_provider_row(&self, table: &mut Table, provider: &crate::types::ProviderRef) {
        let version = provider
            .version_constraint
            .as_ref()
            .map(|c| c.raw.clone())
            .unwrap_or_else(|| "MISSING".to_string());

        // color the version based on status
        let version_cell = if provider.version_constraint.is_none() {
            if self.use_colors {
                Cell::new(&version).fg(Color::Red)
            } else {
                Cell::new(&version)
            }
        } else if self.use_colors {
            Cell::new(&version).fg(Color::Green)
        } else {
            Cell::new(&version)
        };

        // show last 2 directories + filename for context
        let file_display = get_contextual_path(&provider.file_path, 3);

        table.add_row(vec![
            Cell::new(&provider.qualified_source()),
            version_cell,
            Cell::new(&file_display),
        ]);
    }

    /// Format the report footer.
    fn format_footer(&self, result: &ScanResult) -> String {
        tracing::debug!("strict_mode: {}", self.strict_mode);
        let status = if result.analysis.has_errors() {
            if self.use_colors {
                "❌ FAILED - Errors found".red().bold().to_string()
            } else {
                "FAILED - Errors found".to_string()
            }
        } else if result.analysis.has_warnings() && self.strict_mode {
            if self.use_colors {
                "❌ FAILED - Warnings found".red().bold().to_string()
            } else {
                "FAILED - Warnings found".to_string()
            }
        } else if result.analysis.has_warnings() && !self.strict_mode {
            if self.use_colors {
                "⚠️  PASSED with warnings".yellow().to_string()
            } else {
                "PASSED with warnings".to_string()
            }
        } else {
            "PASSED - No issues found".to_string()
        };

        format!("\n{}\n\n", status)
    }
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Get last N directories + filename from a path for better context.
/// Example: /Users/foo/projects/terraform/env/prod/main.tf -> env/prod/main.tf
fn get_contextual_path(path: &std::path::Path, depth: usize) -> String {
    let components: Vec<_> = path.components().collect();
    let start_idx = if components.len() > depth {
        components.len() - depth
    } else {
        0
    };

    components[start_idx..]
        .iter()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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
            files_scanned: vec![PathBuf::from("main.tf")],
            graph: Default::default(),
            analysis: AnalysisResult::default(),
        }
    }

    #[test]
    fn test_text_report_generation() {
        let result = create_test_result();
        let config = Config::default();
        let reporter = TextReporter::new(&config);

        let text = reporter.generate(&result).unwrap();

        assert!(text.contains("MonPhare Analysis Report"));
        assert!(text.contains("Summary"));
        assert!(text.contains("Files scanned"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }
}

