# MonPhare

**Catch version drift, deprecated modules, and risky constraints across all your Terraform repos.**

[![CI](https://github.com/tanguc/monphare/actions/workflows/ci.yml/badge.svg)](https://github.com/tanguc/monphare/actions/workflows/ci.yml)
[![Release](https://github.com/tanguc/monphare/actions/workflows/release.yml/badge.svg)](https://github.com/tanguc/monphare/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## The problem

Across your repos, this is happening right now:

```hcl
# team-a/main.tf -- no pin, pulls latest on every init
module "vpc" {
  source = "terraform-aws-modules/vpc/aws"
}

# team-b/main.tf -- anything goes
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = ">= 0.0.0"
}

# team-c/main.tf -- frozen, no security patches
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "= 2.44.0"
}
```

Nobody knows until something breaks. MonPhare finds these before production does.

## What MonPhare catches

| Issue | What's wrong | Example |
|-------|-------------|---------|
| **Missing version** | No constraint at all, pulls latest on every init | `source = "aws-modules/vpc/aws"` (no `version`) |
| **Too broad** | Accepts anything, including breaking changes | `version = ">= 0.0.0"` |
| **Wildcard** | Same as above, different syntax | `version = "*"` |
| **No upper bound** | Lets breaking majors in silently | `version = ">= 3.0"` |
| **Exact pin** | Frozen -- no patches, no security fixes | `version = "= 2.44.0"` |
| **Pre-release** | Alpha/beta refs in production | `version = "~> 2.0.0-beta"` |
| **Cross-repo conflict** | Two repos want incompatible ranges for the same module | repo-a: `>= 5.0` vs repo-b: `< 5.0` |
| **Deprecated module** | Module with known CVE or retired version still in use | `claranet/azure-log-mngt-v1/azurerm` at `1.0.1` |
| **Deprecated provider** | Provider version range flagged by your security team | `hashicorp/azurerm` `< 3.50.0` |
| **Deprecated runtime** | Terraform/OpenTofu version too old | `terraform < 0.13.0` |

## Quick start

### Installation

#### Homebrew (macOS / Linux)

```bash
brew tap tanguc/tap
brew install monphare
```

#### Pre-built binaries

Download from the [Releases page](https://github.com/tanguc/monphare/releases):

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | `monphare-linux-x86_64.tar.gz` |
| Linux | ARM64 | `monphare-linux-aarch64.tar.gz` |
| macOS | Intel | `monphare-darwin-x86_64.tar.gz` |
| macOS | Apple Silicon | `monphare-darwin-aarch64.tar.gz` |
| Windows | x86_64 | `monphare-windows-x86_64.zip` |

```bash
# linux/macOS
curl -LO https://github.com/tanguc/monphare/releases/latest/download/monphare-linux-x86_64.tar.gz
tar -xzf monphare-linux-x86_64.tar.gz
sudo mv monphare /usr/local/bin/

monphare --version
```

#### From source

```bash
git clone https://github.com/tanguc/monphare
cd monphare
cargo install --path .
```

### Run your first scan

```bash
monphare scan ./terraform
```

Output:

```
MonPhare v0.1.1  [FAILED]  3 errors, 3 warnings
Scanned: 1 files, 4 modules, 3 providers

+------+-----------------------+----------------+----------+-----------+
| Sev  | Resource              | Issue          | Current  | File      |
+------+-----------------------+----------------+----------+-----------+
| ERR  | module.vpc_no_version | No version     | -        | main.tf:0 |
| ERR  | module.git_module     | No version     | -        | main.tf:0 |
| ERR  | provider.aws          | No version     | -        | main.tf:0 |
| WARN | resource.google       | No upper bound | -        | main.tf:0 |
| WARN | resource.azurerm      | No upper bound | -        | main.tf:0 |
| WARN | provider.google       | Too broad      | >= 0.0.0 | main.tf:0 |
| INFO | resource.eks_exact    | Exact version  | -        | main.tf:0 |
+------+-----------------------+----------------+----------+-----------+

Fix errors to pass.
```

## Commands

### `scan` -- analyze constraints across repos

The core command. Point it at local directories, remote repos, or an entire org.

```bash
# local directories
monphare scan ./repo1 ./repo2 ./repo3

# remote repositories
monphare scan --repo https://github.com/org/repo1 --repo https://github.com/org/repo2

# output as JSON or HTML
monphare scan ./terraform --format json --output report.json
monphare scan ./terraform --format html --output report.html

# strict mode for CI -- exit code 1 on warnings
monphare scan ./terraform --strict
```

### `scan` -- at org scale

Scan all repositories from a GitHub org, GitLab group, Azure DevOps project, or Bitbucket workspace in one command.

```bash
export MONPHARE_GIT_TOKEN=ghp_xxx

# all repos in a GitHub organization
monphare scan --github my-org

# all projects in a GitLab group
monphare scan --gitlab my-group

# all repos in an Azure DevOps org or project
monphare scan --ado my-org
monphare scan --ado my-org/my-project

# all repos in a Bitbucket workspace
monphare scan --bitbucket my-workspace
```

### `graph` -- visualize dependencies

See how modules and providers relate to each other. Useful to understand blast radius before upgrading a shared module.

```bash
# DOT format (for Graphviz)
monphare graph ./terraform --format dot --output deps.dot

# Mermaid diagram (renders in GitHub, GitLab, Notion, etc.)
monphare graph ./terraform --format mermaid

# JSON for programmatic use
monphare graph ./terraform --format json

# filter to modules only or providers only
monphare graph ./terraform --modules-only
monphare graph ./terraform --providers-only
```

### `init` -- generate a starter config

Creates a `monphare.yaml` in the current directory with documented defaults.

```bash
monphare init
```

### `validate` -- check your config

Validates a config file before you wire it into CI.

```bash
monphare validate monphare.yaml
```

## Configuration

MonPhare uses a `monphare.yaml` file. Run `monphare init` to generate one with all available options.

```yaml
scan:
  exclude_patterns:
    - "**/test/**"
    - "**/examples/**"
    - "**/.terraform/**"
  continue_on_error: true

analysis:
  check_exact_versions: true
  check_prerelease: true
  check_upper_bound: true

policies:
  require_version_constraint: true
  require_upper_bound: false

# flag modules/providers with known issues
deprecations:
  modules:
    "claranet/azure-log-mngt-v1/azurerm":
      - version: "1.0.1"
        reason: "Critical security vulnerability CVE-2023-1234"
        severity: error
        replacement: "claranet/azure-log-mngt-v3/azurerm"
  providers:
    "hashicorp/azurerm":
      versions:
        - version: "> 0.0.1, < 3.50.0"
          reason: "Multiple CVEs in versions before 3.50.0"
          severity: error
          replacement: ">= 3.50.0"

cache:
  enabled: true
  ttl_hours: 24
```

Configuration priority (highest to lowest):
1. CLI arguments
2. Environment variables (`MONPHARE_GIT_TOKEN`, `MONPHARE_CONFIG`)
3. `monphare.yaml`
4. Defaults

## CI/CD integration

Use `--strict` to fail the pipeline when warnings are found.

```yaml
# GitHub Actions example
- name: Analyze Terraform constraints
  run: monphare scan ./terraform --strict --format json --output report.json
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | No issues (or warnings without `--strict`) |
| 1 | Warnings found (with `--strict`), or runtime error |
| 2 | Constraint errors found |

## Finding reference

| Code | Severity | Description |
|------|----------|-------------|
| `missing-version` | error | Module or provider has no version constraint |
| `wildcard-constraint` | warning | Constraint uses `*` wildcard |
| `broad-constraint` | warning | Constraint is too permissive (e.g. `>= 0.0.0`) |
| `no-upper-bound` | warning | No upper bound allows breaking changes in |
| `exact-version` | info | Exact pin prevents patch and security updates |
| `prerelease-version` | info | Pre-release version referenced |

## Contributing

Contributions welcome. Fork, branch, PR.

```bash
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
```

## License

MIT -- see [LICENSE](LICENSE).
