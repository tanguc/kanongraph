---
sidebar_position: 1
title: scan
---

# scan

Scan local directories, Git repositories, or entire VCS organizations for Terraform/OpenTofu files. Analyzes version constraints, detects risky patterns, and generates reports.

**Alias:** `s`

## Synopsis

```
monphare scan [OPTIONS] [PATH]...
```

## Options

| Short | Long | Env Var | Description | Default |
|-------|------|---------|-------------|---------|
| | `[PATH]...` | | Local directories to scan. If omitted, scans the current directory. | `.` |
| `-r` | `--repo <URL>` | | Git repository URL to clone and scan. Can be repeated. | |
| | `--github <ORG>` | | Scan all repositories in a GitHub organization. | |
| | `--gitlab <GROUP>` | | Scan all projects in a GitLab group. | |
| | `--ado <ORG[/PROJECT]>` | | Scan repositories in an Azure DevOps organization, or a specific project within it. | |
| | `--bitbucket <WORKSPACE>` | | Scan all repositories in a Bitbucket workspace. | |
| | `--yes` | | Skip confirmation prompt when scanning large organizations. | `false` |
| `-f` | `--format <FORMAT>` | | Output format: `text`, `json`, or `html`. | `text` |
| `-o` | `--output <FILE>` | | Write report to a file instead of stdout. | |
| | `--strict` | | Treat warnings as errors (exit code 1). | `false` |
| | `--continue-on-error` | | Continue scanning when individual files or repos fail to parse. | `false` |
| | `--max-depth <N>` | | Maximum depth for recursive directory scanning. | `100` |
| `-e` | `--exclude <PATTERN>` | | Glob pattern to exclude from scanning. Can be repeated. | |
| | `--branch <BRANCH>` | | Git branch to checkout after cloning. | default branch |
| | `--git-token <TOKEN>` | `MONPHARE_GIT_TOKEN` | Authentication token for private Git repositories. | |

The `--github`, `--gitlab`, `--ado`, and `--bitbucket` options are mutually exclusive with `[PATH]` and `--repo`.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success -- no errors found. |
| `1` | Warnings found and `--strict` is enabled, or a runtime error occurred. |
| `2` | Errors found in the analysis (e.g., missing version constraints). |

## Examples

Scan the current directory:

```bash
monphare scan
```

Scan specific directories:

```bash
monphare scan ./infra ./modules/networking
```

Scan a remote repository:

```bash
monphare scan --repo https://github.com/org/terraform-infra
```

Scan multiple repositories:

```bash
monphare scan \
  --repo https://github.com/org/repo1 \
  --repo https://github.com/org/repo2
```

Scan an entire GitHub organization:

```bash
export MONPHARE_GIT_TOKEN=ghp_xxxx
monphare scan --github my-org --yes
```

Generate a JSON report and write to file:

```bash
monphare scan ./infra --format json --output report.json
```

Generate an HTML report:

```bash
monphare scan ./infra --format html --output report.html
```

Strict mode in CI (fail on warnings):

```bash
monphare scan ./infra --strict --continue-on-error
```

Exclude test fixtures:

```bash
monphare scan ./infra -e "**/test/**" -e "**/fixtures/**"
```

Scan a private repo with a specific branch:

```bash
monphare scan \
  --repo https://github.com/org/private-repo \
  --git-token ghp_xxxx \
  --branch develop
```
