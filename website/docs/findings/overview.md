---
sidebar_position: 1
title: Overview
---

# Findings Overview

MonPhare produces findings when it detects issues with Terraform/OpenTofu version constraints. Each finding has a code, severity, and actionable message.

## Finding Codes

| Code | Severity | Description |
|------|----------|-------------|
| [`missing-version`](./missing-version.md) | error | Module or provider has no version constraint. |
| [`broad-constraint`](./broad-constraint.md) | warning | Version constraint is overly broad (e.g., `>= 0.0.0`). |
| [`wildcard-constraint`](./wildcard-constraint.md) | warning | Version constraint uses a wildcard (`*`). |
| [`no-upper-bound`](./no-upper-bound.md) | warning | Constraint has a lower bound but no upper bound. |
| [`exact-version`](./exact-version.md) | info | Exact version pin prevents automatic patch updates. |
| [`prerelease-version`](./prerelease-version.md) | info | Constraint references a pre-release version. |

## Severity Levels

| Level | Meaning |
|-------|---------|
| `critical` | Severe issue requiring immediate attention. |
| `error` | Definite problem that should be fixed. Causes exit code `2`. |
| `warning` | Potential issue. Causes exit code `1` when `--strict` is enabled. |
| `info` | Informational. No effect on exit code. |

Severities can be overridden per finding code using the [`policies.severity_overrides`](../configuration/policies.md) configuration.
