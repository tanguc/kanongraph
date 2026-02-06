---
sidebar_position: 5
title: Deprecations
---

# Deprecations

The `deprecations` section lets you define rules that flag deprecated runtimes, modules, and providers. This is the primary mechanism for enforcing version lifecycle policies across your organization.

```yaml
deprecations:
  runtime: {}
  modules: {}
  providers: {}
```

## Structure

Each deprecation rule has four fields:

| Field | Description |
|-------|-------------|
| `version` | A version constraint string (e.g., `"< 0.13.0"`, `"1.0.1"`, `">= 2.0.8, < 3.0.0"`, `"~> 3.1"`). Matched against the version in use. |
| `reason` | Why this version is deprecated. Shown to the user in findings. |
| `severity` | Severity level: `info`, `warning`, `error`, or `critical`. |
| `replacement` | Suggested replacement version or module. |

Rules can also use `git_ref` instead of `version` to match Git refs, tags, or commit hashes.

## Runtime Deprecations

Flag specific versions of Terraform or OpenTofu. The key is the runtime name (`terraform` or `opentofu`).

```yaml
deprecations:
  runtime:
    terraform:
      - version: "< 0.13.0"
        reason: "Legacy Terraform version, state migration required"
        severity: error
        replacement: ">= 0.13.0"
      - version: ">= 0.13.0, < 1.0.0"
        reason: "Terraform 0.x is approaching end of life"
        severity: warning
        replacement: ">= 1.0.0"
    opentofu:
      - version: "< 1.6.0"
        reason: "Known state locking issues in early versions"
        severity: warning
        replacement: ">= 1.6.0"
```

When MonPhare encounters a `required_version` constraint in a `terraform` block, it checks whether the version matches any of these rules.

## Module Deprecations

Flag specific module versions by their registry source identifier. The key is the module source in `namespace/name/provider` format.

```yaml
deprecations:
  modules:
    "claranet/azure-log-mngt-v1/azurerm":
      - version: "1.0.1"
        reason: "Critical security vulnerability CVE-2023-1234"
        severity: error
        replacement: "claranet/azure-log-mngt-v3/azurerm"
      - version: ">= 2.0.8, < 3.0.0"
        reason: "Breaking API changes, migrate to v3"
        severity: warning
        replacement: "claranet/azure-log-mngt-v3/azurerm"
      - version: "~> 3.1"
        reason: "v3.1.x has known performance issues"
        severity: warning
        replacement: "claranet/azure-log-mngt-v3/azurerm >= 3.2.0"
    "claranet/azure-log-mngt-v2/azurerm":
      - version: "0.0.10"
        reason: "CVE-2024-5678 detected during security audit"
        severity: error
        replacement: "claranet/azure-log-mngt-v3/azurerm"
```

Multiple rules per module are supported. Each rule is evaluated independently.

## Provider Deprecations

Flag specific provider versions. The key is the provider source in `namespace/name` format.

```yaml
deprecations:
  providers:
    "hashicorp/azurerm":
      - version: "> 0.0.1, < 3.50.0"
        reason: "Multiple CVEs in versions before 3.50.0"
        severity: error
        replacement: ">= 3.50.0"
      - version: ">= 4.0.0, < 4.50.0"
        reason: "Known authentication issues in early 4.x versions"
        severity: warning
        replacement: ">= 4.50.0"
    "hashicorp/aws":
      - version: "< 4.0.0"
        reason: "AWS provider 3.x is no longer maintained"
        severity: warning
        replacement: ">= 4.0.0"
```

## Version Constraint Syntax

The `version` field supports the full Terraform constraint syntax:

| Syntax | Meaning |
|--------|---------|
| `"1.0.1"` | Exact match |
| `"< 0.13.0"` | Less than |
| `">= 2.0.8, < 3.0.0"` | Range (AND of multiple constraints) |
| `"~> 3.1"` | Pessimistic -- allows `>= 3.1.0` and `< 4.0.0` |
| `"<= 0.13.0"` | Less than or equal |
| `"> 1.0.0"` | Greater than |

## Use Case: Shared Deprecations Config

A common pattern is for a security or platform team to maintain a central deprecations file that is shared across all teams. This file can be stored in a dedicated repository and referenced by each project.

Example workflow:

1. Security team maintains `deprecations.yaml` in a shared repo
2. CI pipelines download the latest version before running MonPhare
3. Teams merge the shared config with their project-specific `monphare.yaml`

```bash
# download shared deprecation rules
curl -sL https://raw.githubusercontent.com/org/policies/main/deprecations.yaml -o /tmp/deprecations.yaml

# run monphare with the shared config
monphare scan ./infra --config /tmp/deprecations.yaml
```

Or teams can include the deprecations directly in their `monphare.yaml` alongside other project-specific settings.
