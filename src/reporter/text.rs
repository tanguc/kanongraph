//! Plain text report generator.

use crate::config::Config;
use crate::error::Result;
use crate::reporter::ReportGenerator;
use crate::types::{ScanResult, Severity};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

/// Text report generator for CLI output.
pub struct TextReporter {
    /// Whether to use colors
    use_colors: bool,
    /// Whether to show verbose output
    verbose: bool,
}

impl TextReporter {
    /// Create a new text reporter.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            use_colors: config.output.colored,
            verbose: config.output.verbose,
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

        // Module table (if verbose)
        if self.verbose && !result.modules.is_empty() {
            output.push_str(&self.format_modules(result));
            output.push('\n');
        }

        // Provider table (if verbose)
        if self.verbose && !result.providers.is_empty() {
            output.push_str(&self.format_providers(result));
            output.push('\n');
        }

        // Footer
        output.push_str(&self.format_footer(result));

        Ok(output)
    }
}

impl TextReporter {
    /// Format the report header.
    fn format_header(&self) -> String {
        let title = "MonPhare Analysis Report";
        let version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

        if self.use_colors {
            format!(
                "\n{}\n{}\nGenerated: {}\n{}\n",
                "═".repeat(60).bright_blue(),
                format!("  {} {}", title, version).bright_white().bold(),
                timestamp.to_string().dimmed(),
                "═".repeat(60).bright_blue(),
            )
        } else {
            format!(
                "\n{}\n  {} {}\nGenerated: {}\n{}\n",
                "=".repeat(60),
                title,
                version,
                timestamp,
                "=".repeat(60),
            )
        }
    }

    /// Format the summary section.
    fn format_summary(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        let section_title = if self.use_colors {
            "📊 Summary".bright_cyan().bold().to_string()
        } else {
            "Summary".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));
        output.push_str(&"-".repeat(40));
        output.push('\n');

        output.push_str(&format!(
            "  Files scanned:    {}\n",
            result.files_scanned.len()
        ));
        output.push_str(&format!("  Modules found:    {}\n", result.modules.len()));
        output.push_str(&format!(
            "  Providers found:  {}\n",
            result.providers.len()
        ));
        output.push_str(&format!(
            "  Total findings:   {}\n",
            result.analysis.findings.len()
        ));

        // Findings by severity
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

        if self.use_colors {
            output.push_str(&format!(
                "    {} Errors, {} Warnings, {} Info\n",
                errors.to_string().red(),
                warnings.to_string().yellow(),
                infos.to_string().blue()
            ));
        } else {
            output.push_str(&format!(
                "    {errors} Errors, {warnings} Warnings, {infos} Info\n"
            ));
        }

        output
    }

    /// Format the findings section.
    fn format_findings(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        let section_title = if self.use_colors {
            "🔍 Findings".bright_cyan().bold().to_string()
        } else {
            "Findings".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));
        output.push_str(&"-".repeat(40));
        output.push('\n');

        // Sort findings by severity (critical first)
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

        let section_title = if self.use_colors {
            "📦 Modules".bright_cyan().bold().to_string()
        } else {
            "Modules".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Name", "Source", "Version", "Repository", "File"]);

        for module in &result.modules {
            let version = module
                .version_constraint
                .as_ref()
                .map(|c| c.raw.clone())
                .unwrap_or_else(|| {
                    if self.use_colors {
                        "(none)".dimmed().to_string()
                    } else {
                        "(none)".to_string()
                    }
                });

            let version_cell = if module.version_constraint.is_none() && self.use_colors {
                Cell::new(&version).fg(Color::Yellow)
            } else {
                Cell::new(&version)
            };

            table.add_row(vec![
                Cell::new(&module.name),
                Cell::new(truncate(&module.source.canonical_id(), 40)),
                version_cell,
                Cell::new(module.repository.as_deref().unwrap_or("-")),
                Cell::new(truncate(&module.file_path.to_string_lossy(), 30)),
            ]);
        }

        output.push_str(&table.to_string());
        output.push('\n');

        output
    }

    /// Format the providers table.
    fn format_providers(&self, result: &ScanResult) -> String {
        let mut output = String::new();

        let section_title = if self.use_colors {
            "🔌 Providers".bright_cyan().bold().to_string()
        } else {
            "Providers".to_string()
        };

        output.push_str(&format!("\n{section_title}\n"));

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Name", "Source", "Version", "Repository", "File"]);

        for provider in &result.providers {
            let version = provider
                .version_constraint
                .as_ref()
                .map(|c| c.raw.clone())
                .unwrap_or_else(|| "(none)".to_string());

            let version_cell = if provider.version_constraint.is_none() && self.use_colors {
                Cell::new(&version).fg(Color::Yellow)
            } else {
                Cell::new(&version)
            };

            table.add_row(vec![
                Cell::new(&provider.name),
                Cell::new(&provider.qualified_source()),
                version_cell,
                Cell::new(provider.repository.as_deref().unwrap_or("-")),
                Cell::new(truncate(&provider.file_path.to_string_lossy(), 30)),
            ]);
        }

        output.push_str(&table.to_string());
        output.push('\n');

        output
    }

    /// Format the report footer.
    fn format_footer(&self, result: &ScanResult) -> String {
        let status = if result.analysis.has_errors() {
            if self.use_colors {
                "❌ FAILED - Errors found".red().bold().to_string()
            } else {
                "FAILED - Errors found".to_string()
            }
        } else if result.analysis.has_warnings() {
            if self.use_colors {
                "⚠️  PASSED with warnings".yellow().to_string()
            } else {
                "PASSED with warnings".to_string()
            }
        } else {
            if self.use_colors {
                "✅ PASSED - No issues found".green().bold().to_string()
            } else {
                "PASSED - No issues found".to_string()
            }
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

