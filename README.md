# MonPhare

*mon-far* -- French for "my lighthouse". Because someone has to shine a light on your Terraform constraints.

**Catch version drift, deprecated modules, and risky constraints across all your Terraform repos.**

[![CI](https://github.com/tanguc/monphare/actions/workflows/ci.yml/badge.svg)](https://github.com/tanguc/monphare/actions/workflows/ci.yml)
[![Release](https://github.com/tanguc/monphare/actions/workflows/release.yml/badge.svg)](https://github.com/tanguc/monphare/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[Documentation](https://tanguc.github.io/MonPhare/)

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
| **Deprecated module** | Module with known CVE or retired version still in use | `terraform-aws-modules/vpc/aws` at `1.0.1` |
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
```

#### Docker

```bash
docker pull ghcr.io/tanguc/monphare:latest

# scan a local directory
docker run --rm -v "$(pwd):/workspace" ghcr.io/tanguc/monphare scan /workspace

# scan a remote repo
docker run --rm ghcr.io/tanguc/monphare scan https://github.com/terraform-aws-modules/terraform-aws-vpc
```

Available for `linux/amd64` and `linux/arm64`.

#### From source

```bash
git clone https://github.com/tanguc/monphare
cd monphare
cargo install --path .
```

### Run your first scan

```bash
# scan a local directory
monphare scan ./terraform

# scan a remote repo directly (public repos work without a token)
monphare scan https://github.com/terraform-aws-modules/terraform-aws-vpc
```

Output:

```
MonPhare v0.3.0  [FAILED]  3 errors, 3 warnings
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

### `scan` -- analyze constraints

Point it at local directories, remote URLs, or an entire org.

```bash
# local directories
monphare scan ./repo1 ./repo2 ./repo3

# remote repositories (URL auto-detected, no --repo flag needed)
monphare scan https://github.com/org/repo1 https://github.com/org/repo2

# mix local and remote
monphare scan ./local-repo https://github.com/org/remote-repo

# output as JSON or HTML
monphare scan ./terraform --format json --output report.json
monphare scan ./terraform --format html --output report.html

# strict mode for CI -- exit code 1 on warnings
monphare scan ./terraform --strict
```

### `scan` -- at org scale

Scan all repositories from a GitHub org, GitLab group, Azure DevOps project, or Bitbucket workspace. Works without a token for public orgs.

```bash
# public org -- no token needed
monphare scan --github terraform-aws-modules

# private org -- set token via env var
export MONPHARE_GIT_TOKEN=ghp_xxx
monphare scan --github my-private-org

# other platforms
monphare scan --gitlab my-group
monphare scan --ado my-org/my-project
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
    "terraform-aws-modules/vpc/aws":
      - version: "1.0.1"
        reason: "Critical security vulnerability in VPC module versions before 3.0"
        severity: error
        replacement: "terraform-aws-modules/vpc/aws >= 5.0.0"
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
