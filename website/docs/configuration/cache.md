---
sidebar_position: 6
title: Cache
---

# Cache

The `cache` section controls how MonPhare caches cloned repositories. Caching avoids redundant clones when scanning the same repositories repeatedly.

```yaml
cache:
  enabled: true
  directory: ~/.cache/monphare/repos
  ttl_hours: 24
  max_size_mb: 1000
```

## Fields

### `enabled`

Enable or disable repository caching.

**Default:** `true`

When disabled, MonPhare performs a fresh shallow clone for every remote scan.

### `directory`

Path to the cache directory. Supports environment variable expansion.

**Default:** `~/.cache/monphare/repos`

```yaml
cache:
  directory: ${HOME}/.cache/monphare/repos
```

### `ttl_hours`

Time-to-live in hours. After this period, cached repositories are refreshed on next access using `git fetch` instead of a full re-clone.

**Default:** `24`

### `fresh_threshold_minutes`

If a cached repository was updated within this many minutes, skip fetching entirely. This prevents repeated fetches during back-to-back scans.

**Default:** `5`

### `max_size_mb`

Maximum total cache size in megabytes. When exceeded, the oldest cached repositories are evicted.

**Default:** `1000` (1 GB)

## How It Works

MonPhare uses shallow clones (`depth=1`) for initial repository cloning. On subsequent scans, instead of cloning again, it runs `git fetch` on the cached copy to pull the latest changes. This is significantly faster for large repositories.

The cache key is derived from the repository URL, so the same repository scanned from different projects shares a single cache entry.

## Examples

Disable caching (always fresh clones):

```yaml
cache:
  enabled: false
```

Short TTL for fast-moving repos:

```yaml
cache:
  ttl_hours: 1
  max_size_mb: 2000
```

Custom cache location for CI:

```yaml
cache:
  directory: /tmp/monphare-cache
  ttl_hours: 48
```
