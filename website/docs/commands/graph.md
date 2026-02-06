---
sidebar_position: 2
title: graph
---

# graph

Generate a dependency graph visualization from Terraform/OpenTofu files. Shows relationships between modules and providers.

**Alias:** `g`

## Synopsis

```
monphare graph [OPTIONS] <PATH>...
```

## Options

| Short | Long | Description | Default |
|-------|------|-------------|---------|
| | `<PATH>...` | One or more directories to scan (required). | |
| `-f` | `--format <FORMAT>` | Output format: `dot`, `json`, or `mermaid`. | `dot` |
| `-o` | `--output <FILE>` | Write graph to a file instead of stdout. | |
| | `--modules-only` | Include only module nodes, exclude providers. | `false` |
| | `--providers-only` | Include only provider nodes, exclude modules. | `false` |
| | `--filter <FILTER>` | Filter to specific module sources by partial match. | |

## Output Formats

### DOT (Graphviz)

Default format. Produces a Graphviz DOT file that can be rendered to PNG, SVG, or PDF using the `dot` command.

```bash
monphare graph ./infra --format dot --output deps.dot
dot -Tpng deps.dot -o deps.png
```

### Mermaid

Produces a Mermaid diagram that renders natively in GitHub and GitLab markdown files. Paste the output into a fenced code block with the `mermaid` language tag.

```bash
monphare graph ./infra --format mermaid
```

### JSON

Produces a JSON structure with `nodes` and `edges` arrays. Useful for programmatic analysis, custom visualizations, or feeding into other tools.

```bash
monphare graph ./infra --format json | jq '.nodes | length'
```

## Examples

Generate a DOT graph and render it:

```bash
monphare graph ./infra -o deps.dot
dot -Tsvg deps.dot -o deps.svg
```

Show only module relationships:

```bash
monphare graph ./infra --modules-only
```

Filter to a specific module source:

```bash
monphare graph ./infra --filter "terraform-aws-modules/vpc"
```

Generate a Mermaid diagram for a README:

```bash
monphare graph ./infra --format mermaid > graph.md
```

Export JSON for scripting:

```bash
monphare graph ./infra --format json --output graph.json
```
