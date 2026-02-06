---
sidebar_position: 1
title: Local Directories
---

# Local Directories

MonPhare scans local directories for Terraform and OpenTofu files (`.tf`), recursively walking subdirectories to find module blocks, provider requirements, and version constraints.

## Scanning a single directory

Pass a directory path as a positional argument to the `scan` command:

```bash
monphare scan ./infrastructure
```

MonPhare walks the directory recursively, parses every `.tf` file it finds, and reports any constraint issues.

## Scanning multiple directories

You can pass multiple paths in a single invocation. Each directory is scanned independently and results are combined:

```bash
monphare scan ./infrastructure ./modules ./shared
```

This is useful when your Terraform code is spread across several directories within the same repository or workspace.

## What gets scanned

MonPhare looks for all files ending in `.tf` within the given directories. It parses:

- `module` blocks -- extracts `source` and `version` attributes
- `required_providers` blocks inside `terraform {}` -- extracts provider source and version constraints
- `required_version` inside `terraform {}` -- extracts runtime version requirements

Other file types (`.tfvars`, `.json`, `.hcl`) are ignored. Hidden directories (`.terraform`, `.git`) are excluded by default.

## How directories map to repository labels

In the output, each scanned directory becomes a "repository" label. MonPhare uses the directory name (the last path component) as the label. For example:

| Path passed | Repository label in output |
|---|---|
| `./infrastructure` | `infrastructure` |
| `/home/user/repos/my-project` | `my-project` |
| `.` | current directory name |

This label appears in text, JSON, and HTML reports to group findings by source.

## Example

Given a directory with the following Terraform file:

```hcl
# infrastructure/main.tf
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 4.0, < 6.0"
    }
  }
}

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"
}

module "s3_bucket" {
  source = "terraform-aws-modules/s3-bucket/aws"
  # no version constraint
}
```

Running:

```bash
monphare scan ./infrastructure
```

Produces output similar to:

```
MonPhare Scan Report
Status: PASSED (with warnings)
Files scanned: 1 | Modules: 2 | Providers: 1

Repository: infrastructure
+-------+---------+------------------+-----------+------+------------------------------------------+
| Code  | Severity| Module/Provider  | File      | Line | Message                                  |
+-------+---------+------------------+-----------+------+------------------------------------------+
| missing-version | warning | s3_bucket | main.tf | 13 | Module 's3_bucket' has no version constraint |
+-------+---------+------------------+-----------+------+------------------------------------------+

1 warning(s), 0 error(s)
```

## Combining with other flags

Local directory scanning works with all other flags:

```bash
# save results as JSON
monphare scan ./infrastructure --format json --output report.json

# exclude test directories
monphare scan ./infrastructure --exclude "**/test/**"

# fail on warnings (useful in CI)
monphare scan ./infrastructure --strict

# limit recursion depth
monphare scan ./infrastructure --max-depth 5
```

See [Filtering and Excluding](./filtering-and-excluding.md) and [Output Formats](./output-formats.md) for details on these options.
