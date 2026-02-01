//! Self-contained HTML report generator.
//!
//! Generates a complete HTML report with embedded CSS and JavaScript,
//! requiring no external dependencies.

use crate::config::Config;
use crate::error::Result;
use crate::reporter::ReportGenerator;
use crate::types::{ScanResult, Severity};

/// HTML report generator.
pub struct HtmlReporter {
    _config: Config,
}

impl HtmlReporter {
    /// Create a new HTML reporter.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            _config: config.clone(),
        }
    }
}

impl ReportGenerator for HtmlReporter {
    fn generate(&self, result: &ScanResult) -> Result<String> {
        let html = generate_html_report(result);
        Ok(html)
    }
}

/// Generate a complete self-contained HTML report.
fn generate_html_report(result: &ScanResult) -> String {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let version = env!("CARGO_PKG_VERSION");

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

    let status_class = if result.analysis.has_errors() {
        "status-error"
    } else if result.analysis.has_warnings() {
        "status-warning"
    } else {
        "status-success"
    };

    let status_text = if result.analysis.has_errors() {
        "Failed - Errors Found"
    } else if result.analysis.has_warnings() {
        "Passed with Warnings"
    } else {
        "Passed - No Issues"
    };

    // Generate findings HTML
    let findings_html = result
        .analysis
        .findings
        .iter()
        .map(|f| {
            let severity_class = match f.severity {
                Severity::Critical => "severity-critical",
                Severity::Error => "severity-error",
                Severity::Warning => "severity-warning",
                Severity::Info => "severity-info",
            };

            let location_html = f
                .location
                .as_ref()
                .map(|loc| {
                    format!(
                        r#"<div class="finding-location">{}</div>"#,
                        html_escape(&loc.to_string())
                    )
                })
                .unwrap_or_default();

            let description_html = f
                .description
                .as_ref()
                .map(|d| {
                    format!(
                        r#"<div class="finding-description">{}</div>"#,
                        html_escape(d)
                    )
                })
                .unwrap_or_default();

            let suggestion_html = f
                .suggestion
                .as_ref()
                .map(|s| {
                    format!(
                        r#"<div class="finding-suggestion">💡 {}</div>"#,
                        html_escape(s)
                    )
                })
                .unwrap_or_default();

            format!(
                r#"
                <div class="finding {severity_class}">
                    <div class="finding-header">
                        <span class="finding-severity">{}</span>
                        <span class="finding-code">{}</span>
                        <span class="finding-category">{}</span>
                    </div>
                    <div class="finding-message">{}</div>
                    {location_html}
                    {description_html}
                    {suggestion_html}
                </div>
                "#,
                html_escape(&f.severity.to_string()),
                html_escape(&f.code),
                html_escape(&f.category.to_string()),
                html_escape(&f.message),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Generate modules table rows
    let modules_rows = result
        .modules
        .iter()
        .map(|m| {
            let version = m
                .version_constraint
                .as_ref()
                .map(|c| html_escape(&c.raw))
                .unwrap_or_else(|| r#"<span class="no-version">none</span>"#.to_string());

            format!(
                r#"
                <tr>
                    <td>{}</td>
                    <td class="source">{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td class="file">{}</td>
                </tr>
                "#,
                html_escape(&m.name),
                html_escape(&m.source.canonical_id()),
                version,
                html_escape(m.repository.as_deref().unwrap_or("-")),
                html_escape(&m.file_path.to_string_lossy()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Generate providers table rows
    let providers_rows = result
        .providers
        .iter()
        .map(|p| {
            let version = p
                .version_constraint
                .as_ref()
                .map(|c| html_escape(&c.raw))
                .unwrap_or_else(|| r#"<span class="no-version">none</span>"#.to_string());

            format!(
                r#"
                <tr>
                    <td>{}</td>
                    <td class="source">{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td class="file">{}</td>
                </tr>
                "#,
                html_escape(&p.name),
                html_escape(&p.qualified_source()),
                version,
                html_escape(p.repository.as_deref().unwrap_or("-")),
                html_escape(&p.file_path.to_string_lossy()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MonPhare Analysis Report</title>
    <style>
        :root {{
            --color-bg: #0f172a;
            --color-surface: #1e293b;
            --color-surface-hover: #334155;
            --color-border: #334155;
            --color-text: #f1f5f9;
            --color-text-muted: #94a3b8;
            --color-primary: #38bdf8;
            --color-success: #4ade80;
            --color-warning: #fbbf24;
            --color-error: #f87171;
            --color-critical: #ef4444;
            --font-mono: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
            --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
        }}

        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: var(--font-sans);
            background: var(--color-bg);
            color: var(--color-text);
            line-height: 1.6;
            min-height: 100vh;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
        }}

        header {{
            text-align: center;
            margin-bottom: 3rem;
            padding: 2rem;
            background: linear-gradient(135deg, var(--color-surface) 0%, var(--color-bg) 100%);
            border-radius: 1rem;
            border: 1px solid var(--color-border);
        }}

        h1 {{
            font-size: 2.5rem;
            font-weight: 700;
            background: linear-gradient(135deg, var(--color-primary) 0%, #818cf8 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 0.5rem;
        }}

        .subtitle {{
            color: var(--color-text-muted);
            font-size: 0.9rem;
        }}

        .status {{
            display: inline-block;
            padding: 0.5rem 1.5rem;
            border-radius: 2rem;
            font-weight: 600;
            margin-top: 1rem;
            font-size: 0.9rem;
        }}

        .status-success {{
            background: rgba(74, 222, 128, 0.2);
            color: var(--color-success);
            border: 1px solid var(--color-success);
        }}

        .status-warning {{
            background: rgba(251, 191, 36, 0.2);
            color: var(--color-warning);
            border: 1px solid var(--color-warning);
        }}

        .status-error {{
            background: rgba(248, 113, 113, 0.2);
            color: var(--color-error);
            border: 1px solid var(--color-error);
        }}

        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }}

        .stat-card {{
            background: var(--color-surface);
            border: 1px solid var(--color-border);
            border-radius: 0.75rem;
            padding: 1.5rem;
            text-align: center;
            transition: transform 0.2s, box-shadow 0.2s;
        }}

        .stat-card:hover {{
            transform: translateY(-2px);
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
        }}

        .stat-value {{
            font-size: 2.5rem;
            font-weight: 700;
            font-family: var(--font-mono);
            color: var(--color-primary);
        }}

        .stat-label {{
            color: var(--color-text-muted);
            font-size: 0.85rem;
            margin-top: 0.25rem;
        }}

        .stat-card.errors .stat-value {{ color: var(--color-error); }}
        .stat-card.warnings .stat-value {{ color: var(--color-warning); }}
        .stat-card.info .stat-value {{ color: var(--color-primary); }}

        section {{
            background: var(--color-surface);
            border: 1px solid var(--color-border);
            border-radius: 0.75rem;
            margin-bottom: 2rem;
            overflow: hidden;
        }}

        section h2 {{
            padding: 1rem 1.5rem;
            background: var(--color-bg);
            border-bottom: 1px solid var(--color-border);
            font-size: 1.1rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .findings {{
            padding: 1rem;
        }}

        .finding {{
            background: var(--color-bg);
            border-radius: 0.5rem;
            padding: 1rem;
            margin-bottom: 0.75rem;
            border-left: 4px solid var(--color-border);
        }}

        .finding:last-child {{ margin-bottom: 0; }}

        .severity-critical {{ border-left-color: var(--color-critical); }}
        .severity-error {{ border-left-color: var(--color-error); }}
        .severity-warning {{ border-left-color: var(--color-warning); }}
        .severity-info {{ border-left-color: var(--color-primary); }}

        .finding-header {{
            display: flex;
            gap: 0.75rem;
            margin-bottom: 0.5rem;
            flex-wrap: wrap;
        }}

        .finding-severity {{
            font-size: 0.75rem;
            font-weight: 600;
            padding: 0.2rem 0.5rem;
            border-radius: 0.25rem;
            text-transform: uppercase;
        }}

        .severity-critical .finding-severity {{
            background: rgba(239, 68, 68, 0.2);
            color: var(--color-critical);
        }}

        .severity-error .finding-severity {{
            background: rgba(248, 113, 113, 0.2);
            color: var(--color-error);
        }}

        .severity-warning .finding-severity {{
            background: rgba(251, 191, 36, 0.2);
            color: var(--color-warning);
        }}

        .severity-info .finding-severity {{
            background: rgba(56, 189, 248, 0.2);
            color: var(--color-primary);
        }}

        .finding-code {{
            font-family: var(--font-mono);
            font-size: 0.75rem;
            color: var(--color-text-muted);
            background: var(--color-surface);
            padding: 0.2rem 0.5rem;
            border-radius: 0.25rem;
        }}

        .finding-category {{
            font-size: 0.75rem;
            color: var(--color-text-muted);
            padding: 0.2rem 0.5rem;
            background: var(--color-surface);
            border-radius: 0.25rem;
        }}

        .finding-message {{
            font-weight: 500;
            margin-bottom: 0.5rem;
        }}

        .finding-location {{
            font-family: var(--font-mono);
            font-size: 0.8rem;
            color: var(--color-text-muted);
            margin-bottom: 0.5rem;
        }}

        .finding-description {{
            font-size: 0.9rem;
            color: var(--color-text-muted);
            margin-bottom: 0.5rem;
        }}

        .finding-suggestion {{
            font-size: 0.85rem;
            color: var(--color-success);
            background: rgba(74, 222, 128, 0.1);
            padding: 0.5rem;
            border-radius: 0.25rem;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
        }}

        th, td {{
            padding: 0.75rem 1rem;
            text-align: left;
            border-bottom: 1px solid var(--color-border);
        }}

        th {{
            background: var(--color-bg);
            font-weight: 600;
            font-size: 0.85rem;
            color: var(--color-text-muted);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}

        tr:hover td {{
            background: var(--color-surface-hover);
        }}

        td.source, td.file {{
            font-family: var(--font-mono);
            font-size: 0.85rem;
        }}

        .no-version {{
            color: var(--color-warning);
            font-style: italic;
        }}

        .empty-state {{
            text-align: center;
            padding: 3rem;
            color: var(--color-text-muted);
        }}

        footer {{
            text-align: center;
            padding: 2rem;
            color: var(--color-text-muted);
            font-size: 0.85rem;
        }}

        footer a {{
            color: var(--color-primary);
            text-decoration: none;
        }}

        footer a:hover {{
            text-decoration: underline;
        }}

        @media (max-width: 768px) {{
            .container {{ padding: 1rem; }}
            h1 {{ font-size: 1.75rem; }}
            .stats {{ grid-template-columns: repeat(2, 1fr); }}
            table {{ font-size: 0.85rem; }}
            th, td {{ padding: 0.5rem; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🔍 MonPhare Analysis Report</h1>
            <p class="subtitle">Generated: {timestamp} | Version: {version}</p>
            <div class="status {status_class}">{status_text}</div>
        </header>

        <div class="stats">
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Files Scanned</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Modules</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Providers</div>
            </div>
            <div class="stat-card errors">
                <div class="stat-value">{errors}</div>
                <div class="stat-label">Errors</div>
            </div>
            <div class="stat-card warnings">
                <div class="stat-value">{warnings}</div>
                <div class="stat-label">Warnings</div>
            </div>
            <div class="stat-card info">
                <div class="stat-value">{infos}</div>
                <div class="stat-label">Info</div>
            </div>
        </div>

        <section>
            <h2>⚠️ Findings</h2>
            <div class="findings">
                {findings_section}
            </div>
        </section>

        <section>
            <h2>📦 Modules</h2>
            {modules_section}
        </section>

        <section>
            <h2>🔌 Providers</h2>
            {providers_section}
        </section>

        <footer>
            <p>Generated by <a href="https://github.com/yourusername/monphare">MonPhare</a> — Terraform/OpenTofu module constraint analyzer</p>
        </footer>
    </div>
</body>
</html>"##,
        result.files_scanned.len(),
        result.modules.len(),
        result.providers.len(),
        findings_section = if result.analysis.findings.is_empty() {
            r#"<div class="empty-state">✅ No issues found!</div>"#.to_string()
        } else {
            findings_html
        },
        modules_section = if result.modules.is_empty() {
            r#"<div class="empty-state">No modules found</div>"#.to_string()
        } else {
            format!(
                r#"<table>
                    <thead>
                        <tr>
                            <th>Name</th>
                            <th>Source</th>
                            <th>Version</th>
                            <th>Repository</th>
                            <th>File</th>
                        </tr>
                    </thead>
                    <tbody>
                        {modules_rows}
                    </tbody>
                </table>"#
            )
        },
        providers_section = if result.providers.is_empty() {
            r#"<div class="empty-state">No providers found</div>"#.to_string()
        } else {
            format!(
                r#"<table>
                    <thead>
                        <tr>
                            <th>Name</th>
                            <th>Source</th>
                            <th>Version</th>
                            <th>Repository</th>
                            <th>File</th>
                        </tr>
                    </thead>
                    <tbody>
                        {providers_rows}
                    </tbody>
                </table>"#
            )
        },
    )
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
    fn test_html_report_generation() {
        let result = create_test_result();
        let config = Config::default();
        let reporter = HtmlReporter::new(&config);

        let html = reporter.generate(&result).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("MonPhare Analysis Report"));
        assert!(html.contains("vpc"));
        assert!(html.contains("hashicorp/aws"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }
}

