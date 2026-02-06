---
sidebar_position: 4
title: validate
---

# validate

Validates a MonPhare configuration file for syntax and structure errors.

## Synopsis

```
monphare validate [FILE]
```

| Argument | Description | Default |
|----------|-------------|---------|
| `[FILE]` | Path to the configuration file to validate. | `monphare.yaml` |

## Examples

Validate the default config file:

```bash
$ monphare validate
Configuration is valid: monphare.yaml
```

Validate a specific file:

```bash
$ monphare validate config/monphare-prod.yaml
Configuration is valid: config/monphare-prod.yaml
```

When validation fails:

```bash
$ monphare validate broken.yaml
Configuration error: scan.max_depth: invalid type: expected usize, found string at line 5 column 14
```

The command exits with code `0` on success and `1` on error.
