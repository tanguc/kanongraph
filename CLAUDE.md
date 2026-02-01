# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MonPhare is a Terraform/OpenTofu module constraint analyzer and dependency mapper written in Rust. It scans Terraform repositories, parses HCL files, builds dependency graphs, and detects version constraint conflicts, deprecated modules, and risky patterns across multiple repositories.

## Build & Test Commands

### Building
```bash
# Standard debug build
cargo build

# Release build (with optimizations)
cargo build --release

# Check compilation without building
cargo check --all-features
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with all features enabled
cargo test --all-features

# Run specific test by name
cargo test <test_name>

# Run tests in a specific module
cargo test --test integration_tests

# Run with output visible
cargo test -- --nocapture

# Run tests without stopping on first failure
cargo test --no-fail-fast
```

### Linting & Formatting
```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --all -- --check

# Run clippy lints
cargo clippy --all-features

# Clippy with warnings as errors (CI mode)
cargo clippy --all-features -- -D warnings
```

### Running the CLI
```bash
# Run from source
cargo run -- <args>

# Examples:
cargo run -- scan ./terraform
cargo run -- scan --repo https://github.com/org/repo --format json
cargo run -- graph ./terraform --format dot
cargo run -- init
```

### Documentation
```bash
# Generate and open docs
cargo doc --open --no-deps

# Check docs build without warnings
cargo doc --no-deps --all-features
```

### Coverage
```bash
# Install cargo-llvm-cov first: cargo install cargo-llvm-cov
cargo llvm-cov --all-features --lcov --output-path lcov.info
```

## Architecture

### Data Flow Pipeline

1. **Input Layer**: Accepts local paths, Git URLs, or configuration files
2. **Scanner Layer**: Coordinates operations, manages Git cloning via `GitClient`
3. **Parser Layer**: Uses `HclParser` to walk directories and parse .tf files with hcl-rs, extracting `ModuleRef` and `ProviderRef` structures
4. **Graph Layer**: `GraphBuilder` creates dependency graph using petgraph DiGraph with nodes for modules/providers and edges for dependencies
5. **Analyzer Layer**: `Analyzer` performs policy checks (missing constraints, risky patterns, broad constraints) and deprecation detection
6. **Reporter Layer**: Generates reports in JSON, Text, or HTML formats
7. **Output Layer**: Produces stdout, file output, and appropriate exit codes (0=success, 1=warnings with --strict, 2=errors)

### Module Organization

```
src/
├── lib.rs              # Library entry point, Scanner orchestrator
├── main.rs             # CLI entry point
├── types.rs            # Core data structures (ModuleRef, ProviderRef, RuntimeRef, etc.)
├── error.rs            # Error types using thiserror
├── config.rs           # Configuration handling
├── parser/             # HCL parsing
│   ├── mod.rs
│   ├── hcl.rs          # HCL parser implementation using hcl-rs
│   └── source.rs       # Module source parsing (registry, Git, local, S3)
├── graph/              # Dependency graph
│   ├── mod.rs
│   ├── types.rs        # Graph node and edge types
│   ├── builder.rs      # Graph construction from parsed modules
│   └── export.rs       # Export to DOT, JSON, Mermaid formats
├── analyzer/           # Constraint analysis
│   ├── mod.rs
│   ├── conflict.rs     # Constraint conflict detection (O(n²) pairwise comparison)
│   ├── patterns.rs     # Risky pattern detection
│   └── deprecation.rs  # Deprecation checks
├── reporter/           # Report generation
│   ├── mod.rs
│   ├── json.rs         # JSON reports (machine-readable)
│   ├── text.rs         # CLI text reports with colors and tables
│   └── html.rs         # Self-contained HTML reports with embedded CSS
├── git/                # Git provider abstraction
│   ├── mod.rs
│   ├── client.rs       # Git client orchestrator
│   └── providers.rs    # Provider implementations (GitHub, GitLab, Bitbucket, Azure DevOps)
├── vcs.rs              # VCS platform enumeration
├── vcs_clients.rs      # VCS client implementations
└── cli/                # CLI interface using clap
    └── mod.rs
```

### Key Data Structures

- **ModuleRef**: Represents a Terraform module block with source, version constraint, file location
- **ProviderRef**: Represents a provider requirement with version constraint
- **RuntimeRef**: Represents Terraform/OpenTofu version requirements
- **ModuleSource**: Enum for different source types (Registry, Git, Local, S3, Unknown)
- **Constraint**: Version constraint with raw string and parsed VersionRange vector
- **DependencyGraph**: petgraph DiGraph with GraphNode (Module/Provider) and EdgeType, plus HashMap for O(1) lookup
- **Finding**: Analysis issue with code (DRIFT002-007), severity, message, location

### Module Source Parsing

Supports multiple Terraform source formats:
- **Registry**: `hashicorp/consul/aws`
- **Git**: `git::https://github.com/example/module.git?ref=v1.0.0`
- **Local**: `../modules/vpc`
- **S3**: `s3::https://s3-eu-west-1.amazonaws.com/bucket/module.zip`

Uses regex patterns with fallback to `Unknown` for unrecognized formats.

## Finding Codes

- **DRIFT002**: Missing version constraint
- **DRIFT003**: Wildcard constraint (using `*`)
- **DRIFT004**: Overly broad constraint (e.g., `>= 0.0.0`)
- **DRIFT005**: Pre-release version
- **DRIFT006**: Exact version constraint (prevents updates)
- **DRIFT007**: No upper bound (allows breaking changes)

## Configuration

Configuration priority (highest to lowest):
1. CLI arguments
2. Environment variables (e.g., `MONPHARE_GIT_TOKEN`)
3. Configuration file (`monphare.yaml`)
4. Defaults

Generate example config: `monphare init`

## Development Notes

- **No unsafe code**: Enforced via `#![forbid(unsafe_code)]` in Cargo.toml
- **Minimum Rust version**: 1.70
- **Parallel processing**: Uses `rayon` for CPU-bound parsing, `tokio` for async operations
- **Test fixtures**: Located in `tests/fixtures/` with subdirectories for different scenarios (simple, conflicts, risky)
- **Error handling**: Uses `thiserror` for library errors, `anyhow` for application-level errors with continue-on-error support
- **Git operations**: Shallow clones (`depth=1`) for performance

## CI/CD

The GitHub Actions workflow runs:
- `cargo check --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --all-features -- -D warnings`
- `cargo test --all-features` (on Ubuntu, macOS, Windows with Rust stable and 1.70)
- `cargo doc --no-deps --all-features` (with `-D warnings`)
- `cargo llvm-cov` for coverage
- `cargo audit` for security
- Release builds for multiple targets on tags
