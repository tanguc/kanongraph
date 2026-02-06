import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import CodeBlock from '@theme/CodeBlock';

const badTerraform = `# team-a: no version pin at all
module "vpc" {
  source = "terraform-aws-modules/vpc/aws"
}

# team-b: accepts literally anything
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = ">= 0.0.0"
}

# team-c: frozen since 2022, no security patches
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "= 2.44.0"
}`;

const scanOutput = `$ monphare scan --github my-org --yes --format text

MonPhare v0.1.1  [FAILED]  6 errors, 8 warnings
Scanned: 47 files, 31 modules, 12 providers across 15 repositories

+------+-----------------------------+-----------------+----------+-------------------+
| Sev  | Resource                    | Issue           | Current  | File              |
+------+-----------------------------+-----------------+----------+-------------------+
| ERR  | module.vpc (team-infra)     | No version      | -        | main.tf:12        |
| ERR  | module.rds (team-backend)   | No version      | -        | database.tf:3     |
| ERR  | module.lambda (team-api)    | No version      | -        | functions.tf:8    |
| ERR  | provider.aws (team-data)    | No version      | -        | providers.tf:1    |
| ERR  | module.s3 (team-platform)   | No version      | -        | storage.tf:15     |
| ERR  | module.iam (team-security)  | No version      | -        | roles.tf:22       |
| WARN | module.eks (team-platform)  | No upper bound  | >= 19.0  | cluster.tf:5      |
| WARN | module.cdn (team-frontend)  | No upper bound  | >= 3.0   | cdn.tf:1          |
| WARN | provider.google (team-ml)   | Too broad       | >= 0.0.0 | providers.tf:8    |
| WARN | module.vpc (team-staging)   | No upper bound  | >= 5.0   | network.tf:3      |
| WARN | provider.azurerm (team-ops) | No upper bound  | >= 3.0   | providers.tf:1    |
| WARN | module.cache (team-backend) | Wildcard        | *        | cache.tf:1        |
| WARN | module.queue (team-api)     | No upper bound  | >= 12.0  | messaging.tf:10   |
| WARN | module.dns (team-platform)  | No upper bound  | >= 2.0   | dns.tf:5          |
| INFO | module.vault (team-sec)     | Exact version   | = 3.8.2  | vault.tf:1        |
| INFO | module.k8s (team-platform)  | Pre-release     | 2.0-rc1  | k8s.tf:12         |
+------+-----------------------------+-----------------+----------+-------------------+

Fix errors to pass.`;

const configExample = `# monphare.yaml -- define what your org considers deprecated
deprecations:
  modules:
    "claranet/azure-log-mngt-v1/azurerm":
      - version: "< 3.0.0"
        reason: "CVE-2024-5678 -- critical auth bypass"
        severity: error
        replacement: "claranet/azure-log-mngt-v3/azurerm"

  providers:
    "hashicorp/azurerm":
      versions:
        - version: "< 3.50.0"
          reason: "Multiple CVEs in versions before 3.50.0"
          severity: error
          replacement: ">= 3.50.0"

  runtime:
    terraform:
      - version: "< 1.5.0"
        reason: "End of life, no security patches"
        severity: warning`;

const graphOutput = `$ monphare graph ./infrastructure --format mermaid

graph TD
    vpc["vpc ~> 5.0"]
    eks["eks ~> 20.0"]
    rds["rds ~> 6.0"]
    lambda["lambda ~> 7.0"]
    aws(("hashicorp/aws >= 5.0, < 6.0"))
    random(("hashicorp/random ~> 3.0"))

    vpc -.-> aws
    eks -.-> aws
    rds -.-> aws
    lambda -.-> aws
    eks --> vpc`;

const ciExample = `# .github/workflows/terraform-audit.yml
name: Weekly Terraform Audit

on:
  schedule:
    - cron: '0 8 * * 1'   # every Monday 8am

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - name: Run org-wide scan
        env:
          MONPHARE_GIT_TOKEN: \${{ secrets.GITHUB_TOKEN }}
        run: |
          monphare scan \\
            --github my-org \\
            --yes \\
            --strict \\
            --format json \\
            --output audit.json`;

const platforms = [
  { name: 'GitHub', flag: '--github my-org' },
  { name: 'GitLab', flag: '--gitlab my-group' },
  { name: 'Azure DevOps', flag: '--ado my-org/my-project' },
  { name: 'Bitbucket', flag: '--bitbucket my-workspace' },
];

function HeroSection() {
  return (
    <header className="landing-hero">
      <div className="container">
        <div className="landing-hero__content">
          <div className="landing-hero__text">
            <h1 className="landing-hero__title">
              Know what's pinned across<br />
              <span className="landing-hero__highlight">all your Terraform repos</span>
            </h1>
            <p className="landing-hero__subtitle">
              MonPhare scans your organization's Terraform repositories, finds missing version
              pins, deprecated modules, risky constraints, and cross-repo conflicts -- before
              they break production.
            </p>
            <div className="landing-hero__buttons">
              <Link className="button button--primary button--lg" to="/docs/getting-started/installation">
                Get Started
              </Link>
              <Link className="button button--outline button--lg" to="/docs/scanning/org-scale">
                See Org Scanning
              </Link>
            </div>
          </div>
          <div className="landing-hero__terminal">
            <div className="terminal">
              <div className="terminal__header">
                <span className="terminal__dot terminal__dot--red" />
                <span className="terminal__dot terminal__dot--yellow" />
                <span className="terminal__dot terminal__dot--green" />
                <span className="terminal__title">terminal</span>
              </div>
              <div className="terminal__body">
                <code>
                  <span className="terminal__prompt">$</span> monphare scan --github my-org{'\n'}
                  {'\n'}
                  <span className="terminal__muted">Scanning 15 repositories...</span>{'\n'}
                  {'\n'}
                  <span className="terminal__red">ERR</span>  module.vpc         No version      main.tf:12{'\n'}
                  <span className="terminal__red">ERR</span>  module.rds         No version      database.tf:3{'\n'}
                  <span className="terminal__yellow">WARN</span> module.eks         No upper bound  cluster.tf:5{'\n'}
                  <span className="terminal__yellow">WARN</span> provider.google    Too broad       providers.tf:8{'\n'}
                  <span className="terminal__yellow">WARN</span> module.cache       Wildcard        cache.tf:1{'\n'}
                  <span className="terminal__blue">INFO</span> module.vault       Exact version   vault.tf:1{'\n'}
                  {'\n'}
                  <span className="terminal__red">6 errors</span>, <span className="terminal__yellow">8 warnings</span> across <span className="terminal__white">15 repos</span>
                </code>
              </div>
            </div>
          </div>
        </div>
      </div>
    </header>
  );
}

function ProblemSection() {
  return (
    <section className="landing-section landing-section--alt">
      <div className="container">
        <div className="landing-section__header">
          <h2>This is happening across your repos right now</h2>
          <p>Three teams, same module, three different constraint strategies. Nobody knows until something breaks.</p>
        </div>
        <CodeBlock language="hcl" title="What your Terraform repos actually look like">{badTerraform}</CodeBlock>
      </div>
    </section>
  );
}

function ScanDemoSection() {
  return (
    <section className="landing-section">
      <div className="container">
        <div className="landing-section__header">
          <h2>One command to scan your entire org</h2>
          <p>Point MonPhare at a GitHub org, GitLab group, Azure DevOps project, or Bitbucket workspace. Get a full constraint audit in seconds.</p>
        </div>
        <CodeBlock language="bash" title="Scan all repos in your organization">{scanOutput}</CodeBlock>
      </div>
    </section>
  );
}

function PlatformSection() {
  return (
    <section className="landing-section landing-section--alt">
      <div className="container">
        <div className="landing-section__header">
          <h2>Works with your Git platform</h2>
          <p>One token, one command. MonPhare clones, scans, and reports across all your repositories.</p>
        </div>
        <div className="platform-grid">
          {platforms.map((p, i) => (
            <div className="platform-card" key={i}>
              <h3>{p.name}</h3>
              <code className="platform-card__cmd">monphare scan {p.flag}</code>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function DeprecationSection() {
  return (
    <section className="landing-section">
      <div className="container">
        <div className="landing-section__split">
          <div className="landing-section__split-text">
            <h2>Define your own deprecation rules</h2>
            <p>
              Flag modules with known CVEs, providers below a certain version, or
              Terraform runtimes past end-of-life. Your security team defines the
              rules, MonPhare enforces them across every repo.
            </p>
            <ul className="landing-checklist">
              <li>Module deprecations with CVE references</li>
              <li>Provider version range bans</li>
              <li>Terraform/OpenTofu runtime version requirements</li>
              <li>Custom severity per rule (error, warning, info)</li>
              <li>Suggested replacements in scan output</li>
            </ul>
          </div>
          <div className="landing-section__split-code">
            <CodeBlock language="yaml" title="monphare.yaml">{configExample}</CodeBlock>
          </div>
        </div>
      </div>
    </section>
  );
}

function GraphSection() {
  return (
    <section className="landing-section landing-section--alt">
      <div className="container">
        <div className="landing-section__header">
          <h2>Visualize your dependency map</h2>
          <p>See which modules depend on which providers. Understand blast radius before upgrading a shared module.</p>
        </div>
        <CodeBlock language="bash" title="Export dependency graph">{graphOutput}</CodeBlock>
        <div className="landing-formats">
          <div className="landing-format-tag">DOT (Graphviz)</div>
          <div className="landing-format-tag">Mermaid (GitHub / GitLab)</div>
          <div className="landing-format-tag">JSON (programmatic)</div>
        </div>
      </div>
    </section>
  );
}

function CISection() {
  return (
    <section className="landing-section">
      <div className="container">
        <div className="landing-section__split landing-section__split--reverse">
          <div className="landing-section__split-text">
            <h2>Drop it into your CI pipeline</h2>
            <p>
              Use <code>--strict</code> to fail builds on warnings. Schedule weekly org-wide audits.
              Pipe JSON output to dashboards or Slack.
            </p>
            <div className="landing-exit-codes">
              <div className="landing-exit-code">
                <span className="landing-exit-code__num landing-exit-code--ok">0</span>
                <span>Clean -- no issues</span>
              </div>
              <div className="landing-exit-code">
                <span className="landing-exit-code__num landing-exit-code--warn">1</span>
                <span>Warnings (with --strict)</span>
              </div>
              <div className="landing-exit-code">
                <span className="landing-exit-code__num landing-exit-code--err">2</span>
                <span>Errors found</span>
              </div>
            </div>
          </div>
          <div className="landing-section__split-code">
            <CodeBlock language="yaml" title=".github/workflows/terraform-audit.yml">{ciExample}</CodeBlock>
          </div>
        </div>
      </div>
    </section>
  );
}

function CTASection() {
  return (
    <section className="landing-cta">
      <div className="container">
        <h2>Find what's wrong before production does</h2>
        <p>Install in 30 seconds. Scan your first repo in 60.</p>
        <div className="landing-hero__buttons">
          <Link className="button button--primary button--lg" to="/docs/getting-started/installation">
            Get Started
          </Link>
          <Link className="button button--outline button--lg" href="https://github.com/tanguc/monphare">
            View on GitHub
          </Link>
        </div>
      </div>
    </section>
  );
}

export default function Home() {
  const { siteConfig } = useDocusaurusContext();

  return (
    <Layout title={siteConfig.title} description={siteConfig.tagline}>
      <HeroSection />
      <main>
        <ProblemSection />
        <ScanDemoSection />
        <PlatformSection />
        <DeprecationSection />
        <GraphSection />
        <CISection />
        <CTASection />
      </main>
    </Layout>
  );
}
