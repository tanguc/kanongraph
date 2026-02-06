---
sidebar_position: 1
title: Installation
---

# Installation

## Homebrew (macOS / Linux)

```bash
brew tap tanguc/tap
brew install monphare
```

## Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/tanguc/monphare/releases).

| Platform            | Target                        |
|---------------------|-------------------------------|
| Linux x86_64        | `x86_64-unknown-linux-gnu`    |
| Linux ARM64         | `aarch64-unknown-linux-gnu`   |
| macOS Intel         | `x86_64-apple-darwin`         |
| macOS Apple Silicon | `aarch64-apple-darwin`        |
| Windows x86_64      | `x86_64-pc-windows-msvc`     |

### Quick install (Linux / macOS)

```bash
curl -sL https://github.com/tanguc/monphare/releases/latest/download/monphare-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').tar.gz | tar xz
sudo mv monphare /usr/local/bin/
```

## From source

Requires Rust 1.85 or later.

```bash
git clone https://github.com/tanguc/monphare.git
cd monphare
cargo install --path .
```

Verify the installation:

```bash
monphare --version
```
