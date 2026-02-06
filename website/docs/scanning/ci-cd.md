---
sidebar_position: 6
title: CI/CD Integration
---

# CI/CD Integration

MonPhare is designed to fit into CI/CD pipelines. It uses exit codes to signal pass/fail, supports machine-readable JSON output, and provides a `--strict` flag to control what constitutes a failure.

## Exit codes

| Exit code | Meaning |
|---|---|
| `0` | No issues found, or only warnings (without `--strict`) |
| `1` | Warnings found with `--strict` enabled, or a runtime error occurred |
| `2` | Errors found (e.g., missing version constraints when policy requires them) |

The key flag is `--strict`: without it, warnings alone do not fail the pipeline. With `--strict`, any warning bumps the exit code to `1`.

```bash
# warnings are allowed, only errors fail
monphare scan ./infrastructure

# warnings also fail the pipeline
monphare scan ./infrastructure --strict
```

## GitHub Actions

### Basic check on pull requests

```yaml
name: Terraform Constraint Check

on:
  pull_request:
    paths:
      - '**/*.tf'

jobs:
  monphare:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install MonPhare
        run: |
          curl -fsSL https://github.com/tanguc/monphare/releases/latest/download/monphare-x86_64-unknown-linux-gnu.tar.gz \
            | tar xz -C /usr/local/bin

      - name: Scan Terraform constraints
        run: monphare scan . --strict
```

### Organization-wide scheduled audit

```yaml
name: Weekly Terraform Audit

on:
  schedule:
    - cron: '0 8 * * 1'  # every Monday at 8am
  workflow_dispatch:       # allow manual trigger

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - name: Install MonPhare
        run: |
          curl -fsSL https://github.com/tanguc/monphare/releases/latest/download/monphare-x86_64-unknown-linux-gnu.tar.gz \
            | tar xz -C /usr/local/bin

      - name: Run org-wide scan
        env:
          MPH_GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          monphare scan \
            --github my-org \
            --yes \
            --format json \
            --output audit.json \
            --continue-on-error \
            --exclude "**/test/**" \
            --exclude "**/examples/**"

      - name: Upload report
        uses: actions/upload-artifact@v4
        with:
          name: monphare-audit
          path: audit.json

      - name: Check for errors
        run: |
          errors=$(jq '.summary.errors' audit.json)
          if [ "$errors" -gt 0 ]; then
            echo "Found $errors error(s) across the organization"
            jq '.findings[] | select(.errors > 0) | {repository, errors, warnings}' audit.json
            exit 1
          fi
```

### PR comment with HTML report

```yaml
      - name: Generate HTML report
        if: github.event_name == 'pull_request'
        run: monphare scan . --format html --output report.html

      - name: Upload HTML report
        if: github.event_name == 'pull_request'
        uses: actions/upload-artifact@v4
        with:
          name: monphare-report
          path: report.html
```

## GitLab CI

```yaml
monphare:
  stage: validate
  image: ubuntu:latest
  before_script:
    - curl -fsSL https://github.com/tanguc/monphare/releases/latest/download/monphare-x86_64-unknown-linux-gnu.tar.gz
        | tar xz -C /usr/local/bin
  script:
    - monphare scan . --strict --format json --output report.json
  artifacts:
    paths:
      - report.json
    when: always
  rules:
    - changes:
        - '**/*.tf'
```

For organization-wide GitLab group scanning:

```yaml
monphare-audit:
  stage: audit
  image: ubuntu:latest
  variables:
    MPH_GITLAB_TOKEN: $GITLAB_API_TOKEN
  before_script:
    - curl -fsSL https://github.com/tanguc/monphare/releases/latest/download/monphare-x86_64-unknown-linux-gnu.tar.gz
        | tar xz -C /usr/local/bin
  script:
    - monphare scan --gitlab my-group --yes --format json --output audit.json --continue-on-error
  artifacts:
    paths:
      - audit.json
    when: always
  rules:
    - if: $CI_PIPELINE_SOURCE == "schedule"
```

## Using the Docker image

If you prefer not to install a binary, the Docker image works in any CI system that supports containers.

### GitHub Actions with Docker

```yaml
name: Terraform Constraint Check

on:
  pull_request:
    paths:
      - '**/*.tf'

jobs:
  monphare:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/tanguc/monphare:latest
    steps:
      - uses: actions/checkout@v4

      - name: Scan Terraform constraints
        run: monphare scan . --strict
```

### GitLab CI with Docker

```yaml
monphare:
  stage: validate
  image: ghcr.io/tanguc/monphare:latest
  script:
    - monphare scan . --strict --format json --output report.json
  artifacts:
    paths:
      - report.json
    when: always
  rules:
    - changes:
        - '**/*.tf'
```

### Generic Docker usage in CI

For any CI system that can run Docker:

```bash
docker run --rm -v "$(pwd):/workspace" ghcr.io/tanguc/monphare:latest scan /workspace --strict
```

## Using JSON output with jq

JSON output combined with `jq` is useful for custom checks beyond what `--strict` provides:

```bash
# get just the summary
monphare scan . --format json | jq '.summary'

# list all findings with severity "error"
monphare scan . --format json | jq '.findings[].files[].findings[] | select(.severity == "error")'

# count modules missing version constraints
monphare scan . --format json | jq '[.findings[].files[].findings[] | select(.code == "missing-version")] | length'

# list repositories with errors (org scan)
monphare scan --github my-org --yes --format json | jq '.findings[] | select(.errors > 0) | .repository'

# fail if any wildcard constraints exist
result=$(monphare scan . --format json | jq '[.findings[].files[].findings[] | select(.code == "wildcard-constraint")] | length')
if [ "$result" -gt 0 ]; then
  echo "Wildcard constraints detected"
  exit 1
fi
```

## Combining with org-wide scanning for scheduled audits

A common pattern is to run organization-wide scans on a schedule and use the results for compliance tracking:

```bash
#!/bin/bash
# weekly-audit.sh

set -euo pipefail

DATE=$(date +%Y-%m-%d)
REPORT_DIR="/reports/monphare"
mkdir -p "$REPORT_DIR"

# scan entire org
monphare scan \
  --github my-org \
  --yes \
  --format json \
  --output "$REPORT_DIR/audit-$DATE.json" \
  --continue-on-error

# also generate HTML for human review
monphare scan \
  --github my-org \
  --yes \
  --format html \
  --output "$REPORT_DIR/audit-$DATE.html" \
  --continue-on-error

# extract error count
errors=$(jq '.summary.errors' "$REPORT_DIR/audit-$DATE.json")
warnings=$(jq '.summary.warnings' "$REPORT_DIR/audit-$DATE.json")

echo "Audit complete: $errors error(s), $warnings warning(s)"

# exit with appropriate code for alerting
if [ "$errors" -gt 0 ]; then
  exit 2
fi
```

Schedule this with cron, a CI pipeline schedule, or any task runner. The JSON reports can be fed into dashboards, ticketing systems, or compliance databases for trend tracking over time.
