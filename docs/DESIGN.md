# DriftOps Design Document

## Overview

DriftOps is a Terraform/OpenTofu module constraint analyzer and dependency mapper. This document explains the key design decisions, data flow, and architecture of the system.

## Goals

1. **Detect version constraint conflicts** across multiple repositories
2. **Build dependency graphs** for visualization and analysis
3. **Identify risky patterns** in Terraform configurations
4. **Support multiple Git providers** (GitHub, GitLab, Bitbucket, Azure DevOps)
5. **Generate comprehensive reports** in multiple formats
6. **Integrate with CI/CD pipelines** via exit codes

## Non-Goals (v1.0)

- Auto-remediation of conflicts
- Real-time monitoring
- Terraform state analysis
- Cost estimation

## Architecture

### High-Level Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              INPUT LAYER                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
│   │ Local Paths  │    │  Git URLs    │    │   Config     │             │
│   │  ./terraform │    │ github.com/  │    │ driftops.yaml│             │
│   └──────────────┘    └──────────────┘    └──────────────┘             │
│          │                   │                   │                      │
└──────────┼───────────────────┼───────────────────┼──────────────────────┘
           │                   │                   │
           ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            SCANNER LAYER                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                          Scanner                                  │  │
│   │  - Coordinates all operations                                     │  │
│   │  - Manages Git cloning                                           │  │
│   │  - Orchestrates parsing and analysis                             │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│          │                                                               │
│          ▼                                                               │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                        GitClient                                  │  │
│   │  - Detects provider from URL                                      │  │
│   │  - Handles authentication                                         │  │
│   │  - Clones to temp directory                                       │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            PARSER LAYER                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                        HclParser                                  │  │
│   │  - Walks directory tree                                           │  │
│   │  - Parses .tf files using hcl-rs                                 │  │
│   │  - Extracts module blocks                                         │  │
│   │  - Extracts required_providers                                    │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│          │                                                               │
│          ▼                                                               │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                     ModuleRef / ProviderRef                       │  │
│   │  - Structured representation of parsed data                       │  │
│   │  - Includes source, version constraint, location                  │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            GRAPH LAYER                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                       GraphBuilder                                │  │
│   │  - Creates nodes for modules and providers                        │  │
│   │  - Creates edges for dependencies                                 │  │
│   │  - Infers provider requirements from module sources               │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│          │                                                               │
│          ▼                                                               │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                     DependencyGraph                               │  │
│   │  - petgraph DiGraph<GraphNode, EdgeType>                         │  │
│   │  - HashMap for O(1) node lookup                                   │  │
│   │  - Supports cycle detection, traversal                            │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           ANALYZER LAYER                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                    ConstraintAnalyzer                             │  │
│   │  - Groups modules/providers by source                             │  │
│   │  - Performs pairwise constraint comparison                        │  │
│   │  - Detects conflicts (no overlap)                                 │  │
│   │  - Checks for risky patterns                                      │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│          │                                                               │
│          ▼                                                               │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                      AnalysisResult                               │  │
│   │  - List of Findings (code, severity, message, location)          │  │
│   │  - Summary statistics                                             │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           REPORTER LAYER                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌────────────────┐  ┌────────────────┐  ┌────────────────┐           │
│   │  JsonReporter  │  │  TextReporter  │  │  HtmlReporter  │           │
│   │  - Structured  │  │  - CLI output  │  │  - Self-       │           │
│   │  - Machine-    │  │  - Tables      │  │    contained   │           │
│   │    readable    │  │  - Colors      │  │  - Embedded    │           │
│   │                │  │                │  │    CSS/JS      │           │
│   └────────────────┘  └────────────────┘  └────────────────┘           │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            OUTPUT LAYER                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
│   │    stdout    │    │  File Output │    │  Exit Code   │             │
│   │              │    │  report.json │    │  0/1/2       │             │
│   └──────────────┘    └──────────────┘    └──────────────┘             │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. Module Source Parsing

Terraform module sources can be in many formats:

```hcl
# Registry
source = "hashicorp/consul/aws"

# Git
source = "git::https://github.com/example/module.git?ref=v1.0.0"

# Local
source = "../modules/vpc"

# S3
source = "s3::https://s3-eu-west-1.amazonaws.com/bucket/module.zip"
```

**Decision**: Create a `ModuleSource` enum with variants for each type. Use regex patterns for parsing, with a fallback to `Unknown` for unrecognized formats.

**Rationale**: This provides type safety and makes it easy to add new source types later.

### 2. Version Constraint Representation

Terraform uses a rich constraint syntax:

```hcl
version = "~> 5.0"           # Pessimistic
version = ">= 4.0, < 6.0"    # Range
version = "= 1.0.0"          # Exact
```

**Decision**: Parse constraints into a `Constraint` struct containing:
- Raw string for display
- Vector of `VersionRange` enums for evaluation

**Rationale**: Separating the raw string from parsed ranges allows accurate display while enabling programmatic comparison.

### 3. Conflict Detection Algorithm

**Algorithm**:

1. **Group** all modules/providers by canonical source ID
2. **Compare** all pairs within each group
3. **Check overlap** by computing effective bounds for each constraint
4. **Report** conflicts where bounds don't overlap

```rust
// Pseudocode
for (source, items) in group_by_source(modules) {
    for (i, j) in pairs(items) {
        if !constraints_overlap(items[i], items[j]) {
            report_conflict(items[i], items[j]);
        }
    }
}
```

**Complexity**: O(n²) within each group, but groups are typically small.

### 4. Dependency Graph Structure

**Decision**: Use `petgraph::DiGraph` with:
- `GraphNode` enum (Module or Provider)
- `EdgeType` enum (DependsOn, RequiresProvider, etc.)
- HashMap for O(1) lookup by canonical ID

**Rationale**: petgraph provides efficient graph algorithms (cycle detection, traversal) while the HashMap provides fast lookups.

### 5. Error Handling Strategy

**Decision**: Use `thiserror` for library errors, `anyhow` for application-level errors.

**Error categories**:
- `Recoverable`: Continue processing other files
- `Fatal`: Stop processing immediately

**Rationale**: This allows flexible error handling - CLI can continue on parse errors while library users can choose their behavior.

### 6. Git Provider Abstraction

**Decision**: Create a `GitProvider` trait with implementations for each provider.

```rust
#[async_trait]
pub trait GitProvider: Send + Sync {
    fn can_handle(&self, url: &str) -> bool;
    fn normalize_url(&self, url: &str) -> Result<String>;
    async fn clone(&self, url: &str, path: &Path, ...) -> Result<()>;
}
```

**Rationale**: This allows easy addition of new providers and provider-specific authentication handling.

### 7. Report Generation

**Decision**: Self-contained HTML reports with embedded CSS.

**Rationale**: 
- No external dependencies required
- Works offline
- Easy to share via email/Slack
- Can be stored as artifacts

### 8. Configuration Layering

**Priority** (highest to lowest):
1. CLI arguments
2. Environment variables
3. Configuration file
4. Defaults

**Rationale**: Follows the principle of least surprise and allows CI/CD flexibility.

## Performance Considerations

### Target: 1000+ Terraform files in <30 seconds

**Strategies**:

1. **Parallel file parsing**: Use `rayon` for CPU-bound HCL parsing
2. **Shallow Git clones**: Use `depth=1` for faster cloning
3. **Streaming**: Process files as they're found, don't load all into memory
4. **Efficient data structures**: Use `DashMap` for concurrent access

### Memory Efficiency

- String interning for repeated paths/sources
- Lazy loading of file contents
- Streaming JSON output for large reports

## Security Considerations

1. **No unsafe code**: Enforced via `#![forbid(unsafe_code)]`
2. **Git token handling**: Never logged, environment variable support
3. **Path traversal**: Validate paths before file operations
4. **HTML escaping**: All user content escaped in HTML reports

## Future Considerations

### API Module (v0.2.0)

```rust
// Future API structure
mod api {
    mod routes;
    mod handlers;
    mod auth;
}

// REST endpoints
// POST /api/v1/scan
// GET /api/v1/reports/{id}
// GET /api/v1/graph/{id}
```

### Pulumi Support (v0.4.0)

```rust
// Future parser abstraction
trait IaCParser {
    fn parse(&self, content: &str) -> Result<ParsedConfig>;
}

impl IaCParser for HclParser { ... }
impl IaCParser for PulumiParser { ... }
```

### Auto-Remediation (v0.5.0)

```rust
// Future remediation system
struct Remediation {
    finding: Finding,
    suggested_fix: String,
    auto_applicable: bool,
}

fn suggest_remediation(finding: &Finding) -> Option<Remediation>;
fn apply_remediation(remediation: &Remediation) -> Result<()>;
```

## Testing Strategy

### Unit Tests

- Each module has inline `#[cfg(test)]` tests
- Focus on edge cases and error conditions
- Use `pretty_assertions` for readable diffs

### Integration Tests

- Located in `tests/` directory
- Use fixture files in `tests/fixtures/`
- Test end-to-end workflows

### Fixtures

```
tests/fixtures/
├── simple/          # Basic valid configuration
├── conflicts/       # Configurations with conflicts
│   ├── repo_a/
│   └── repo_b/
└── risky/          # Configurations with risky patterns
```

## Glossary

| Term | Definition |
|------|------------|
| **Constraint** | A version requirement (e.g., `~> 5.0`) |
| **Module** | A reusable Terraform configuration |
| **Provider** | A plugin that manages resources (e.g., AWS, GCP) |
| **Source** | The location of a module (registry, Git, local) |
| **Finding** | An issue detected during analysis |
| **Severity** | The importance level of a finding (Info, Warning, Error, Critical) |

