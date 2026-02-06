---
sidebar_position: 3
title: Organization-wide Scanning
---

# Organization-wide Scanning

MonPhare can discover and scan every repository in a GitHub organization, GitLab group, Azure DevOps organization or project, or Bitbucket workspace. This is the most powerful way to audit Terraform constraints across your entire infrastructure codebase.

## Supported platforms

### GitHub

Scan all repositories in a GitHub organization:

```bash
export MPH_GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
monphare scan --github my-org
```

The token needs the `repo` scope (or `read:org` + `repo` for organization-level access to private repos).

### GitLab

Scan all projects in a GitLab group (including subgroups):

```bash
export MPH_GITLAB_TOKEN=glpat-xxxxxxxxxxxxxxxxxxxx
monphare scan --gitlab my-group
```

The token needs `read_api` and `read_repository` scopes.

### Azure DevOps

Scan all repositories across all projects in an Azure DevOps organization:

```bash
export MPH_AZURE_DEVOPS_TOKEN=xxxxxxxxxxxxxxxxxxxxxxxx
monphare scan --ado my-org
```

Or narrow down to a specific project:

```bash
monphare scan --ado my-org/my-project
```

The token needs `Code (Read)` scope.

### Bitbucket

Scan all repositories in a Bitbucket workspace:

```bash
export MPH_BITBUCKET_TOKEN=xxxxxxxxxxxxxxxxxxxxxxxx
monphare scan --bitbucket my-workspace
```

The token needs repository read permissions. For Bitbucket Cloud, use an App Password with `Repositories: Read` permission.

## Authentication

All organization-wide scanning modes require a token. MonPhare resolves tokens with the following priority:

1. Platform-specific config value in `monphare.yaml` (e.g., `git.github_token`)
2. Platform-specific environment variable (e.g., `MPH_GITHUB_TOKEN`)
3. Legacy fallback environment variable (`MONPHARE_GIT_TOKEN`)

You can also pass `--git-token` on the command line, which applies as a fallback across all platforms.

See the full token resolution table in [Remote Repositories](./remote-repositories.md#authentication).

## Confirmation prompt for large organizations

When scanning an organization with many repositories, MonPhare displays a confirmation prompt:

```
Found 147 repositories in 'my-org'. Proceed with scanning? [y/N]
```

To skip this prompt (useful in CI or automated scripts), use the `--yes` flag:

```bash
monphare scan --github my-org --yes
```

## Mode exclusivity

Organization-wide scanning flags (`--github`, `--gitlab`, `--ado`, `--bitbucket`) conflict with `--repo` and positional path arguments. You can only use one scanning mode at a time:

```bash
# valid -- org scan
monphare scan --github my-org

# valid -- repo scan
monphare scan --repo https://github.com/my-org/repo

# valid -- local scan
monphare scan ./infrastructure

# invalid -- will error
monphare scan --github my-org --repo https://github.com/my-org/repo
monphare scan --github my-org ./infrastructure
```

## Filtering repositories

When scanning an organization, you can use `git.include_patterns` and `git.exclude_patterns` in `monphare.yaml` to control which repositories are scanned:

```yaml
git:
  exclude_patterns:
    - "archived-*"
    - "deprecated-*"
    - "sandbox-*"
  include_patterns:
    - "terraform-*"
    - "infra-*"
```

You can also combine with `--exclude` for file-level exclusions within each repository:

```bash
monphare scan --github my-org --exclude "**/test/**" --exclude "**/examples/**"
```

## Full org audit example

Run a complete audit of a GitHub organization, output as JSON, and save to a file:

```bash
export MPH_GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx

monphare scan \
  --github acme-corp \
  --yes \
  --format json \
  --output org-audit.json \
  --continue-on-error \
  --exclude "**/test/**" \
  --exclude "**/examples/**"
```

This will:

1. List all repositories in the `acme-corp` GitHub organization
2. Skip the confirmation prompt (`--yes`)
3. Clone/update each repository using the local cache
4. Scan all `.tf` files, skipping test and example directories
5. Continue scanning even if individual files fail to parse
6. Write a JSON report with findings grouped by repository

The JSON output is well-suited for feeding into dashboards, JIRA ticket automation, or custom compliance checks. See [Output Formats](./output-formats.md) for the JSON structure.

## Scheduled audits

Organization-wide scanning pairs well with scheduled CI jobs. See [CI/CD Integration](./ci-cd.md) for examples of setting up weekly cron-based audits that scan your entire org and post results.
