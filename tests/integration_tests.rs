//! Integration tests for MonPhare.
//!
//! These tests verify the end-to-end functionality of the scanner,
//! parser, analyzer, and reporter modules.

use monphare::{Config, Scanner};
use std::path::PathBuf;

/// Get the path to the test fixtures directory.
fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

mod parser_tests {
    use super::*;
    use monphare::parser::HclParser;

    #[tokio::test]
    async fn test_parse_simple_terraform() {
        let config = Config::default();
        let parser = HclParser::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let result = parser.parse_directory(&fixture_path).await.unwrap();

        // Should find 2 modules (vpc, eks)
        assert_eq!(result.modules.len(), 2);

        // Should find 2 providers (aws, random)
        assert_eq!(result.providers.len(), 2);

        // Check module details
        let vpc_module = result.modules.iter().find(|m| m.name == "vpc").unwrap();
        assert!(vpc_module.version_constraint.is_some());
        assert_eq!(
            vpc_module.version_constraint.as_ref().unwrap().raw,
            "~> 5.0"
        );

        let eks_module = result.modules.iter().find(|m| m.name == "eks").unwrap();
        assert!(eks_module.version_constraint.is_some());
    }

    #[tokio::test]
    async fn test_parse_risky_patterns() {
        let config = Config::default();
        let parser = HclParser::new(&config);

        let fixture_path = fixtures_path().join("risky");
        let result = parser.parse_directory(&fixture_path).await.unwrap();

        // Should find modules with various patterns
        assert!(!result.modules.is_empty());

        // Check for module without version
        let no_version = result
            .modules
            .iter()
            .find(|m| m.name == "vpc_no_version")
            .unwrap();
        assert!(no_version.version_constraint.is_none());

        // Check for exact version module
        let exact = result
            .modules
            .iter()
            .find(|m| m.name == "eks_exact")
            .unwrap();
        assert_eq!(exact.version_constraint.as_ref().unwrap().raw, "19.15.3");
    }
}

mod analyzer_tests {
    use super::*;
    use monphare::analyzer::Analyzer;
    use monphare::graph::GraphBuilder;
    use monphare::parser::HclParser;
    use monphare::types::FindingCategory;

    #[tokio::test]
    async fn test_detect_missing_constraints() {
        let config = Config::default();
        let parser = HclParser::new(&config);
        let analyzer = Analyzer::new(&config);

        let fixture_path = fixtures_path().join("risky");
        let parsed = parser.parse_directory(&fixture_path).await.unwrap();

        let graph = GraphBuilder::new()
            .build(&parsed.modules, &parsed.providers, &parsed.runtimes)
            .unwrap();

        let result = analyzer
            .analyze(&graph, &parsed.modules, &parsed.providers, &parsed.runtimes)
            .unwrap();

        // Should find missing constraint findings
        let missing: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == FindingCategory::MissingConstraint)
            .collect();

        assert!(!missing.is_empty(), "Should detect missing constraints");
    }
}

mod scanner_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_scan() {
        let config = Config::default();
        let scanner = Scanner::new(config);

        let fixture_path = fixtures_path().join("simple");
        let result = scanner.scan_paths(vec![fixture_path]).await.unwrap();

        assert!(!result.modules.is_empty());
        assert!(!result.providers.is_empty());
        assert!(!result.files_scanned.is_empty());
    }

    #[tokio::test]
    async fn test_scan_multiple_paths() {
        let config = Config::default();
        let scanner = Scanner::new(config);

        let paths = vec![
            fixtures_path().join("simple"),
            fixtures_path().join("risky"),
        ];

        let result = scanner.scan_paths(paths).await.unwrap();

        // Should have modules from both directories
        assert!(result.modules.len() > 2);
    }
}

mod reporter_tests {
    use super::*;
    use monphare::reporter::Reporter;
    use monphare::types::ReportFormat;

    #[tokio::test]
    async fn test_json_report() {
        let config = Config::default();
        let scanner = Scanner::new(config.clone());
        let reporter = Reporter::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let result = scanner.scan_paths(vec![fixture_path]).await.unwrap();

        let json = reporter.generate(&result, ReportFormat::Json).unwrap();

        // Verify it's valid JSON with new structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["meta"]["version"].is_string());
        assert!(parsed["inventory"]["modules"].is_array());
    }

    #[tokio::test]
    async fn test_text_report() {
        let config = Config::default();
        let scanner = Scanner::new(config.clone());
        let reporter = Reporter::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let result = scanner.scan_paths(vec![fixture_path]).await.unwrap();

        let text = reporter.generate(&result, ReportFormat::Text).unwrap();

        // Text report uses new table-based format
        assert!(text.contains("MonPhare"));
    }

    #[tokio::test]
    async fn test_html_report() {
        let config = Config::default();
        let scanner = Scanner::new(config.clone());
        let reporter = Reporter::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let result = scanner.scan_paths(vec![fixture_path]).await.unwrap();

        let html = reporter.generate(&result, ReportFormat::Html).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("MonPhare"));
        // HTML should be self-contained
        assert!(html.contains("<style>"));
    }
}

mod graph_tests {
    use super::*;
    use monphare::graph::{export_graph, GraphBuilder};
    use monphare::parser::HclParser;
    use monphare::types::GraphFormat;

    #[tokio::test]
    async fn test_graph_building() {
        let config = Config::default();
        let parser = HclParser::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let parsed = parser.parse_directory(&fixture_path).await.unwrap();

        let graph = GraphBuilder::new()
            .build(&parsed.modules, &parsed.providers, &parsed.runtimes)
            .unwrap();

        // Should have nodes for modules and providers
        assert!(graph.node_count() > 0);
    }

    #[tokio::test]
    async fn test_graph_export_dot() {
        let config = Config::default();
        let parser = HclParser::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let parsed = parser.parse_directory(&fixture_path).await.unwrap();

        let graph = GraphBuilder::new()
            .build(&parsed.modules, &parsed.providers, &parsed.runtimes)
            .unwrap();

        let dot = export_graph(&graph, GraphFormat::Dot).unwrap();

        assert!(dot.contains("digraph"));
    }

    #[tokio::test]
    async fn test_graph_export_mermaid() {
        let config = Config::default();
        let parser = HclParser::new(&config);

        let fixture_path = fixtures_path().join("simple");
        let parsed = parser.parse_directory(&fixture_path).await.unwrap();

        let graph = GraphBuilder::new()
            .build(&parsed.modules, &parsed.providers, &parsed.runtimes)
            .unwrap();

        let mermaid = export_graph(&graph, GraphFormat::Mermaid).unwrap();

        assert!(mermaid.contains("graph TD"));
    }
}

mod config_tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        let yaml = r#"
scan:
  exclude_patterns:
    - "**/vendor/**"
  continue_on_error: true
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert!(config.scan.continue_on_error);
        assert!(config
            .scan
            .exclude_patterns
            .contains(&"**/vendor/**".to_string()));
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.analysis.check_exact_versions);
        assert!(config.analysis.check_prerelease);
        assert_eq!(config.scan.max_depth, 100);
    }
}
