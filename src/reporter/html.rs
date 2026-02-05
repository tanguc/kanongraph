//! Self-contained HTML report generator.
//!
//! Generates a beautiful, modern HTML report with embedded CSS and JavaScript.
//! Designed to be visually stunning, easy to navigate, and crystal clear about
//! where issues come from and how to fix them.

use crate::config::Config;
use crate::error::Result;
use crate::reporter::ReportGenerator;
use crate::types::{Finding, ModuleRef, ProviderRef, ScanResult, ScanWarning, Severity};
use std::collections::HashMap;

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

    // Count findings by severity
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

    // Determine status
    let (status_class, status_icon, status_text) = if result.analysis.has_errors() {
        ("status-error", "✖", "Errors Found")
    } else if result.analysis.has_warnings() {
        ("status-warning", "⚠", "Passed with Warnings")
    } else {
        ("status-success", "✔", "All Checks Passed")
    };

    // Count repositories
    let repos: std::collections::HashSet<_> = result
        .modules
        .iter()
        .filter_map(|m| m.repository.as_ref())
        .chain(result.providers.iter().filter_map(|p| p.repository.as_ref()))
        .collect();

    // Generate scan warnings HTML (if any)
    let warnings_html = generate_scan_warnings_html(result);
    let has_scan_warnings = !result.warnings.is_empty();

    // Generate findings HTML grouped by repository
    let findings_html = generate_findings_html(result);

    // Generate modules HTML
    let modules_html = generate_modules_html(result);

    // Generate providers HTML
    let providers_html = generate_providers_html(result);

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MonPhare Report - {status_text}</title>
    <style>
{css}
    </style>
</head>
<body>
    <div class="app">
        <!-- Sidebar -->
        <aside class="sidebar">
            <div class="logo">
                <span class="logo-icon">◈</span>
                <span class="logo-text">MonPhare</span>
            </div>
            <nav class="nav">
                <a href="#overview" class="nav-item active">
                    <span class="nav-icon">◉</span> Overview
                </a>
                {scan_warnings_nav}
                <a href="#findings" class="nav-item">
                    <span class="nav-icon">⚑</span> Findings
                    {findings_badge}
                </a>
                <a href="#modules" class="nav-item">
                    <span class="nav-icon">◫</span> Modules
                    <span class="badge">{modules_count}</span>
                </a>
                <a href="#providers" class="nav-item">
                    <span class="nav-icon">◩</span> Providers
                    <span class="badge">{providers_count}</span>
                </a>
            </nav>
            <div class="sidebar-footer">
                <div class="version">v{version}</div>
                <div class="timestamp">{timestamp}</div>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="main">
            <!-- Overview Section -->
            <section id="overview" class="section">
                <div class="status-banner {status_class}">
                    <span class="status-icon">{status_icon}</span>
                    <div class="status-content">
                        <h1 class="status-title">{status_text}</h1>
                        <p class="status-subtitle">Terraform/OpenTofu Module Constraint Analysis</p>
                    </div>
                </div>

                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-icon">📁</div>
                        <div class="stat-value">{files_count}</div>
                        <div class="stat-label">Files Scanned</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-icon">📦</div>
                        <div class="stat-value">{modules_count}</div>
                        <div class="stat-label">Modules</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-icon">🔌</div>
                        <div class="stat-value">{providers_count}</div>
                        <div class="stat-label">Providers</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-icon">📂</div>
                        <div class="stat-value">{repos_count}</div>
                        <div class="stat-label">Repositories</div>
                    </div>
                </div>

                <div class="findings-summary">
                    <div class="finding-stat error">
                        <span class="finding-count">{errors}</span>
                        <span class="finding-label">Errors</span>
                    </div>
                    <div class="finding-stat warning">
                        <span class="finding-count">{warnings}</span>
                        <span class="finding-label">Warnings</span>
                    </div>
                    <div class="finding-stat info">
                        <span class="finding-count">{infos}</span>
                        <span class="finding-label">Info</span>
                    </div>
                </div>
            </section>

            {scan_warnings_section}

            <!-- Findings Section -->
            <section id="findings" class="section">
                <h2 class="section-title">
                    <span class="section-icon">⚑</span>
                    Findings
                </h2>
                {findings_html}
            </section>

            <!-- Modules Section -->
            <section id="modules" class="section">
                <h2 class="section-title">
                    <span class="section-icon">◫</span>
                    Module Inventory
                </h2>
                {modules_html}
            </section>

            <!-- Providers Section -->
            <section id="providers" class="section">
                <h2 class="section-title">
                    <span class="section-icon">◩</span>
                    Provider Inventory
                </h2>
                {providers_html}
            </section>
        </main>
    </div>

    <script>
{js}
    </script>
</body>
</html>"##,
        css = get_css(),
        js = get_js(),
        files_count = result.files_scanned.len(),
        modules_count = result.modules.len(),
        providers_count = result.providers.len(),
        repos_count = repos.len(),
        findings_badge = if errors > 0 {
            format!(r#"<span class="badge badge-error">{errors}</span>"#)
        } else if warnings > 0 {
            format!(r#"<span class="badge badge-warning">{warnings}</span>"#)
        } else {
            String::new()
        },
        scan_warnings_nav = if has_scan_warnings {
            format!(
                r##"<a href="#scan-warnings" class="nav-item">
                    <span class="nav-icon">⚠</span> Skipped
                    <span class="badge badge-warning">{}</span>
                </a>"##,
                result.warnings.len()
            )
        } else {
            String::new()
        },
        scan_warnings_section = if has_scan_warnings {
            format!(
                r##"<!-- Scan Warnings Section -->
            <section id="scan-warnings" class="section">
                <h2 class="section-title">
                    <span class="section-icon">⚠</span>
                    Skipped Items
                </h2>
                {}
            </section>"##,
                warnings_html
            )
        } else {
            String::new()
        },
    )
}

/// Generate scan warnings HTML (for unparseable files, etc.)
fn generate_scan_warnings_html(result: &ScanResult) -> String {
    if result.warnings.is_empty() {
        return String::new();
    }

    // Group warnings by repository
    let mut by_repo: HashMap<String, Vec<&ScanWarning>> = HashMap::new();
    for warning in &result.warnings {
        let repo = warning.repository.clone().unwrap_or_else(|| "local".to_string());
        by_repo.entry(repo).or_default().push(warning);
    }

    let mut html = String::new();
    html.push_str(r#"<div class="warnings-note">
        <strong>Note:</strong> The following items had unparseable version constraints and were skipped during analysis.
        They may still have issues that couldn't be detected.
    </div>"#);

    for (repo, warnings) in by_repo {
        html.push_str(&format!(
            r#"<div class="repo-group">
                <div class="repo-header" onclick="toggleRepo(this)">
                    <span class="repo-icon">📁</span>
                    <span class="repo-name">{repo}</span>
                    <span class="badge badge-warning">{count} skipped</span>
                    <span class="expand-icon">▼</span>
                </div>
                <div class="repo-content">"#,
            repo = html_escape(&repo),
            count = warnings.len()
        ));

        for warning in warnings {
            let file = warning.file.display().to_string();
            // Extract relative path from file
            let relative_file = extract_relative_path(&file, warning.repository.as_deref());
            let line_info = warning.line.map_or(String::new(), |l| format!(":{}", l));

            html.push_str(&format!(
                r#"<div class="finding warning">
                    <div class="finding-header">
                        <span class="severity-badge warning">SKIPPED</span>
                        <span class="finding-code">{code}</span>
                    </div>
                    <div class="finding-message">{message}</div>
                    <div class="finding-location">
                        <span class="location-icon">📄</span>
                        <span class="location-file">{file}{line}</span>
                    </div>
                </div>"#,
                code = html_escape(&warning.code),
                message = html_escape(&warning.message),
                file = html_escape(&relative_file),
                line = line_info
            ));
        }

        html.push_str("</div></div>");
    }

    html
}

/// Extract relative path from a full file path.
fn extract_relative_path(full_path: &str, repo_name: Option<&str>) -> String {
    // Try to find the repo name in the path and return everything after it
    if let Some(repo) = repo_name {
        if let Some(idx) = full_path.find(repo) {
            let after_repo = &full_path[idx + repo.len()..];
            return after_repo.trim_start_matches('/').to_string();
        }
    }
    
    // Fallback: just return the file name or short path
    full_path.split('/').last().unwrap_or(full_path).to_string()
}

/// Generate findings HTML grouped by repository.
fn generate_findings_html(result: &ScanResult) -> String {
    if result.analysis.findings.is_empty() {
        return r#"<div class="empty-state">
            <div class="empty-icon">✔</div>
            <h3>No Issues Found</h3>
            <p>All modules and providers passed the checks.</p>
        </div>"#
            .to_string();
    }

    // Group findings by repository
    let mut by_repo: HashMap<String, Vec<&Finding>> = HashMap::new();
    for finding in &result.analysis.findings {
        let repo = finding
            .location
            .as_ref()
            .and_then(|l| l.repository.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        by_repo.entry(repo).or_default().push(finding);
    }

    let mut html = String::new();

    // Sort repos by error count
    let mut repos: Vec<_> = by_repo.keys().cloned().collect();
    repos.sort_by(|a, b| {
        let errors_a = by_repo[a]
            .iter()
            .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
            .count();
        let errors_b = by_repo[b]
            .iter()
            .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
            .count();
        errors_b.cmp(&errors_a)
    });

    for repo in repos {
        let findings = &by_repo[&repo];
        let error_count = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
            .count();
        let warning_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();

        html.push_str(&format!(
            r#"<div class="repo-group">
                <div class="repo-header" onclick="toggleRepo(this)">
                    <span class="repo-icon">📁</span>
                    <span class="repo-name">{}</span>
                    <div class="repo-badges">
                        {}
                        {}
                    </div>
                    <span class="repo-toggle">▼</span>
                </div>
                <div class="repo-content">"#,
            html_escape(&repo),
            if error_count > 0 {
                format!(r#"<span class="badge badge-error">{error_count} errors</span>"#)
            } else {
                String::new()
            },
            if warning_count > 0 {
                format!(r#"<span class="badge badge-warning">{warning_count} warnings</span>"#)
            } else {
                String::new()
            },
        ));

        // Group by file within repo
        let mut by_file: HashMap<String, Vec<&&Finding>> = HashMap::new();
        for finding in findings {
            let file = finding
                .location
                .as_ref()
                .map(|l| l.file.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            by_file.entry(file).or_default().push(finding);
        }

        for (file, file_findings) in &by_file {
            html.push_str(&format!(
                r#"<div class="file-group">
                    <div class="file-header">
                        <span class="file-icon">📄</span>
                        <span class="file-name">{}</span>
                    </div>"#,
                html_escape(file)
            ));

            for finding in file_findings {
                let severity_class = match finding.severity {
                    Severity::Critical => "severity-critical",
                    Severity::Error => "severity-error",
                    Severity::Warning => "severity-warning",
                    Severity::Info => "severity-info",
                };

                let line = finding
                    .location
                    .as_ref()
                    .map(|l| l.line)
                    .unwrap_or(0);

                html.push_str(&format!(
                    r#"<div class="finding-card {severity_class}">
                        <div class="finding-header">
                            <span class="finding-severity">{}</span>
                            <span class="finding-code">{}</span>
                            <span class="finding-line">Line {}</span>
                        </div>
                        <div class="finding-message">{}</div>
                        {}
                        {}
                    </div>"#,
                    html_escape(&finding.severity.to_string()),
                    html_escape(&finding.code),
                    line,
                    html_escape(&finding.message),
                    finding
                        .description
                        .as_ref()
                        .map(|d| format!(
                            r#"<div class="finding-description">{}</div>"#,
                            html_escape(d)
                        ))
                        .unwrap_or_default(),
                    finding
                        .suggestion
                        .as_ref()
                        .map(|s| format!(
                            r#"<div class="finding-suggestion">
                                <span class="suggestion-icon">💡</span>
                                {}
                            </div>"#,
                            html_escape(s)
                        ))
                        .unwrap_or_default(),
                ));
            }

            html.push_str("</div>"); // file-group
        }

        html.push_str("</div></div>"); // repo-content, repo-group
    }

    html
}

/// Generate modules HTML.
fn generate_modules_html(result: &ScanResult) -> String {
    if result.modules.is_empty() {
        return r#"<div class="empty-state">
            <div class="empty-icon">📦</div>
            <h3>No Modules Found</h3>
            <p>No module blocks were found in the scanned files.</p>
        </div>"#
            .to_string();
    }

    // Group by repository
    let mut by_repo: HashMap<String, Vec<&ModuleRef>> = HashMap::new();
    for module in &result.modules {
        let repo = module
            .repository
            .clone()
            .unwrap_or_else(|| "Local".to_string());
        by_repo.entry(repo).or_default().push(module);
    }

    let mut html = String::new();

    for (repo, modules) in &by_repo {
        let issues_count = modules
            .iter()
            .filter(|m| {
                let pattern = format!("'{}'", m.name);
                result
                    .analysis
                    .findings
                    .iter()
                    .any(|f| f.message.contains(&pattern))
            })
            .count();

        html.push_str(&format!(
            r#"<div class="repo-group">
                <div class="repo-header" onclick="toggleRepo(this)">
                    <span class="repo-icon">📁</span>
                    <span class="repo-name">{}</span>
                    <span class="repo-count">{} modules</span>
                    {}
                    <span class="repo-toggle">▼</span>
                </div>
                <div class="repo-content">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Status</th>
                                <th>Name</th>
                                <th>Source</th>
                                <th>Version</th>
                                <th>File</th>
                            </tr>
                        </thead>
                        <tbody>"#,
            html_escape(repo),
            modules.len(),
            if issues_count > 0 {
                format!(r#"<span class="badge badge-warning">{issues_count} issues</span>"#)
            } else {
                String::new()
            },
        ));

        for module in modules {
            let has_issue = {
                let pattern = format!("'{}'", module.name);
                result
                    .analysis
                    .findings
                    .iter()
                    .any(|f| f.message.contains(&pattern))
            };

            let status_class = if has_issue { "status-bad" } else { "status-good" };
            let status_icon = if has_issue { "✗" } else { "✓" };

            let version = module
                .version_constraint
                .as_ref()
                .map(|c| html_escape(&c.raw))
                .unwrap_or_else(|| r#"<span class="no-version">none</span>"#.to_string());

            let source = shorten_source(&module.source.canonical_id(), 50);

            html.push_str(&format!(
                r#"<tr class="{}">
                    <td><span class="status-icon {}">{}</span></td>
                    <td class="name-cell">{}</td>
                    <td class="source-cell" title="{}">{}</td>
                    <td class="version-cell">{}</td>
                    <td class="file-cell">{}:{}</td>
                </tr>"#,
                if has_issue { "row-issue" } else { "" },
                status_class,
                status_icon,
                html_escape(&module.name),
                html_escape(&module.source.canonical_id()),
                html_escape(&source),
                version,
                html_escape(&module.file_path.to_string_lossy()),
                module.line_number,
            ));
        }

        html.push_str("</tbody></table></div></div>");
    }

    html
}

/// Generate providers HTML.
fn generate_providers_html(result: &ScanResult) -> String {
    if result.providers.is_empty() {
        return r#"<div class="empty-state">
            <div class="empty-icon">🔌</div>
            <h3>No Providers Found</h3>
            <p>No provider requirements were found in the scanned files.</p>
        </div>"#
            .to_string();
    }

    // Group by repository
    let mut by_repo: HashMap<String, Vec<&ProviderRef>> = HashMap::new();
    for provider in &result.providers {
        let repo = provider
            .repository
            .clone()
            .unwrap_or_else(|| "Local".to_string());
        by_repo.entry(repo).or_default().push(provider);
    }

    let mut html = String::new();

    for (repo, providers) in &by_repo {
        let issues_count = providers
            .iter()
            .filter(|p| {
                let pattern = format!("'{}'", p.name);
                result
                    .analysis
                    .findings
                    .iter()
                    .any(|f| f.message.contains(&pattern))
            })
            .count();

        html.push_str(&format!(
            r#"<div class="repo-group">
                <div class="repo-header" onclick="toggleRepo(this)">
                    <span class="repo-icon">📁</span>
                    <span class="repo-name">{}</span>
                    <span class="repo-count">{} providers</span>
                    {}
                    <span class="repo-toggle">▼</span>
                </div>
                <div class="repo-content">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Status</th>
                                <th>Name</th>
                                <th>Source</th>
                                <th>Version</th>
                                <th>File</th>
                            </tr>
                        </thead>
                        <tbody>"#,
            html_escape(repo),
            providers.len(),
            if issues_count > 0 {
                format!(r#"<span class="badge badge-warning">{issues_count} issues</span>"#)
            } else {
                String::new()
            },
        ));

        for provider in providers {
            let has_issue = {
                let pattern = format!("'{}'", provider.name);
                result
                    .analysis
                    .findings
                    .iter()
                    .any(|f| f.message.contains(&pattern))
            };

            let status_class = if has_issue { "status-bad" } else { "status-good" };
            let status_icon = if has_issue { "✗" } else { "✓" };

            let version = provider
                .version_constraint
                .as_ref()
                .map(|c| html_escape(&c.raw))
                .unwrap_or_else(|| r#"<span class="no-version">none</span>"#.to_string());

            html.push_str(&format!(
                r#"<tr class="{}">
                    <td><span class="status-icon {}">{}</span></td>
                    <td class="name-cell">{}</td>
                    <td class="source-cell">{}</td>
                    <td class="version-cell">{}</td>
                    <td class="file-cell">{}:{}</td>
                </tr>"#,
                if has_issue { "row-issue" } else { "" },
                status_class,
                status_icon,
                html_escape(&provider.name),
                html_escape(&provider.qualified_source()),
                version,
                html_escape(&provider.file_path.to_string_lossy()),
                provider.line_number,
            ));
        }

        html.push_str("</tbody></table></div></div>");
    }

    html
}

/// Get the embedded CSS.
fn get_css() -> &'static str {
    r"
:root {
    --bg-primary: #0a0e17;
    --bg-secondary: #111827;
    --bg-tertiary: #1f2937;
    --bg-hover: #374151;
    --border: #374151;
    --text-primary: #f9fafb;
    --text-secondary: #9ca3af;
    --text-muted: #6b7280;
    --accent: #3b82f6;
    --accent-hover: #2563eb;
    --success: #10b981;
    --success-bg: rgba(16, 185, 129, 0.1);
    --warning: #f59e0b;
    --warning-bg: rgba(245, 158, 11, 0.1);
    --error: #ef4444;
    --error-bg: rgba(239, 68, 68, 0.1);
    --info: #3b82f6;
    --info-bg: rgba(59, 130, 246, 0.1);
    --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    --font-mono: 'JetBrains Mono', 'Fira Code', Consolas, monospace;
    --radius: 8px;
    --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
    font-family: var(--font-sans);
    background: var(--bg-primary);
    color: var(--text-primary);
    line-height: 1.6;
}

.app {
    display: flex;
    min-height: 100vh;
}

/* Sidebar */
.sidebar {
    width: 260px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    position: fixed;
    height: 100vh;
}

.logo {
    padding: 1.5rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    border-bottom: 1px solid var(--border);
}

.logo-icon {
    font-size: 1.5rem;
    color: var(--accent);
}

.logo-text {
    font-size: 1.25rem;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent), #818cf8);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.nav {
    flex: 1;
    padding: 1rem 0;
}

.nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.5rem;
    color: var(--text-secondary);
    text-decoration: none;
    transition: all 0.2s;
}

.nav-item:hover, .nav-item.active {
    background: var(--bg-tertiary);
    color: var(--text-primary);
}

.nav-item.active {
    border-left: 3px solid var(--accent);
}

.nav-icon { font-size: 1rem; }

.badge {
    margin-left: auto;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    background: var(--bg-hover);
}

.badge-error { background: var(--error-bg); color: var(--error); }
.badge-warning { background: var(--warning-bg); color: var(--warning); }

.sidebar-footer {
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    color: var(--text-muted);
}

.version { font-weight: 600; color: var(--text-secondary); }

/* Main Content */
.main {
    flex: 1;
    margin-left: 260px;
    padding: 2rem;
    max-width: 1200px;
}

.section { margin-bottom: 3rem; }

.section-title {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 1.5rem;
    font-weight: 600;
    margin-bottom: 1.5rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--border);
}

.section-icon { color: var(--accent); }

/* Status Banner */
.status-banner {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 2rem;
    border-radius: var(--radius);
    margin-bottom: 2rem;
}

.status-success { background: linear-gradient(135deg, rgba(16, 185, 129, 0.2), rgba(16, 185, 129, 0.05)); border: 1px solid var(--success); }
.status-warning { background: linear-gradient(135deg, rgba(245, 158, 11, 0.2), rgba(245, 158, 11, 0.05)); border: 1px solid var(--warning); }
.status-error { background: linear-gradient(135deg, rgba(239, 68, 68, 0.2), rgba(239, 68, 68, 0.05)); border: 1px solid var(--error); }

.status-icon {
    font-size: 3rem;
}

.status-success .status-icon { color: var(--success); }
.status-warning .status-icon { color: var(--warning); }
.status-error .status-icon { color: var(--error); }

.status-title {
    font-size: 1.75rem;
    font-weight: 700;
}

.status-subtitle {
    color: var(--text-secondary);
    margin-top: 0.25rem;
}

/* Stats Grid */
.stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    margin-bottom: 2rem;
}

.stat-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1.5rem;
    text-align: center;
    transition: transform 0.2s, box-shadow 0.2s;
}

.stat-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
}

.stat-icon { font-size: 1.5rem; margin-bottom: 0.5rem; }
.stat-value { font-size: 2rem; font-weight: 700; font-family: var(--font-mono); color: var(--accent); }
.stat-label { font-size: 0.875rem; color: var(--text-secondary); margin-top: 0.25rem; }

/* Findings Summary */
.findings-summary {
    display: flex;
    gap: 2rem;
    padding: 1.5rem;
    background: var(--bg-secondary);
    border-radius: var(--radius);
    border: 1px solid var(--border);
}

.finding-stat {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.finding-count {
    font-size: 2rem;
    font-weight: 700;
    font-family: var(--font-mono);
}

.finding-stat.error .finding-count { color: var(--error); }
.finding-stat.warning .finding-count { color: var(--warning); }
.finding-stat.info .finding-count { color: var(--info); }

.finding-label {
    font-size: 0.875rem;
    color: var(--text-secondary);
}

/* Repo Groups */
.repo-group {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 1rem;
    overflow: hidden;
}

.repo-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: var(--bg-tertiary);
    cursor: pointer;
    transition: background 0.2s;
}

.repo-header:hover { background: var(--bg-hover); }

.repo-icon { font-size: 1.25rem; }
.repo-name { font-weight: 600; flex: 1; }
.repo-count { color: var(--text-muted); font-size: 0.875rem; }
.repo-badges { display: flex; gap: 0.5rem; }
.repo-toggle { color: var(--text-muted); transition: transform 0.2s; }
.repo-group.collapsed .repo-toggle { transform: rotate(-90deg); }
.repo-group.collapsed .repo-content { display: none; }

.repo-content { padding: 1rem; }

/* File Groups */
.file-group {
    margin-bottom: 1rem;
}

.file-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0;
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-family: var(--font-mono);
}

/* Finding Cards */
.finding-card {
    background: var(--bg-primary);
    border-radius: var(--radius);
    padding: 1rem;
    margin-bottom: 0.75rem;
    border-left: 4px solid var(--border);
}

.severity-critical { border-left-color: var(--error); background: var(--error-bg); }
.severity-error { border-left-color: var(--error); }
.severity-warning { border-left-color: var(--warning); }
.severity-info { border-left-color: var(--info); }

.finding-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
}

.finding-severity {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    text-transform: uppercase;
}

.severity-critical .finding-severity,
.severity-error .finding-severity { background: var(--error-bg); color: var(--error); }
.severity-warning .finding-severity { background: var(--warning-bg); color: var(--warning); }
.severity-info .finding-severity { background: var(--info-bg); color: var(--info); }

.finding-code {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
}

.finding-line {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-left: auto;
}

.finding-message {
    font-weight: 500;
    margin-bottom: 0.5rem;
}

.finding-description {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
}

.finding-suggestion {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--success);
    background: var(--success-bg);
    padding: 0.75rem;
    border-radius: 4px;
}

/* Warnings Note */
.warnings-note {
    background: var(--warning-bg);
    border: 1px solid var(--warning);
    border-radius: var(--radius);
    padding: 1rem 1.25rem;
    margin-bottom: 1.5rem;
    color: var(--text-primary);
    font-size: 0.9rem;
}

.warnings-note strong {
    color: var(--warning);
}

/* Data Tables */
.data-table {
    width: 100%;
    border-collapse: collapse;
}

.data-table th,
.data-table td {
    padding: 0.75rem 1rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
}

.data-table th {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: var(--bg-tertiary);
}

.data-table tr:hover td { background: var(--bg-hover); }
.row-issue { background: var(--error-bg); }

.status-icon { font-weight: bold; }
.status-good { color: var(--success); }
.status-bad { color: var(--error); }

.name-cell { font-weight: 500; }
.source-cell { font-family: var(--font-mono); font-size: 0.875rem; color: var(--text-secondary); }
.version-cell { font-family: var(--font-mono); }
.file-cell { font-family: var(--font-mono); font-size: 0.75rem; color: var(--text-muted); }

.no-version { color: var(--warning); font-style: italic; }

/* Empty State */
.empty-state {
    text-align: center;
    padding: 4rem 2rem;
    color: var(--text-secondary);
}

.empty-icon { font-size: 3rem; margin-bottom: 1rem; }
.empty-state h3 { font-size: 1.25rem; margin-bottom: 0.5rem; color: var(--text-primary); }

/* Responsive */
@media (max-width: 1024px) {
    .sidebar { display: none; }
    .main { margin-left: 0; }
    .stats-grid { grid-template-columns: repeat(2, 1fr); }
}

@media (max-width: 640px) {
    .stats-grid { grid-template-columns: 1fr; }
    .findings-summary { flex-direction: column; gap: 1rem; }
    .status-banner { flex-direction: column; text-align: center; }
}
"
}

/// Get the embedded JavaScript.
fn get_js() -> &'static str {
    r"
// Toggle repo groups
function toggleRepo(header) {
    const group = header.parentElement;
    group.classList.toggle('collapsed');
}

// Smooth scroll for nav
document.querySelectorAll('.nav-item').forEach(item => {
    item.addEventListener('click', function(e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({ behavior: 'smooth' });
        }
        
        // Update active state
        document.querySelectorAll('.nav-item').forEach(i => i.classList.remove('active'));
        this.classList.add('active');
    });
});

// Update active nav on scroll
const sections = document.querySelectorAll('.section');
const navItems = document.querySelectorAll('.nav-item');

window.addEventListener('scroll', () => {
    let current = '';
    sections.forEach(section => {
        const sectionTop = section.offsetTop;
        if (scrollY >= sectionTop - 100) {
            current = section.getAttribute('id');
        }
    });

    navItems.forEach(item => {
        item.classList.remove('active');
        if (item.getAttribute('href') === '#' + current) {
            item.classList.add('active');
        }
    });
});
"
}

/// Shorten a module source identifier.
fn shorten_source(source: &str, max_len: usize) -> String {
    if source.len() <= max_len {
        return source.to_string();
    }

    // For registry sources, strip the hostname
    if source.contains("registry.terraform.io/") {
        if let Some(idx) = source.find("registry.terraform.io/") {
            let short = &source[idx + 22..];
            if short.len() <= max_len {
                return short.to_string();
            }
        }
    }

    // Generic truncation
    format!("{}...", &source[..max_len.saturating_sub(3)])
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
    use super::*;
    use crate::types::{
        AnalysisResult, Constraint, ModuleSource, RuntimeRef, RuntimeSource,
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
    fn test_html_report_generation() {
        let result = create_test_result();
        let config = Config::default();
        let reporter = HtmlReporter::new(&config);

        let html = reporter.generate(&result).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("MonPhare"));
        assert!(html.contains("All Checks Passed"));
        assert!(html.contains("vpc"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_shorten_source() {
        assert_eq!(
            shorten_source("registry.terraform.io/hashicorp/aws/provider", 30),
            "hashicorp/aws/provider"
        );
    }
}
