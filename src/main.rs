//! MonPhare CLI entry point.
//!
//! This binary provides the command-line interface for MonPhare.

use clap::Parser;
use monphare::cli::{Cli, Commands};
use monphare::{Config, Scanner, VcsPlatform};
use std::process::ExitCode;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> ExitCode {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.quiet);

    // Run the appropriate command
    match run(cli).await {
        Ok(exit_code) => exit_code,
        Err(e) => {
            // print clean error message
            eprintln!("Error: {e}");

            // Print backtrace if RUST_BACKTRACE is set
            let backtrace = e.backtrace();
            if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                eprintln!("\nStack backtrace:");
                let backtrace_str = format!("{backtrace}");
                let mut in_monphare = false;
                let mut prev_was_at = false;

                for line in backtrace_str.lines() {
                    let trimmed = line.trim();

                    // Check if this is a monphare frame
                    if trimmed.contains("monphare::") {
                        in_monphare = true;
                        prev_was_at = false;
                        eprintln!("{line}");
                    } else if in_monphare
                        && trimmed.starts_with("at ")
                        && trimmed.contains("./src/")
                    {
                        // This is the location line for a monphare frame
                        eprintln!("{line}");
                        in_monphare = false;
                        prev_was_at = true;
                    } else if !prev_was_at && line.starts_with("   ") && line.contains(":") {
                        // This might be a frame number, check next iteration
                        in_monphare = false;
                        prev_was_at = false;
                    } else {
                        prev_was_at = false;
                    }
                }
            }

            ExitCode::from(1)
        }
    }
}

fn init_logging(verbose: u8, quiet: bool) {
    let filter = if quiet {
        EnvFilter::new("error")
    } else {
        // First try to use RUST_LOG from environment, otherwise use verbose flag
        EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default filter: show logs for monphare only, suppress all other crates
            let base_level = match verbose {
                0 => "warn",
                1 => "info",
                2 => "debug",
                _ => "trace",
            };
            // Filter string: monphare at specified level, everything else at warn
            EnvFilter::new(format!("warn,monphare={base_level}"))
        })
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(false))
        .with(filter)
        .with(ErrorLayer::default())
        .init();
}

async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    // Load configuration
    tracing::debug!("Loading configuration");
    let mut config = load_config(&cli)?;
    tracing::debug!("Configuration loaded successfully");

    // merge CLI arguments into config
    config.merge_cli_args(&cli);
    tracing::debug!(
        strict_mode = config.scan.strict_mode,
        verbose = config.output.verbose,
        "CLI arguments merged into configuration"
    );

    match cli.command {
        Commands::Scan(args) => {
            tracing::debug!("Executing scan command");
            let scanner = Scanner::new(config.clone());

            // Handle bulk VCS organization scanning
            let bulk_scan = args.github.is_some()
                || args.gitlab.is_some()
                || args.ado.is_some()
                || args.bitbucket.is_some();
            tracing::debug!(bulk_scan = bulk_scan, "Scan mode determined");

            let result = if bulk_scan {
                // Bulk organization scanning
                let org_spec = if let Some(org) = &args.github {
                    (VcsPlatform::GitHub, org.clone())
                } else if let Some(group) = &args.gitlab {
                    (VcsPlatform::GitLab, group.clone())
                } else if let Some(org_proj) = &args.ado {
                    (VcsPlatform::AzureDevOps, org_proj.clone())
                } else if let Some(workspace) = &args.bitbucket {
                    (VcsPlatform::Bitbucket, workspace.clone())
                } else {
                    unreachable!()
                };

                scanner
                    .scan_vcs_organization(org_spec.0, &org_spec.1, args.yes)
                    .await?
            } else if !args.repositories.is_empty() {
                let urls: Vec<&str> = args.repositories.iter().map(String::as_str).collect();
                scanner.scan_repositories(&urls).await?
            } else {
                // Single directory scanning
                scanner.scan_paths(args.paths).await?
            };

            // Generate report
            let reporter = monphare::reporter::Reporter::new(&config);
            let report = reporter.generate(&result, args.format)?;

            // Output report
            if let Some(output_path) = args.output {
                std::fs::write(&output_path, &report)?;
                tracing::info!(path = %output_path.display(), "Report written");
            } else {
                println!("{report}");
            }

            // Return appropriate exit code
            let exit_code = if result.analysis.has_errors() {
                2 // Errors found
            } else if result.analysis.has_warnings() && args.strict {
                1 // Warnings in strict mode
            } else {
                0 // Success
            };

            Ok(ExitCode::from(exit_code))
        }

        Commands::Graph(args) => {
            let scanner = Scanner::new(config);
            let result = scanner.scan_paths(args.paths).await?;

            // Output graph in requested format
            let graph_output = monphare::graph::export_graph(&result.graph, args.format)?;

            if let Some(output_path) = args.output {
                std::fs::write(&output_path, &graph_output)?;
                tracing::info!(path = %output_path.display(), "Graph written");
            } else {
                println!("{graph_output}");
            }

            Ok(ExitCode::from(0))
        }

        Commands::Init => {
            // Generate example configuration file
            let example_config = Config::example_yaml();
            let config_path = std::path::Path::new("monphare.yaml");

            if config_path.exists() {
                anyhow::bail!(
                    "Configuration file already exists: {}",
                    config_path.display()
                );
            }

            std::fs::write(config_path, example_config)?;
            println!("Created example configuration: monphare.yaml");
            Ok(ExitCode::from(0))
        }

        Commands::Validate(args) => {
            // Validate configuration file
            let config_content = std::fs::read_to_string(&args.config)?;
            match Config::from_yaml(&config_content) {
                Ok(_) => {
                    println!("Configuration is valid: {}", args.config.display());
                    Ok(ExitCode::from(0))
                }
                Err(e) => {
                    eprintln!("Configuration error: {e}");
                    Ok(ExitCode::from(1))
                }
            }
        }
    }
}

fn load_config(cli: &Cli) -> anyhow::Result<Config> {
    // Check for explicit config file
    if let Some(ref config_path) = cli.config {
        tracing::debug!(path = %config_path.display(), "Loading configuration from explicit path");
        let content = std::fs::read_to_string(config_path)?;
        let mut config = Config::from_yaml(&content)?;
        config.load_vcs_tokens_from_env();
        return Ok(config);
    }

    // Look for default config files
    let default_paths = ["monphare.yaml", "monphare.yml", ".monphare.yaml"];
    tracing::debug!("Searching for default configuration files");
    for path in &default_paths {
        if std::path::Path::new(path).exists() {
            tracing::debug!(path = %path, "Found configuration file");
            let content = std::fs::read_to_string(path)?;
            let mut config = Config::from_yaml(&content)?;
            config.load_vcs_tokens_from_env();
            return Ok(config);
        }
    }

    tracing::debug!("No configuration file found, using default configuration");
    // Use default configuration
    let mut config = Config::default();
    config.load_vcs_tokens_from_env();
    Ok(config)
}
