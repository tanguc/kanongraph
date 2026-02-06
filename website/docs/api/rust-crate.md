---
sidebar_position: 1
title: Rust Crate
---

# Rust Crate

MonPhare is available as a Rust library crate in addition to the CLI. You can use it to build custom tooling, integrate with your own CI pipelines, or embed scanning into other Rust applications.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
monphare = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Key Types

| Type | Description |
|------|-------------|
| `Scanner` | Main entry point. Scans paths or repositories and returns results. |
| `Config` | Configuration struct. Loaded from YAML or constructed with defaults. |
| `ScanResult` | Contains modules, providers, dependency graph, and analysis findings. |
| `ReportFormat` | Enum: `Json`, `Text`, `Html`. |
| `GraphFormat` | Enum: `Dot`, `Json`, `Mermaid`. |
| `ModuleRef` | A parsed module block with source, version constraint, and location. |
| `ProviderRef` | A parsed provider requirement with source and version constraint. |
| `Finding` | A single analysis finding with code, severity, message, and location. |

## Basic Usage: Scan and Report

```rust
use monphare::{Scanner, Config, ReportFormat};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::default();
    let scanner = Scanner::new(config);

    let paths = vec![PathBuf::from("./terraform")];
    let result = scanner.scan_paths(paths).await?;

    println!("Modules found: {}", result.modules.len());
    println!("Providers found: {}", result.providers.len());
    println!("Findings: {}", result.analysis.findings.len());

    // generate a JSON report
    let report = result.generate_report(ReportFormat::Json)?;
    println!("{}", report);

    Ok(())
}
```

## Graph Generation

```rust
use monphare::{Scanner, Config};
use monphare::graph::export_graph;
use monphare::types::GraphFormat;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::default();
    let scanner = Scanner::new(config);

    let result = scanner.scan_paths(vec![PathBuf::from("./infra")]).await?;

    // export as DOT for Graphviz
    let dot = export_graph(&result.graph, GraphFormat::Dot)?;
    std::fs::write("deps.dot", &dot)?;

    // export as Mermaid
    let mermaid = export_graph(&result.graph, GraphFormat::Mermaid)?;
    std::fs::write("deps.mmd", &mermaid)?;

    Ok(())
}
```

## Custom Configuration

```rust
use monphare::Config;

let yaml = r#"
scan:
  exclude_patterns:
    - "**/test/**"
  continue_on_error: true
analysis:
  check_exact_versions: false
  check_upper_bound: true
"#;

let config = Config::from_yaml(yaml).unwrap();
```

## API Documentation

Full API documentation is available on [docs.rs/monphare](https://docs.rs/monphare) (when published).

You can also generate docs locally:

```bash
cargo doc --open --no-deps
```
