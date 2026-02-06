---
sidebar_position: 6
title: exact-version
---

# exact-version

| | |
|---|---|
| **Code** | `exact-version` |
| **Severity** | info |
| **Category** | Best Practice |

## What it means

A module or provider pins to an exact version (e.g., `= 1.0.0` or `1.0.0` with no operator). No other version will be accepted.

## Why it matters

Exact version pins provide maximum stability but prevent automatic patch updates. When a critical security fix is released as `1.0.1`, an exact pin to `1.0.0` will not pick it up until someone manually updates the constraint.

This is a tradeoff. Some teams intentionally pin exact versions and manage updates through a controlled process. MonPhare reports this at `info` severity -- it is not necessarily wrong, but you should be aware of it.

## Example

This HCL triggers the finding:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.1.2"
}
```

MonPhare output:

```
INFO [exact-version] 'vpc' uses exact version constraint
  --> main.tf:1
  Suggestion: Consider using '~> X.Y.0' to allow patch updates
```

## How to fix

If you want to receive patch updates automatically, use the pessimistic operator at the minor level:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.1.0"
}
```

This allows `5.1.x` patches but blocks `5.2.0` and above.

If the exact pin is intentional (e.g., for compliance), you can suppress this finding by setting its severity to `info` or disabling it:

```yaml
# monphare.yaml
analysis:
  check_exact_versions: false
```
