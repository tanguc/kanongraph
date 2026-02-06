---
sidebar_position: 2
title: Remote Repositories
---

# Remote Repositories

MonPhare can clone and scan remote Git repositories without requiring you to have them checked out locally. Repositories are shallow-cloned (`depth=1`) for speed and cached locally to avoid redundant network calls.

## Scanning a remote repo

Use the `--repo` flag with a Git-compatible URL:

```bash
monphare scan --repo https://github.com/my-org/infra-modules
```

MonPhare clones the repository into a local cache, scans all `.tf` files, and reports findings. The repository name (e.g., `infra-modules`) is used as the label in the output.

## Multiple repositories

Pass `--repo` multiple times to scan several repositories in one run:

```bash
monphare scan \
  --repo https://github.com/my-org/infra-modules \
  --repo https://github.com/my-org/platform-config \
  --repo https://github.com/my-org/data-pipelines
```

Results from all repositories appear in a single combined report, grouped by repository name.

## Authentication

Private repositories require a Git token. You can provide it in two ways:

### Environment variable (preferred)

```bash
export MONPHARE_GIT_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
monphare scan --repo https://github.com/my-org/private-repo
```

You can also use platform-specific environment variables which take precedence over the generic one:

| Platform | Environment variable |
|---|---|
| GitHub | `MPH_GITHUB_TOKEN` |
| GitLab | `MPH_GITLAB_TOKEN` |
| Azure DevOps | `MPH_AZURE_DEVOPS_TOKEN` |
| Bitbucket | `MPH_BITBUCKET_TOKEN` |
| Any (fallback) | `MONPHARE_GIT_TOKEN` |

### CLI flag

```bash
monphare scan --repo https://github.com/my-org/private-repo --git-token ghp_xxxxxxxxxxxxxxxxxxxx
```

The `--git-token` flag is convenient for one-off usage but avoid it in scripts where the token might end up in shell history. Prefer the environment variable approach.

## Checking out a specific branch

By default, MonPhare uses the repository's default branch. To scan a different branch:

```bash
monphare scan --repo https://github.com/my-org/infra-modules --branch feature/new-modules
```

The `--branch` flag applies to all repositories in the same command.

## Repository caching

Cloned repositories are cached in `~/.cache/monphare/repos` (or the platform-equivalent cache directory). On subsequent scans, MonPhare runs `git fetch` to check for updates instead of cloning from scratch.

Cache behavior is configurable in `monphare.yaml`:

```yaml
cache:
  enabled: true
  directory: ~/.cache/monphare/repos
  ttl_hours: 24
  fresh_threshold_minutes: 5
  max_size_mb: 1000
```

- `ttl_hours` -- after this period, the cache entry is refreshed on next access
- `fresh_threshold_minutes` -- if the cache was updated within this window, skip fetching entirely
- `max_size_mb` -- oldest entries are evicted when total cache size exceeds this limit

To force a fresh clone, delete the cache directory:

```bash
rm -rf ~/.cache/monphare/repos
```

## Private repository example

A full example scanning two private GitHub repos with authentication:

```bash
export MPH_GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx

monphare scan \
  --repo https://github.com/acme-corp/terraform-networking \
  --repo https://github.com/acme-corp/terraform-compute \
  --branch main \
  --format json \
  --output audit.json
```

This clones both repos (or updates the cache), scans all `.tf` files on the `main` branch, and writes a JSON report to `audit.json`.
