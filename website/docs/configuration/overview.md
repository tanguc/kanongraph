---
sidebar_position: 1
title: Overview
---

# Configuration Overview

MonPhare is configured through a YAML file, environment variables, and CLI arguments.

## Config File

The default config file is `monphare.yaml` in the project root. MonPhare also checks for `monphare.yml` and `.monphare.yaml`.

Generate a starter config:

```bash
monphare init
```

Validate a config file:

```bash
monphare validate
```

## Priority

Configuration values are resolved in this order (highest priority first):

1. CLI arguments (e.g., `--strict`, `--format json`)
2. Environment variables (`MONPHARE_GIT_TOKEN`, `MONPHARE_CONFIG`, `MPH_GITHUB_TOKEN`, etc.)
3. `monphare.yaml` config file
4. Built-in defaults

## Minimal Config

A minimal config that just customizes exclusions:

```yaml
scan:
  exclude_patterns:
    - "**/test/**"
    - "**/.terraform/**"
```

## Full Config

A complete configuration with all sections:

```yaml
scan:
  exclude_patterns:
    - "**/test/**"
    - "**/tests/**"
    - "**/examples/**"
    - "**/.terraform/**"
  continue_on_error: false
  max_depth: 100

analysis:
  check_exact_versions: true
  check_prerelease: true
  check_upper_bound: true
  max_age_months: 12

output:
  colored: true
  verbose: false
  pretty: true

git:
  github_token: ${GITHUB_TOKEN}
  gitlab_token: ${GITLAB_TOKEN}
  azure_devops_token: ${AZURE_DEVOPS_PAT}
  bitbucket_token: ${BITBUCKET_APP_PASSWORD}
  branch: main

cache:
  enabled: true
  directory: ${HOME}/.cache/monphare/repos
  ttl_hours: 24
  max_size_mb: 1000

policies:
  require_version_constraint: true
  require_upper_bound: false
  allowed_providers:
    - hashicorp/*
    - terraform-aws-modules/*
  blocked_modules: []
  severity_overrides:
    exact-version: warning

deprecations:
  runtime:
    terraform:
      - version: "< 0.13.0"
        reason: "Legacy Terraform, migrate to v0.13+"
        severity: error
        replacement: ">= 0.13.0"
  modules: {}
  providers: {}
```

## Environment Variable Expansion

String values in the config file support `${VAR}` and `$VAR` syntax for environment variable expansion. If the variable is not set, the placeholder is left as-is.

```yaml
git:
  github_token: ${GITHUB_TOKEN}
```

## Sections

- [Scan Options](./scan-options.md) -- file discovery and parsing behavior
- [Analysis Options](./analysis-options.md) -- which checks to enable
- [Policies](./policies.md) -- enforcement rules and severity overrides
- [Deprecations](./deprecations.md) -- deprecated runtimes, modules, and providers
- [Cache](./cache.md) -- repository caching for remote scans
