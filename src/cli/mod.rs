//! Command-line interface module.
//!
//! This module defines the CLI structure using Clap, including
//! all commands, arguments, and options.
//!
//! # Commands
//!
//! - `scan`: Scan directories or repositories for Terraform files
//! - `graph`: Generate dependency graph visualizations
//! - `init`: Create an example configuration file
//! - `validate`: Validate a configuration file
//!
//! # Example Usage
//!
//! ```bash
//! # Scan local directories
//! monphare scan ./terraform ./modules
//!
//! # Scan remote repositories
//! monphare scan --repo https://github.com/org/repo1 --repo https://github.com/org/repo2
//!
//! # Generate JSON report
//! monphare scan ./terraform --format json --output report.json
//!
//! # Generate dependency graph
//! monphare graph ./terraform --format dot --output deps.dot
//!
//! # Initialize configuration
//! monphare init
//!
//! # Validate configuration
//! monphare validate monphare.yaml
//! ```

use crate::types::{GraphFormat, ReportFormat};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// MonPhare - Terraform/OpenTofu module constraint analyzer and dependency mapper.
#[derive(Parser, Debug)]
#[command(
    name = "monphare",
    author,
    version,
    about = "Terraform/OpenTofu module constraint analyzer and dependency mapper",
    long_about = "MonPhare scans Terraform/OpenTofu repositories, parses HCL files, builds \
                  dependency graphs, and detects version constraint conflicts, deprecated \
                  modules, and risky patterns.",
    after_help = "For more information, visit: https://github.com/yourusername/monphare"
)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, global = true, env = "MONPHARE_CONFIG")]
    pub config: Option<PathBuf>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan directories or repositories for Terraform/OpenTofu files
    #[command(visible_alias = "s")]
    Scan(ScanArgs),

    /// Generate dependency graph visualization
    #[command(visible_alias = "g")]
    Graph(GraphArgs),

    /// Create an example configuration file
    Init,

    /// Validate a configuration file
    Validate(ValidateArgs),
}

/// Arguments for the scan command.
#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Paths to scan (directories containing Terraform files)
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Git repository URLs to clone and scan
    #[arg(short, long = "repo", value_name = "URL")]
    pub repositories: Vec<String>,

    /// GitHub organization to scan all repositories from
    #[arg(long, value_name = "ORG", conflicts_with = "paths", conflicts_with = "repositories")]
    pub github: Option<String>,

    /// GitLab group to scan all projects from
    #[arg(long, value_name = "GROUP", conflicts_with = "paths", conflicts_with = "repositories")]
    pub gitlab: Option<String>,

    /// Azure DevOps organization or project to scan (format: 'org' for all projects, or 'org/project' for single project)
    #[arg(long, value_name = "ORG[/PROJECT]", conflicts_with = "paths", conflicts_with = "repositories")]
    pub ado: Option<String>,

    /// Bitbucket workspace to scan all repositories from
    #[arg(long, value_name = "WORKSPACE", conflicts_with = "paths", conflicts_with = "repositories")]
    pub bitbucket: Option<String>,

    /// Skip confirmation prompt for large organizations
    #[arg(long)]
    pub yes: bool,
    /// Output format
    #[arg(short, long, default_value = "text", value_enum)]
    pub format: ReportFormat,

    /// Output file path (stdout if not specified)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Treat warnings as errors (exit code 1)
    #[arg(long)]
    pub strict: bool,

    /// Continue scanning even if some files fail to parse
    #[arg(long)]
    pub continue_on_error: bool,

    /// Maximum depth for recursive directory scanning
    #[arg(long, default_value = "100")]
    pub max_depth: usize,

    /// Patterns to exclude from scanning (glob patterns)
    #[arg(short, long = "exclude", value_name = "PATTERN")]
    pub exclude_patterns: Vec<String>,

    /// Only show errors and warnings (no info-level findings)
    #[arg(long)]
    pub errors_only: bool,

    /// Git branch to checkout after cloning (default: default branch)
    #[arg(long, value_name = "BRANCH")]
    pub branch: Option<String>,

    /// Git authentication token for private repositories
    #[arg(long, env = "MONPHARE_GIT_TOKEN", hide_env_values = true)]
    pub git_token: Option<String>,
}

/// Arguments for the graph command.
#[derive(Args, Debug)]
pub struct GraphArgs {
    /// Paths to scan
    #[arg(value_name = "PATH", required = true)]
    pub paths: Vec<PathBuf>,

    /// Output format for the graph
    #[arg(short, long, default_value = "dot", value_enum)]
    pub format: GraphFormat,

    /// Output file path (stdout if not specified)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Include only modules (exclude providers)
    #[arg(long)]
    pub modules_only: bool,

    /// Include only providers (exclude modules)
    #[arg(long)]
    pub providers_only: bool,

    /// Filter to specific module sources (partial match)
    #[arg(long, value_name = "FILTER")]
    pub filter: Option<String>,
}

/// Arguments for the validate command.
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to configuration file to validate
    #[arg(value_name = "FILE", default_value = "monphare.yaml")]
    pub config: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parsing() {
        // Verify CLI structure is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_scan_command() {
        let cli = Cli::parse_from(["monphare", "scan", "./terraform"]);
        match cli.command {
            Commands::Scan(args) => {
                assert_eq!(args.paths.len(), 1);
                assert_eq!(args.paths[0], PathBuf::from("./terraform"));
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_scan_with_options() {
        let cli = Cli::parse_from([
            "monphare",
            "scan",
            "./terraform",
            "--format",
            "json",
            "--output",
            "report.json",
            "--strict",
        ]);
        match cli.command {
            Commands::Scan(args) => {
                assert_eq!(args.format, ReportFormat::Json);
                assert_eq!(args.output, Some(PathBuf::from("report.json")));
                assert!(args.strict);
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_scan_with_repos() {
        let cli = Cli::parse_from([
            "monphare",
            "scan",
            "--repo",
            "https://github.com/org/repo1",
            "--repo",
            "https://github.com/org/repo2",
        ]);
        match cli.command {
            Commands::Scan(args) => {
                assert_eq!(args.repositories.len(), 2);
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_graph_command() {
        let cli = Cli::parse_from([
            "monphare",
            "graph",
            "./terraform",
            "--format",
            "mermaid",
        ]);
        match cli.command {
            Commands::Graph(args) => {
                assert_eq!(args.format, GraphFormat::Mermaid);
            }
            _ => panic!("Expected Graph command"),
        }
    }

    #[test]
    fn test_init_command() {
        let cli = Cli::parse_from(["monphare", "init"]);
        assert!(matches!(cli.command, Commands::Init));
    }

    #[test]
    fn test_validate_command() {
        let cli = Cli::parse_from(["monphare", "validate", "custom.yaml"]);
        match cli.command {
            Commands::Validate(args) => {
                assert_eq!(args.config, PathBuf::from("custom.yaml"));
            }
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_global_options() {
        let cli = Cli::parse_from([
            "monphare",
            "-vvv",
            "--config",
            "custom.yaml",
            "scan",
            "./terraform",
        ]);
        assert_eq!(cli.verbose, 3);
        assert_eq!(cli.config, Some(PathBuf::from("custom.yaml")));
    }

    #[test]
    fn test_alias() {
        let cli = Cli::parse_from(["monphare", "s", "./terraform"]);
        assert!(matches!(cli.command, Commands::Scan(_)));
    }
}

