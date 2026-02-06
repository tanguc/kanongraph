---
sidebar_position: 2
title: missing-version
---

# missing-version

| | |
|---|---|
| **Code** | `missing-version` |
| **Severity** | error |
| **Category** | Missing Constraint |

## What it means

A module or provider declaration has no `version` attribute. Without a version constraint, Terraform will use whatever version happens to be available, which can change between runs and across environments.

## Why it matters

Missing version constraints lead to non-reproducible infrastructure. A `terraform init` today might pull version 3.x, while the same command next week pulls 4.x with breaking changes. This causes unpredictable plan diffs and potential outages.

## Example

This HCL triggers the finding:

```hcl
module "vpc" {
  source = "terraform-aws-modules/vpc/aws"
  # no version specified
}
```

MonPhare output:

```
ERROR [missing-version] Module 'vpc' has no version constraint
  --> main.tf:1
  Suggestion: Add a version constraint, e.g., version = "~> 1.0"
```

## How to fix

Add a `version` attribute with a constraint:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"
}
```

For providers, add a `version` in the `required_providers` block:

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

The `~>` (pessimistic) operator is recommended -- it allows patch and minor updates while preventing breaking major version changes.
