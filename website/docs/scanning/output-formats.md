---
sidebar_position: 5
title: Output Formats
---

# Output Formats

MonPhare supports three output formats: text (default), JSON, and HTML. Use the `--format` flag to select one, and `--output` to write to a file instead of stdout.

## Text (default)

The text format produces a colored, table-based report designed for terminal use:

```bash
monphare scan ./infrastructure
```

Example output:

```
MonPhare Scan Report
Status: PASSED (with warnings)
Files scanned: 3 | Modules: 5 | Providers: 2 | Repositories: 1

Repository: infrastructure
+-------------------+----------+------------+-------------+------+----------------------------------------------+
| Code              | Severity | Resource   | File        | Line | Message                                      |
+-------------------+----------+------------+-------------+------+----------------------------------------------+
| missing-version   | warning  | s3_bucket  | modules.tf  |   12 | Module 's3_bucket' has no version constraint |
| no-upper-bound    | warning  | eks        | main.tf     |   38 | Constraint '>= 1.0' has no upper bound       |
+-------------------+----------+------------+-------------+------+----------------------------------------------+

2 warning(s), 0 error(s)
```

Colors are enabled by default and auto-detected based on terminal support. Disable with `--quiet` or set `output.colored: false` in config.

## JSON

JSON output is machine-readable and designed for CI pipelines, dashboards, and custom tooling:

```bash
monphare scan ./infrastructure --format json
```

The JSON structure has four top-level sections:

```json
{
  "meta": {
    "tool": "monphare",
    "version": "0.1.1",
    "timestamp": "2026-02-06T10:30:00Z",
    "files_scanned": 3
  },
  "status": {
    "passed": true,
    "exit_code": 0,
    "message": "Passed with warnings"
  },
  "summary": {
    "total_findings": 2,
    "errors": 0,
    "warnings": 2,
    "infos": 0,
    "modules": {
      "total": 5,
      "with_issues": 2,
      "local": 1
    },
    "providers": {
      "total": 2,
      "with_issues": 0
    },
    "repositories": 1
  },
  "findings": [
    {
      "repository": "infrastructure",
      "errors": 0,
      "warnings": 2,
      "files": [
        {
          "path": "modules.tf",
          "findings": [
            {
              "code": "missing-version",
              "severity": "warning",
              "category": "constraint",
              "line": 12,
              "message": "Module 's3_bucket' has no version constraint",
              "suggestion": "Add a version constraint like: version = \"~> 1.0\""
            }
          ]
        }
      ]
    }
  ],
  "inventory": {
    "modules": [
      {
        "name": "vpc",
        "source": {
          "type": "registry",
          "canonical": "terraform-aws-modules/vpc/aws",
          "is_local": false
        },
        "version": "~> 5.0",
        "location": {
          "repository": "infrastructure",
          "path": "main.tf",
          "line": 18
        },
        "has_issues": false
      }
    ],
    "providers": [
      {
        "name": "aws",
        "source": "hashicorp/aws",
        "version": ">= 4.0, < 6.0",
        "location": {
          "repository": "infrastructure",
          "path": "main.tf",
          "line": 7
        },
        "has_issues": false
      }
    ]
  }
}
```

Key sections:

- **`status`** -- check `passed` and `exit_code` first for quick pass/fail decisions
- **`summary`** -- aggregate counts for quick overview
- **`findings`** -- issues grouped by repository, then by file
- **`inventory`** -- complete list of all modules and providers found

Pretty-printing is enabled by default. Disable with `output.pretty: false` in config for compact output.

## HTML

HTML output generates a self-contained report with embedded CSS -- no external dependencies needed. It is suitable for sharing with team leads, attaching to tickets, or hosting as a static page:

```bash
monphare scan ./infrastructure --format html --output report.html
```

The report includes:

- Summary dashboard with pass/fail status and finding counts
- Findings table grouped by repository with severity indicators
- Full inventory of modules and providers
- Responsive layout that works in any browser

Open the file directly in a browser:

```bash
open report.html        # macOS
xdg-open report.html    # Linux
```

## Writing output to a file

Use `--output` (or `-o`) to write the report to a file instead of stdout:

```bash
# JSON to file
monphare scan ./infrastructure --format json --output report.json

# HTML to file
monphare scan ./infrastructure --format html --output report.html

# text to file (strips colors automatically)
monphare scan ./infrastructure --output report.txt
```

Without `--output`, the report is printed to stdout, which makes it easy to pipe into other tools:

```bash
# pipe JSON to jq
monphare scan ./infrastructure --format json | jq '.summary'

# count errors
monphare scan ./infrastructure --format json | jq '.summary.errors'
```
