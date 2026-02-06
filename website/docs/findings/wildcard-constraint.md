---
sidebar_position: 4
title: wildcard-constraint
---

# wildcard-constraint

| | |
|---|---|
| **Code** | `wildcard-constraint` |
| **Severity** | warning |
| **Category** | Best Practice |

## What it means

A module or provider uses `*` as its version constraint, which matches any version.

## Why it matters

A wildcard constraint accepts every version, including future major releases with breaking changes. It provides zero protection against incompatible upgrades and makes it impossible to reproduce a specific infrastructure state.

## Example

This HCL triggers the finding:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "*"
}
```

MonPhare output:

```
WARNING [wildcard-constraint] 'vpc' uses wildcard version constraint
  --> main.tf:1
  Suggestion: Replace with a specific constraint like '~> 1.0'
```

## How to fix

Replace the wildcard with a meaningful constraint:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"
}
```
