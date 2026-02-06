---
sidebar_position: 2
title: Quick Start
---

# Quick Start

## Scan a local directory

```bash
monphare scan ./terraform
```

Example output:

```
MonPhare v0.1.1  [FAILED]  3 errors, 3 warnings
Scanned: 1 files, 4 modules, 3 providers

| Sev  | Resource              | Issue          | Current  | File      |
|------|-----------------------|----------------|----------|-----------|
| ERR  | module.vpc_no_version | No version     | -        | main.tf:0 |
| ERR  | module.git_module     | No version     | -        | main.tf:0 |
| ERR  | provider.aws          | No version     | -        | main.tf:0 |
| WARN | resource.google       | No upper bound | -        | main.tf:0 |
| WARN | resource.azurerm      | No upper bound | -        | main.tf:0 |
| WARN | provider.google       | Too broad      | >= 0.0.0 | main.tf:0 |
| INFO | resource.eks_exact    | Exact version  | -        | main.tf:0 |
```

## Scan remote repositories

```bash
monphare scan --repo https://github.com/org/repo1 --repo https://github.com/org/repo2
```

## Generate reports

JSON report:

```bash
monphare scan ./terraform --format json --output report.json
```

HTML report:

```bash
monphare scan ./terraform --format html --output report.html
```

## Create a configuration file

```bash
monphare init
```

This generates a `monphare.yaml` with default settings you can customize.

---

See the **Scanning** section for more advanced use cases.
