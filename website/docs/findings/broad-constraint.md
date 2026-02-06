---
sidebar_position: 3
title: broad-constraint
---

# broad-constraint

| | |
|---|---|
| **Code** | `broad-constraint` |
| **Severity** | warning |
| **Category** | Broad Constraint |

## What it means

A module or provider has a version constraint that is so broad it provides no meaningful protection. The canonical example is `>= 0.0.0`, which matches every possible version.

## Why it matters

An overly broad constraint is functionally equivalent to having no constraint at all. It gives a false sense of safety -- the version field is present, but it does not prevent incompatible upgrades.

## Example

This HCL triggers the finding:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = ">= 0.0.0"
}
```

MonPhare output:

```
WARNING [broad-constraint] Module 'vpc' has overly broad constraint: >= 0.0.0
  --> main.tf:1
  Suggestion: Use a more specific constraint like '~> 1.0' or '>= 1.0, < 2.0'
```

## How to fix

Replace the broad constraint with one that limits the acceptable version range:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"
}
```

Or use an explicit range:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = ">= 5.0, < 6.0"
}
```
