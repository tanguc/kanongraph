---
sidebar_position: 3
title: init
---

# init

Creates a `monphare.yaml` configuration file in the current directory with documented defaults and example values.

## Synopsis

```
monphare init
```

No options. The command fails if `monphare.yaml` already exists in the current directory.

## Example

```bash
$ monphare init
Created example configuration: monphare.yaml
```

The generated file includes all configuration sections with comments explaining each option. See [Configuration Overview](../configuration/overview.md) for details on each section.
