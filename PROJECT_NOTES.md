# Project Notes & Decisions

## Session: 2026-01-31

### Project Renaming Decision

**Decision:** Rename project from `monphare` to **`monphare`**

**Rationale:**
- "Mon Infra Phare" = "My Infrastructure Lighthouse" (French + English)
- Personal touch: YOUR guardian watching over infrastructure
- Expandable beyond Terraform to all IaC tools (Ansible, Pulumi, Dagger, etc.)
- Unique, memorable branding
- Short CLI alias: `mip`

**Rejected alternatives:**
- TerraVigie, Phare Guard, Cloud Phare (too Terraform-specific)
- Infra Phare (good but less personal)
- Policy Beacon (too generic)

### Product Positioning

**Core Identity:** Policy Compliance & Audit Tool for Infrastructure Code

**NOT:**
- A dependency updater (that's Renovate/Dependabot)
- A version bumper (that's tfupdate)

**IS:**
- Internal module deprecation tracker
- Multi-repo Terraform audit tool
- Policy compliance scanner
- Organization-wide visibility tool

**Key differentiators from Renovate/Dependabot:**
1. Custom internal module deprecation rules (Azure DevOps, private Git)
2. Organization-wide compliance audits (not per-repo PRs)
3. Policy enforcement reporting (missing constraints, risky patterns)
4. Cross-repo dependency visualization

### Architecture Changes

#### Removed: Conflict Detection Feature

**Reason:** Not valuable for real-world ops workflows
- Within-repo conflicts: Terraform init already catches these
- Cross-repo conflicts: Better served by deprecation rules
- Created noise without actionable insights

**What was removed:**
- `detect_module_conflicts()` and `detect_provider_conflicts()` methods
- `FindingCategory::ConstraintConflict` enum variant
- Conflict detection finding code
- All conflict detection tests

**New analysis pipeline:**
1. Phase 1: Missing constraints (missing-version)
2. Phase 2: Risky patterns (wildcard-constraint, prerelease-version, exact-version, no-upper-bound)
3. Phase 3: Broad constraints (broad-constraint)
4. Phase 4: Deprecations (user-defined rules)

### CLI Design: Inline Deprecation Rules

**Problem:** Users don't want to write config files for quick checks

**Solution:** Inline CLI deprecation flags

```bash
# Define deprecations directly via CLI
monphare scan ./terraform \
  --deprecate-module ssh.dev.azure.com/v3/org/Terraform/mod-azurerm-search \
  --deprecate-tag 3.2.0 \
  --deprecate-tag 3.1.0 \
  --deprecate-branch 3.0.42 \
  --deprecate-reason "Security vulnerability CVE-2024-1234" \
  --deprecate-replacement "Use tag 3.2.1+"

# Multiple modules in one command
monphare scan ./terraform \
  --deprecate-module ssh.dev.azure.com/.../module-a \
  --deprecate-tag 3.0.0 \
  --deprecate-module terraform-aws-modules/vpc/aws \
  --deprecate-version "< 5.0.0"

# Provider deprecation
monphare scan ./terraform \
  --deprecate-provider hashicorp/azurerm \
  --deprecate-version "< 3.0.0"

# Runtime deprecation
monphare scan ./terraform \
  --deprecate-runtime terraform \
  --deprecate-version "< 1.0.0"
```

**Grouping logic:**
- Each `--deprecate-module/provider/runtime` starts a new rule
- Subsequent flags apply to the last started rule
- Rules merge with config file rules (if present)

**Status:** NOT YET IMPLEMENTED (planned feature)

### Future Expansion

**Vision:** Multi-IaC support

**Roadmap:**
- v0.1: Terraform/OpenTofu (current)
- v0.2: Ansible role/collection deprecation
- v0.3: Pulumi package tracking
- v0.4: Dagger module support
- v0.5: Helm chart deprecation

**Universal concepts:**
- Modules = Terraform modules, Ansible roles, Pulumi packages, etc.
- Deprecation tracking across all IaC ecosystems
- Policy compliance (version pinning, patterns)

### Technical Decisions

#### Tracing in Tests

**Problem:** `tracing::debug!()` logs not appearing in tests

**Solution:** Use `tracing-test` crate (recommended)
```toml
[dev-dependencies]
tracing-test = "0.2"
```

```rust
#[test]
#[traced_test]
fn test_something() {
    // tracing::debug! logs automatically appear
}
```

**Alternatives considered:**
- Manual `tracing_subscriber::fmt().with_test_writer().try_init()` (overkill)
- `test-log` crate (good alternative)
- Just use `RUST_LOG=debug cargo test -- --nocapture` (temporary debugging only)

### Key Insights from Session

1. **Conflict detection is noise:** Independent projects using different versions is drift, not conflict. Only matters within same Terraform state.

2. **Deprecation > Conflict detection:** Better to say "upgrade from deprecated v4.0" than "v4.0 conflicts with v5.0 somewhere else"

3. **Internal modules are the use case:** Public modules already covered by Renovate. Value is in custom Azure DevOps/private Git module deprecation.

4. **French branding works:** Unique identity in crowded IaC space. "Mon-Infra-Phare" memorable and personal.

5. **Policy tool, not updater:** Audit/compliance/visibility, not automation. Different problem space than Renovate.

### Next Steps

**Immediate (v0.1):**
- [ ] Rename project from `monphare` to `monphare`
- [ ] Implement inline `--deprecate-*` CLI flags
- [ ] Add `--only-deprecations` filter flag
- [ ] Update all docs/branding

**Short-term (v0.2):**
- [ ] Built-in deprecation rules for common modules (optional)
- [ ] Improved deprecation reporting (show upgrade paths)
- [ ] Export deprecation data to CSV/JSON for dashboards

**Long-term (v0.3+):**
- [ ] Ansible support
- [ ] Pulumi support
- [ ] GitHub App for automated org scanning
- [ ] Web UI for deprecation rule management

### Open Questions

1. ~~Should we keep "drift" in finding codes (DRIFT002-007) or rename to something more policy-focused?~~ **RESOLVED:** Renamed to descriptive kebab-case codes like Clippy (missing-version, wildcard-constraint, etc.)

2. Built-in deprecation rules vs always user-defined?
   - Option A: Ship with common module deprecations (terraform-aws-modules, etc.)
   - Option B: Always require user configuration
   - **Leaning toward:** Option A with opt-in flag `--builtin-rules`

3. Should deprecation rules support wildcards/patterns?
   ```bash
   --deprecate-module "internal/*" --deprecate-version "< 2.0"
   ```

4. How to handle monorepo vs multi-repo scanning differently?

---

## Contact / Context

- User is French, prefers French-inspired naming
- Works with Azure DevOps Git SSH modules extensively
- Main use case: Internal module deprecation tracking across organization
- Team: Ops managing multiple Terraform repos across departments
