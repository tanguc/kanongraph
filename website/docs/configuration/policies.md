---
sidebar_position: 4
title: Policies
---

# Policies

The `policies` section defines enforcement rules for version constraints. Policies let you require certain practices and control the severity of findings.

```yaml
policies:
  require_version_constraint: true
  require_upper_bound: false
  allowed_providers:
    - hashicorp/*
    - terraform-aws-modules/*
  blocked_modules: []
  severity_overrides: {}
```

## Fields

### `require_version_constraint`

When `true`, every non-local module must have a `version` attribute. Modules without a version constraint produce an `error`-level finding.

**Default:** `true`

### `require_upper_bound`

When `true`, all version constraints must include an upper bound (via `~>`, `<`, or `<=`). Constraints like `>= 1.0` without a ceiling are flagged.

**Default:** `false`

### `allowed_providers`

A whitelist of provider source patterns (glob syntax). When this list is non-empty, any provider not matching at least one pattern is flagged.

```yaml
policies:
  allowed_providers:
    - hashicorp/*
    - terraform-aws-modules/*
```

An empty list (default) allows all providers.

### `blocked_modules`

A blacklist of module source patterns (glob syntax). Any module matching a pattern in this list is flagged.

```yaml
policies:
  blocked_modules:
    - deprecated-org/*
    - legacy/old-module/*
```

### `severity_overrides`

Override the default severity for specific finding codes. Valid severity values are `info`, `warning`, `error`, and `critical`.

```yaml
policies:
  severity_overrides:
    missing-version: warning    # downgrade from error
    wildcard-constraint: error  # upgrade from warning
    exact-version: warning      # upgrade from info
```

Available finding codes: `missing-version`, `broad-constraint`, `wildcard-constraint`, `no-upper-bound`, `exact-version`, `prerelease-version`.

## Examples

Strict enterprise policy:

```yaml
policies:
  require_version_constraint: true
  require_upper_bound: true
  allowed_providers:
    - hashicorp/*
  blocked_modules:
    - unmaintained-org/*
  severity_overrides:
    no-upper-bound: error
```

Relaxed policy for development teams:

```yaml
policies:
  require_version_constraint: true
  require_upper_bound: false
  severity_overrides:
    missing-version: warning
    exact-version: info
```
