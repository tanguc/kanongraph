---
sidebar_position: 4
title: Filtering and Excluding
---

# Filtering and Excluding

MonPhare provides several mechanisms to control which files and directories are included in a scan. This is useful for skipping test fixtures, examples, vendored code, or deeply nested directories that are not relevant.

## Exclude patterns

Use `--exclude` to skip files and directories matching a glob pattern. The flag can be passed multiple times:

```bash
monphare scan ./infrastructure \
  --exclude "**/test/**" \
  --exclude "**/examples/**" \
  --exclude "**/vendor/**"
```

Patterns use standard glob syntax:

| Pattern | What it matches |
|---|---|
| `**/test/**` | any `test` directory at any depth |
| `**/examples/**` | any `examples` directory at any depth |
| `**/.terraform/**` | Terraform state/cache directories |
| `**/vendor/**` | vendored dependencies |
| `**/modules/legacy_*/**` | directories starting with `legacy_` under `modules` |

## Default exclusions

MonPhare excludes the following patterns by default (even without explicit `--exclude` flags):

- `**/test/**`
- `**/tests/**`
- `**/examples/**`
- `**/.terraform/**`

These defaults come from the built-in configuration. If you provide a `monphare.yaml` config file with a `scan.exclude_patterns` list, your list replaces the defaults entirely.

## Max depth

Use `--max-depth` to limit how deep MonPhare recurses into subdirectories:

```bash
# only scan 3 levels deep
monphare scan ./infrastructure --max-depth 3
```

The default max depth is `100`, which is effectively unlimited for practical purposes. Lower values are useful for large monorepos where you only want top-level module directories.

## Continue on error

By default, MonPhare stops and reports an error if it encounters an unparseable `.tf` file. Use `--continue-on-error` to skip broken files and keep scanning:

```bash
monphare scan ./infrastructure --continue-on-error
```

Skipped files are reported as scan warnings in the output, so you still know what was missed.

## Config file approach

For persistent exclude patterns, define them in `monphare.yaml` under the `scan` section:

```yaml
scan:
  exclude_patterns:
    - "**/test/**"
    - "**/tests/**"
    - "**/examples/**"
    - "**/.terraform/**"
    - "**/vendor/**"
    - "**/legacy/**"
  continue_on_error: true
  max_depth: 50
```

Generate a starter config with `monphare init`, then edit as needed.

CLI flags are additive with config file patterns: if your config already excludes `**/test/**` and you pass `--exclude "**/staging/**"` on the command line, both patterns apply.

## Common patterns to exclude

Here are patterns commonly used in real Terraform codebases:

```yaml
scan:
  exclude_patterns:
    # test and example directories
    - "**/test/**"
    - "**/tests/**"
    - "**/examples/**"
    - "**/fixtures/**"

    # terraform internal directories
    - "**/.terraform/**"
    - "**/.terragrunt-cache/**"

    # vendored or generated code
    - "**/vendor/**"
    - "**/.external_modules/**"

    # documentation with embedded HCL snippets
    - "**/docs/**"
```

## Combining with organization scanning

Exclude patterns work the same way with remote and organization-wide scans. They are applied per-repository after cloning:

```bash
monphare scan --github my-org \
  --exclude "**/test/**" \
  --exclude "**/examples/**" \
  --exclude "**/.terraform/**" \
  --continue-on-error \
  --yes
```
