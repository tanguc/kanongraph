---
sidebar_position: 7
title: prerelease-version
---

# prerelease-version

| | |
|---|---|
| **Code** | `prerelease-version` |
| **Severity** | info |
| **Category** | Best Practice |

## What it means

A version constraint references a pre-release version -- one that includes a suffix like `-alpha`, `-beta`, `-rc`, `-dev`, or `-pre`.

## Why it matters

Pre-release versions are explicitly marked as unstable by their authors. APIs may change, bugs are expected, and there is no guarantee of forward compatibility. Using pre-release versions in production infrastructure increases the risk of unexpected behavior.

This finding is reported at `info` severity because there are legitimate reasons to use pre-release versions in non-production environments (e.g., testing upcoming features).

## Example

This HCL triggers the finding:

```hcl
module "k8s" {
  source  = "terraform-aws-modules/eks/aws"
  version = "20.0.0-beta1"
}
```

MonPhare output:

```
INFO [prerelease-version] 'k8s' uses pre-release version
  --> main.tf:1
  Suggestion: Consider using a stable release version
```

## How to fix

Switch to the latest stable release:

```hcl
module "k8s" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 20.0"
}
```

If you need to use a pre-release version intentionally, you can disable this check:

```yaml
# monphare.yaml
analysis:
  check_prerelease: false
```
