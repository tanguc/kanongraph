use std::collections::{HashMap, HashSet};

use crate::types::{DeprecationResult, ModuleSource, RuntimeRef};
use crate::{Config, Constraint, ModuleRef, ProviderRef};

const DEFAULT_TERRAFORM_REGISTRY: &str = "registry.terraform.io";

pub struct DeprecationAnalyzer {
    config: Config,
}

impl DeprecationAnalyzer {
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    #[must_use]
    pub fn analyze(
        &self,
        modules: &[ModuleRef],
        providers: &[ProviderRef],
        runtimes: &[RuntimeRef],
    ) -> DeprecationResult {
        tracing::debug!(
            modules = modules.len(),
            providers = providers.len(),
            runtimes = runtimes.len(),
            "Starting deprecation analysis"
        );
        let deprecated_runtimes = self.check_runtime_deprecations(runtimes);
        tracing::debug!(
            deprecated_runtimes = deprecated_runtimes.len(),
            "Runtime deprecation check complete"
        );
        let deprecated_modules = self.check_module_deprecations(modules);
        tracing::debug!(
            deprecated_modules = deprecated_modules.len(),
            "Module deprecation check complete"
        );
        let deprecated_providers = self.check_provider_deprecations(providers);
        tracing::debug!(
            deprecated_providers = deprecated_providers.len(),
            "Provider deprecation check complete"
        );

        tracing::debug!(
            total_deprecated =
                deprecated_runtimes.len() + deprecated_modules.len() + deprecated_providers.len(),
            "Deprecation analysis complete"
        );

        DeprecationResult {
            runtimes: deprecated_runtimes,
            modules: deprecated_modules,
            providers: deprecated_providers,
            unique_module_sources: HashSet::new(),
            unique_provider_sources: HashSet::new(),
            unique_runtime_sources: HashSet::new(),
        }
    }

    fn check_runtime_deprecations(&self, runtimes: &[RuntimeRef]) -> Vec<RuntimeRef> {
        tracing::debug!(
            runtimes = runtimes.len(),
            deprecation_rules = self.config.deprecations.runtime.len(),
            "Checking runtime deprecations"
        );
        let mut parsed_rules: HashMap<&str, Vec<Constraint>> = HashMap::new();
        for (name, rules) in &self.config.deprecations.runtime {
            let mut constraints = Vec::new();
            for rule in rules {
                if let Some(v) = &rule.version {
                    if let Ok(c) = Constraint::parse(v) {
                        constraints.push(c);
                    } else {
                        tracing::debug!(
                            runtime_name = %name,
                            version = %v,
                            "Failed to parse deprecation constraint"
                        );
                    }
                }
            }
            tracing::debug!(
                runtime_name = %name,
                constraints_count = constraints.len(),
                "Parsed deprecation rules for runtime"
            );
            parsed_rules.insert(name.as_str(), constraints);
        }

        let mut findings = Vec::new();
        for runtime in runtimes {
            let Some(deprecations) = parsed_rules.get(runtime.name.as_str()) else {
                tracing::debug!(
                    runtime_name = %runtime.name,
                    "No deprecation rules found for runtime"
                );
                continue;
            };
            tracing::debug!(
                runtime_name = %runtime.name,
                version = %runtime.version.raw,
                deprecation_rules_count = deprecations.len(),
                "Checking runtime against deprecation rules"
            );
            for deprecated_constraint in deprecations {
                if runtime.version.has_overlap_with(deprecated_constraint) {
                    tracing::debug!(
                        runtime_name = %runtime.name,
                        version = %runtime.version.raw,
                        deprecated_constraint = %deprecated_constraint.raw,
                        "Runtime matches deprecated version"
                    );
                    findings.push(runtime.clone());
                    break;
                }
            }
        }
        tracing::debug!(
            findings = findings.len(),
            "Runtime deprecation check complete"
        );
        findings
    }

    fn check_module_deprecations(&self, modules: &[ModuleRef]) -> Vec<ModuleRef> {
        let mut findings = Vec::new();
        for module in modules {
            if module.source.is_local() {
                tracing::trace!(module = %module.name, "Local module, skipping deprecation check");
                continue;
            }
            let mut deprecated = false;
            for key in module_deprecation_keys(&module.source) {
                tracing::debug!(
                    module_name = %module.name,
                    key = %key,
                    "Checking module source for deprecations"
                );
                let Some(rules) = self.config.deprecations.modules.get(&key) else {
                    tracing::debug!(
                        module_name = %module.name,
                        key = %key,
                        "No deprecation rules found for module key"
                    );
                    continue;
                };
                tracing::debug!(
                    module_name = %module.name,
                    key = %key,
                    rules_count = rules.len(),
                    "Found deprecation rules for module"
                );

                for rule in rules {
                    // Registry-style semver rules
                    if let Some(v) = &rule.version {
                        if let (Some(module_constraint), Ok(deprecated_constraint)) =
                            (module.version_constraint.as_ref(), Constraint::parse(v))
                        {
                            tracing::debug!(
                                module_name = %module.name,
                                module_constraint = %module_constraint.raw,
                                deprecated_constraint = %deprecated_constraint.raw,
                                "Comparing module constraint with deprecation rule"
                            );
                            if module_constraint.has_overlap_with(&deprecated_constraint) {
                                tracing::debug!(
                                    module_name = %module.name,
                                    "Module matches deprecated version constraint"
                                );
                                deprecated = true;
                                break;
                            }
                        } else {
                            tracing::debug!(
                                module_name = %module.name,
                                version = %v,
                                "Failed to parse or compare module deprecation constraint"
                            );
                        }
                    }

                    // Git ref rules
                    if let Some(rule_ref) = &rule.git_ref {
                        if let ModuleSource::Git {
                            ref_: Some(actual_ref),
                            ..
                        } = &module.source
                        {
                            tracing::debug!(
                                module_name = %module.name,
                                actual_ref = %actual_ref,
                                rule_ref = %rule_ref,
                                "Comparing Git ref with deprecation rule"
                            );
                            if actual_ref == rule_ref {
                                tracing::debug!(
                                    module_name = %module.name,
                                    "Module Git ref matches deprecated ref"
                                );
                                deprecated = true;
                                break;
                            }
                        }
                    }
                }

                if deprecated {
                    break;
                }
            }

            if deprecated {
                findings.push(module.clone());
            }
        }
        findings
    }

    fn check_provider_deprecations(&self, providers: &[ProviderRef]) -> Vec<ProviderRef> {
        tracing::debug!(
            providers = providers.len(),
            deprecation_rules = self.config.deprecations.providers.len(),
            "Checking provider deprecations"
        );
        let mut findings = Vec::new();
        for provider in providers {
            let key = provider.qualified_source();
            let Some(rules) = self.config.deprecations.providers.get(&key) else {
                tracing::debug!(
                    provider_name = %provider.name,
                    key = %key,
                    "No deprecation rules found for provider"
                );
                continue;
            };

            tracing::debug!(
                provider_name = %provider.name,
                key = %key,
                rules_count = rules.len(),
                "Checking provider against deprecation rules"
            );
            let mut deprecated = false;
            for rule in rules {
                if let Some(v) = &rule.version {
                    if let (Some(provider_constraint), Ok(deprecated_constraint)) =
                        (provider.version_constraint.as_ref(), Constraint::parse(v))
                    {
                        if provider_constraint.has_overlap_with(&deprecated_constraint) {
                            tracing::debug!(
                                provider_name = %provider.name,
                                constraint = %provider_constraint.raw,
                                deprecated_constraint = %deprecated_constraint.raw,
                                "Provider matches deprecated version"
                            );
                            deprecated = true;
                            break;
                        }
                    } else {
                        tracing::debug!(
                            provider_name = %provider.name,
                            version = %v,
                            "Failed to parse or compare deprecation constraint"
                        );
                    }
                }
            }

            if deprecated {
                findings.push(provider.clone());
            }
        }
        tracing::debug!(
            findings = findings.len(),
            "Provider deprecation check complete"
        );
        findings
    }
}

fn module_deprecation_keys(source: &ModuleSource) -> Vec<String> {
    tracing::debug!("looking for module deprecations for source: {:#?}", source);
    match source {
        ModuleSource::Git {
            host, url, subdir, ..
        } => {
            let mut keys: Vec<String> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();

            let mut push_key = |k: String| {
                if seen.insert(k.clone()) {
                    keys.push(k);
                }
            };

            // Preferred canonical form: `host/path` (no scheme, no `.git`)
            push_key(host.clone());

            // Backward-compatible forms users often have in configs:
            // - raw URL (ssh://..., https://...)
            // - Terraform `git::` URL
            push_key(url.clone());
            push_key(format!("git::{url}"));

            // Backward-compatible SCP style: `git@host:path`
            if let Ok(parsed) = url::Url::parse(url) {
                if parsed.scheme() == "ssh" {
                    if let Some(h) = parsed.host_str() {
                        let path = parsed.path().trim_start_matches('/');
                        if !path.is_empty() {
                            let user = parsed.username();
                            let user = if user.is_empty() { "git" } else { user };
                            push_key(format!("{user}@{h}:{path}"));
                        }
                    }
                }
            }

            // Apply optional subdir (`//...`) to all base keys
            let subdir_suffix = subdir
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("//{s}"));

            if let Some(suffix) = subdir_suffix {
                keys.into_iter().map(|k| format!("{k}{suffix}")).collect()
            } else {
                keys
            }
        }
        ModuleSource::Registry {
            hostname,
            namespace,
            name,
            provider,
        } => {
            // Accept both:
            // - "registry.terraform.io/namespace/name/provider" (canonical_id)
            // - "namespace/name/provider" (common shorthand)
            let full = source.canonical_id();
            if hostname == DEFAULT_TERRAFORM_REGISTRY {
                vec![full, format!("{namespace}/{name}/{provider}")]
            } else {
                vec![full]
            }
        }
        _ => vec![source.canonical_id()],
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::DeprecationRef;
    use crate::types::RuntimeSource;

    use super::*;

    #[test]
    fn test_check_runtime_deprecations() {
        let mut config = Config::default();

        config.deprecations.runtime.insert(
            "terraform".to_string(),
            vec![
                DeprecationRef {
                    version: Some("< 0.13.0".to_string()),
                    git_ref: None,
                    reason: "Terraform < 0.13.0 is deprecated".to_string(),
                    severity: "error".to_string(),
                    replacement: ">= 0.13.0".to_string(),
                },
                DeprecationRef {
                    version: Some(">= 0.13.0, < 0.14.0".to_string()),
                    git_ref: None,
                    reason: "Terraform >= 0.13.0, < 0.14.0 is deprecated".to_string(),
                    severity: "error".to_string(),
                    replacement: ">= 0.14.0".to_string(),
                },
            ],
        );
        let analyzer = DeprecationAnalyzer::new(&config);

        let runtimes = vec![
            RuntimeRef {
                name: "terraform".to_string(),
                version: Constraint::parse("0.13.0").unwrap(),
                source: RuntimeSource::Terraform,
                file_path: PathBuf::from("terraform.tf"),
                line_number: 1,
                repository: Some("terraform".to_string()),
            },
            RuntimeRef {
                name: "terraform".to_string(),
                version: Constraint::parse("0.14.0").unwrap(),
                source: RuntimeSource::Terraform,
                file_path: PathBuf::from("terraform.tf"),
                line_number: 1,
                repository: Some("terraform".to_string()),
            },
            RuntimeRef {
                name: "terraform".to_string(),
                version: Constraint::parse("0.15.0").unwrap(),
                source: RuntimeSource::Terraform,
                file_path: PathBuf::from("terraform.tf"),
                line_number: 1,
                repository: Some("terraform".to_string()),
            },
            RuntimeRef {
                name: "terraform".to_string(),
                version: Constraint::parse("0.13.42").unwrap(),
                source: RuntimeSource::Terraform,
                file_path: PathBuf::from("terraform.tf"),
                line_number: 1,
                repository: Some("terraform".to_string()),
            },
        ];

        let result = analyzer.analyze(&[], &[], &runtimes);
        assert_eq!(result.runtimes.len(), 2);
        assert_eq!(
            result.runtimes,
            vec![runtimes[0].clone(), runtimes[3].clone()]
        );
    }

    #[test]
    fn test_check_modules_deprecations() {
        let mut config = Config::default();

        // Registry module rules (realistic terraform-aws-modules names)
        config.deprecations.modules = HashMap::new();
        config.deprecations.modules.insert(
            "terraform-aws-modules/vpc/aws".to_string(),
            vec![DeprecationRef {
                version: Some("< 5.0.0".to_string()),
                git_ref: None,
                reason: "terraform-aws-vpc < 5.0.0 is deprecated".to_string(),
                severity: "error".to_string(),
                replacement: ">= 5.0.0".to_string(),
            }],
        );
        config.deprecations.modules.insert(
            "terraform-aws-modules/eks/aws".to_string(),
            vec![DeprecationRef {
                version: Some("< 20.0.0".to_string()),
                git_ref: None,
                reason: "terraform-aws-eks < 20.0.0 is deprecated".to_string(),
                severity: "error".to_string(),
                replacement: ">= 20.0.0".to_string(),
            }],
        );

        // Git module rule (Azure DevOps SSH normalized URL + subdir; match by git_ref)
        config.deprecations.modules.insert(
            "ssh.dev.azure.com/v3/foo-bar/Terraform/mod-azurerm-resource-group".to_string(),
            vec![DeprecationRef {
                version: None,
                git_ref: Some("refs/tags/3.0.0".to_string()),
                reason: "Tag 3.0.0 is deprecated".to_string(),
                severity: "error".to_string(),
                replacement: "refs/tags/3.0.1".to_string(),
            }],
        );

        let analyzer = DeprecationAnalyzer::new(&config);

        let modules = vec![
            // Deprecated VPC (4.x)
            ModuleRef {
                name: "vpc".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "vpc".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("4.2.0").unwrap()),
                file_path: PathBuf::from("network/main.tf"),
                line_number: 10,
                repository: Some("parent".to_string()),
                attributes: Default::default(),
            },
            // Non-deprecated VPC (5.x)
            ModuleRef {
                name: "vpc".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "vpc".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("5.1.0").unwrap()),
                file_path: PathBuf::from("network/main.tf"),
                line_number: 40,
                repository: Some("child-a".to_string()),
                attributes: Default::default(),
            },
            // Deprecated EKS (19.x)
            ModuleRef {
                name: "eks".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "eks".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("19.15.3").unwrap()),
                file_path: PathBuf::from("cluster/main.tf"),
                line_number: 12,
                repository: Some("child-b".to_string()),
                attributes: Default::default(),
            },
            // Non-deprecated EKS (20.x)
            ModuleRef {
                name: "eks".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "eks".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("20.1.0").unwrap()),
                file_path: PathBuf::from("cluster/main.tf"),
                line_number: 55,
                repository: Some("parent".to_string()),
                attributes: Default::default(),
            },
            // Other terraform-aws-modules (no rules => non-deprecated)
            ModuleRef {
                name: "s3_bucket".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "s3-bucket".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("4.0.0").unwrap()),
                file_path: PathBuf::from("storage/main.tf"),
                line_number: 8,
                repository: Some("parent".to_string()),
                attributes: Default::default(),
            },
            ModuleRef {
                name: "ecs".to_string(),
                source: ModuleSource::Registry {
                    hostname: "registry.terraform.io".to_string(),
                    namespace: "terraform-aws-modules".to_string(),
                    name: "ecs".to_string(),
                    provider: "aws".to_string(),
                },
                version_constraint: Some(Constraint::parse("5.12.0").unwrap()),
                file_path: PathBuf::from("compute/main.tf"),
                line_number: 20,
                repository: Some("child-a".to_string()),
                attributes: Default::default(),
            },
            // Git module with a branch named 3.0.0 and not a tag (should not be deprecated)
            ModuleRef {
                name: "resource_group".to_string(),
                source: ModuleSource::Git {
                    host: "ssh.dev.azure.com/v3/foo-bar/Terraform/mod-azurerm-resource-group".to_string(),
                    url: "ssh://git@ssh.dev.azure.com/v3/foo-bar/Terraform/mod-azurerm-resource-group"
                        .to_string(),
                    ref_: Some("3.0.0".to_string()),
                    subdir: Some("modules/resource-group".to_string()),
                },
                version_constraint: None,
                file_path: PathBuf::from("azure/main.tf"),
                line_number: 5,
                repository: Some("parent".to_string()),
                attributes: Default::default(),
            },
            // Git module with non-deprecated tag
            ModuleRef {
                name: "resource_group".to_string(),
                source: ModuleSource::Git {
                    host: "ssh.dev.azure.com/v3/foo-bar/Terraform/mod-azurerm-resource-group".to_string(),
                    url: "ssh://git@ssh.dev.azure.com/v3/foo-bar/Terraform/mod-azurerm-resource-group"
                        .to_string(),
                    ref_: Some("refs/tags/3.0.0".to_string()),
                    subdir: Some("".to_string()),
                },
                version_constraint: None,
                file_path: PathBuf::from("azure/main.tf"),
                line_number: 30,
                repository: Some("child-b".to_string()),
                attributes: Default::default(),
            },
        ];

        let result = analyzer.analyze(&modules, &[], &[]);
        println!("{result:#?}");
        assert_eq!(result.modules.len(), 3);

        let deprecated_sources: HashSet<String> = result
            .modules
            .iter()
            .map(|m| m.source.canonical_id())
            .collect();

        eprintln!("deprecated_sources: {deprecated_sources:#?}");

        assert!(deprecated_sources.contains("registry.terraform.io/terraform-aws-modules/vpc/aws"));
        assert!(deprecated_sources.contains("registry.terraform.io/terraform-aws-modules/eks/aws"));
        assert!(deprecated_sources.contains(
            "ssh.dev.azure.com/v3/foo-bar/Terraform/mod-azurerm-resource-group?ref=refs/tags/3.0.0"
        ));
    }
}
