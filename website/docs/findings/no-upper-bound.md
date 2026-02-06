---
sidebar_position: 5
title: no-upper-bound
---

# no-upper-bound

| | |
|---|---|
| **Code** | `no-upper-bound` |
| **Severity** | warning |
| **Category** | Best Practice |

## What it means

A version constraint specifies a lower bound (e.g., `>= 1.0`) but no upper bound. This allows any future version, including major releases with breaking changes.

## Why it matters

Semantic versioning reserves major version bumps for breaking changes. A constraint like `>= 1.0` will happily accept version `2.0.0`, `3.0.0`, and beyond -- each of which may change or remove APIs your configuration depends on. This can cause `terraform plan` failures or, worse, silent behavior changes in your infrastructure.

## Example

This HCL triggers the finding:

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 4.0"
    }
  }
}
```

MonPhare output:

```
WARNING [no-upper-bound] 'aws' has no upper bound on version
  --> versions.tf:4
  Suggestion: Add an upper bound, e.g., '>= 1.0, < 2.0'
```

## How to fix

**Option 1: Use the pessimistic operator (`~>`).**

The `~>` operator automatically sets an upper bound. `~> 4.0` allows `>= 4.0.0` and `< 5.0.0`.

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}
```

**Option 2: Add an explicit upper bound.**

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 5.0, < 6.0"
    }
  }
}
```

Both approaches prevent automatic upgrades across major version boundaries.
