# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0](https://github.com/tanguc/MonPhare/compare/v0.3.0...v0.4.0) (2026-02-06)


### Features

* add cargo-husky pre-commit hooks and update README ([ec395aa](https://github.com/tanguc/MonPhare/commit/ec395aa9d26e15fb29330f2013937679d1ac65b5))
* add confirmation prompt for large org scans, update docs ([17c3e9a](https://github.com/tanguc/MonPhare/commit/17c3e9aea0ec59dc6e51454cf581b47574f56b38))
* add multi-arch Docker images to release workflow ([9f344b0](https://github.com/tanguc/MonPhare/commit/9f344b0ab915f49eabaadfe1a33584947e602bc7))

## [0.3.0](https://github.com/tanguc/MonPhare/compare/v0.2.0...v0.3.0) (2026-02-06)


### Features

* add lighthouse logo, favicon, and docs link in README ([ef9feb4](https://github.com/tanguc/MonPhare/commit/ef9feb446d7456f08ae61fba1ba5c83cb56a009e))
* allow scanning public repos without token and add URL auto-detect ([4f391e9](https://github.com/tanguc/MonPhare/commit/4f391e9c77329c30d147e0c88cec8577633b695d))
* regex fallback parser, tokenless public org scanning, replace claranet examples ([1ac854d](https://github.com/tanguc/MonPhare/commit/1ac854da2f87930735ca77c89876b153f7532c7f))

## [0.2.0](https://github.com/tanguc/MonPhare/compare/v0.1.1...v0.2.0) (2026-02-06)


### Features

* add documentation website, Homebrew tap, and README rewrite ([e2fd74b](https://github.com/tanguc/MonPhare/commit/e2fd74bb55ec52d44fc2be415451e4a4a424b7c6))
* **scan:** add URL detection for repository scanning ([56c11c7](https://github.com/tanguc/MonPhare/commit/56c11c76e9285da37905ff03e34b8cb3f54b65a6))


### Bug Fixes

* use correct case for baseUrl (/MonPhare/) and ignore RUSTSEC-2026-0009 ([943cc8a](https://github.com/tanguc/MonPhare/commit/943cc8abda88e490725405d8b3989a656380b43b))

## [0.1.1](https://github.com/tanguc/MonPhare/compare/v0.1.0...v0.1.1) (2026-02-05)


### Bug Fixes

* use vendored OpenSSL for cross-platform builds ([21b4d6f](https://github.com/tanguc/MonPhare/commit/21b4d6f5ed62356fd82486179ea5a5db12632aa3))

## 0.1.0 (2026-02-05)


### Features

* add repository caching and improve reporters ([2444a89](https://github.com/tanguc/MonPhare/commit/2444a89b387d8bec9b68984f223a3cf9f0728994))
* **error:** add source location tracking to errors ([a545154](https://github.com/tanguc/MonPhare/commit/a545154ed51d871ec37bf7ab8de232da57dc5111))
* initial commit ([8eb103d](https://github.com/tanguc/MonPhare/commit/8eb103d897da66c1f8093c80fe27c7a6e0269188))
* remove phase 1 & phase 2 feature ([87956d7](https://github.com/tanguc/MonPhare/commit/87956d761c74c8705072adc578b39d41ca1e1edb))


### Bug Fixes

* bump MSRV to 1.85 for edition 2024 dependency (comfy-table) ([b2edfc1](https://github.com/tanguc/MonPhare/commit/b2edfc12a4bc3a65be4eb91b47b2ba336b15d56e))
* iter ([0c1164e](https://github.com/tanguc/MonPhare/commit/0c1164e27088d5ad7f61966f917b7566b875dd4f))
* resolve CI failures with clippy, formatting, and tests ([41938d9](https://github.com/tanguc/MonPhare/commit/41938d98ab9b3b232eb0363958ad9db747d743f8))
* simplify CI test matrix and fix security audit ([eb444b0](https://github.com/tanguc/MonPhare/commit/eb444b068a355e41087add5f7a061193aaa7d645))
* update dependencies and MSRV for CI compatibility ([5e9df40](https://github.com/tanguc/MonPhare/commit/5e9df409e06a180f02fd27506f2c6203791a8109))

## [Unreleased]

### Features

- Repository caching with fresh threshold to avoid unnecessary git fetches
- Descriptive finding codes (e.g., `missing-version` instead of `DRIFT002`)
- Graceful handling of unparseable version constraints with warnings
- Redesigned text reporter with table-based output grouped by repository
- Redesigned JSON reporter with structured format
- Redesigned HTML reporter with modern dashboard UI
- Support for GitHub, GitLab, Bitbucket, and Azure DevOps
- Configurable cache settings (TTL, fresh threshold, max size)

### Bug Fixes

- Fixed resource name extraction in reports
- Fixed file path display to show relative paths within repository

## [0.1.0] - Initial Release

### Features

- Terraform/OpenTofu module constraint analysis
- Dependency graph generation (DOT, JSON, Mermaid formats)
- Version constraint conflict detection
- Deprecation tracking for modules and providers
- Multi-repository scanning via VCS APIs
- Policy-based analysis with configurable severities
- Multiple output formats (Text, JSON, HTML)
