---
sidebar_position: 2
title: Scan Options
---

# Scan Options

The `scan` section controls how MonPhare discovers and parses Terraform files.

```yaml
scan:
  exclude_patterns:
    - "**/test/**"
    - "**/tests/**"
    - "**/examples/**"
    - "**/.terraform/**"
  continue_on_error: false
  max_depth: 100
```

## Fields

### `exclude_patterns`

A list of glob patterns. Files and directories matching any of these patterns are skipped during scanning.

**Default:**

```yaml
exclude_patterns:
  - "**/test/**"
  - "**/tests/**"
  - "**/examples/**"
  - "**/.terraform/**"
```

Patterns specified via `--exclude` on the CLI are appended to this list.

### `continue_on_error`

When `true`, MonPhare continues scanning even if individual files fail to parse. Errors are logged as warnings instead of stopping the scan.

**Default:** `false`

This is useful when scanning large organizations where some repositories may contain invalid HCL.

### `max_depth`

Maximum depth for recursive directory traversal. Prevents runaway scanning in deeply nested directory structures.

**Default:** `100`

## Examples

Skip vendor directories and limit depth:

```yaml
scan:
  exclude_patterns:
    - "**/vendor/**"
    - "**/node_modules/**"
    - "**/.terraform/**"
  max_depth: 20
```

Resilient scanning for CI across many repos:

```yaml
scan:
  continue_on_error: true
  exclude_patterns:
    - "**/test/**"
    - "**/fixtures/**"
```
