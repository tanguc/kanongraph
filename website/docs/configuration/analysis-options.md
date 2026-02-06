---
sidebar_position: 3
title: Analysis Options
---

# Analysis Options

The `analysis` section controls which checks MonPhare performs on version constraints.

```yaml
analysis:
  check_exact_versions: true
  check_prerelease: true
  check_upper_bound: true
  max_age_months: 12
```

## Fields

### `check_exact_versions`

When `true`, MonPhare flags modules and providers that pin to an exact version. Exact pins prevent automatic patch updates.

**Default:** `true` | **Finding:** [`exact-version`](../findings/exact-version.md)

This Terraform triggers the check:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.1.2"    # exact pin -- no patches applied
}
```

MonPhare output:

```
INFO  module.vpc  Exact version  main.tf:1
```

Set to `false` if your team intentionally pins versions and manages updates manually.

### `check_prerelease`

When `true`, MonPhare flags version constraints that reference pre-release versions. Pre-release versions may be unstable.

**Default:** `true` | **Finding:** [`prerelease-version`](../findings/prerelease-version.md)

This Terraform triggers the check:

```hcl
module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "20.0.0-beta1"    # pre-release in production
}
```

MonPhare output:

```
INFO  module.eks  Pre-release  main.tf:1
```

### `check_upper_bound`

When `true`, MonPhare flags constraints that have a lower bound but no upper bound. Missing upper bounds allow breaking changes from major version bumps.

**Default:** `true` | **Finding:** [`no-upper-bound`](../findings/no-upper-bound.md)

This Terraform triggers the check:

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 4.0"    # v5.0, v6.0, v99.0 all accepted
    }
  }
}
```

MonPhare output:

```
WARN  provider.aws  No upper bound  versions.tf:4
```

Fix with `~>` or an explicit range:

```hcl
version = "~> 5.0"          # allows 5.x, blocks 6.0
version = ">= 5.0, < 6.0"  # same thing, explicit
```

### `max_age_months`

Flag modules that have not been updated in this many months. Set to `0` to disable.

**Default:** `12`

## Common profiles

### Strict -- production infrastructure

Everything on, short age window. Paired with `--strict` in CI to fail on warnings.

```yaml
analysis:
  check_exact_versions: true
  check_prerelease: true
  check_upper_bound: true
  max_age_months: 6
```

### Relaxed -- development / sandbox

Only catch the critical stuff. Let teams experiment.

```yaml
analysis:
  check_exact_versions: false
  check_prerelease: false
  check_upper_bound: true
  max_age_months: 0    # disabled
```

### Pinned versions team

Team that intentionally pins exact versions and manages upgrades via PRs.

```yaml
analysis:
  check_exact_versions: false    # intentional, not a problem
  check_prerelease: true         # still catch accidental pre-releases
  check_upper_bound: false       # exact pins have no range to bound
  max_age_months: 12             # still flag very old pins
```
