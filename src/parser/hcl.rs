//! HCL file parser implementation.
//!
//! This module provides the core HCL parsing functionality using the `hcl-rs` crate.

use crate::VersionRange;
use crate::config::Config;
use crate::error::{MonPhareError, ErrorCollector, Result};
use crate::parser::{Parser, SKIP_FILES, TERRAFORM_EXTENSIONS};
use crate::types::{Constraint, ModuleRef, ParsedHcl, ProviderRef, RuntimeRef, RuntimeSource};

use hcl::{Block, Body, Expression};
use std::path::Path;
use walkdir::WalkDir;

/// HCL parser for Terraform/OpenTofu files.
///
/// The parser walks directories, reads `.tf` files, and extracts
/// module and provider information.
pub struct HclParser {
    /// Configuration for parsing behavior
    config: Config,
}

impl HclParser {
    /// Create a new HCL parser with the given configuration.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Parse all Terraform files in a directory.
    ///
    /// Recursively walks the directory tree and parses all `.tf` files.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory doesn't exist or if parsing fails
    /// for any file (unless `continue_on_error` is enabled in config).
    pub async fn parse_directory(&self, path: &Path) -> Result<ParsedHcl> {
        if !path.exists() {
            return Err(crate::err!(DirectoryNotFound {
                path: path.to_path_buf(),
            }));
        }

        let mut result = ParsedHcl::default();
        let mut error_collector = ErrorCollector::new();

        // Determine repository name from path
        let repository = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from);

        // Walk directory tree
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !self.should_skip(e.path()))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read directory entry");
                    continue;
                }
            };

            let file_path = entry.path();

            // Skip directories
            if file_path.is_dir() {
                continue;
            }

            // Check file extension
            if !self.is_terraform_file(file_path) {
                continue;
            }

            tracing::debug!(file = %file_path.display(), "Parsing file");

            // Read and parse file
            match self.parse_file(file_path, repository.as_deref()).await {
                Ok(parsed) => {
                    result.merge(parsed);
                }
                Err(e) => {
                    if self.config.scan.continue_on_error && e.is_recoverable() {
                        tracing::warn!(
                            file = %file_path.display(),
                            "failed to parse file, continuing: {}",
                            e
                        );
                        error_collector.add(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // Log summary
        tracing::info!(
            modules = result.modules.len(),
            providers = result.providers.len(),
            files = result.files.len(),
            errors = error_collector.count(),
            "Parsing complete"
        );

        Ok(result)
    }

    /// Parse a single Terraform file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub async fn parse_file(&self, path: &Path, repository: Option<&str>) -> Result<ParsedHcl> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| MonPhareError::io(path, e, file!(), line!()))?;

        self.parse_content(&content, path, repository)
    }

    /// Check if a path should be skipped.
    fn should_skip(&self, path: &Path) -> bool {
        // Check against skip patterns
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            // Skip hidden files/directories
            if file_name.starts_with('.') {
                tracing::debug!(path = %path.display(), reason = "hidden file/directory", "Skipping path");
                return true;
            }

            // Skip known directories
            if SKIP_FILES.iter().any(|s| file_name == *s) {
                tracing::debug!(path = %path.display(), reason = "known skip file", "Skipping path");
                return true;
            }

            // Check config exclusions
            if self.config.scan.exclude_patterns.iter().any(|pattern| {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(file_name))
                    .unwrap_or(false)
            }) {
                tracing::debug!(path = %path.display(), reason = "matches exclude pattern", "Skipping path");
                return true;
            }
        }

        false
    }

    /// Check if a file is a Terraform file.
    fn is_terraform_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        TERRAFORM_EXTENSIONS
            .iter()
            .any(|ext| path_str.ends_with(ext))
    }
}

impl Parser for HclParser {
    fn parse_content(
        &self,
        content: &str,
        file_path: &Path,
        repository: Option<&str>,
    ) -> Result<ParsedHcl> {
        // Parse HCL content
        let body: Body = hcl::from_str(content).map_err(|e| crate::err!(HclParse {
            file: file_path.to_path_buf(),
            message: e.to_string(),
            line: None,
            column: None,
        }))?;

        let mut result = ParsedHcl {
            modules: Vec::new(),
            providers: Vec::new(),
            runtimes: Vec::new(),
            files: vec![file_path.to_path_buf()],
        };

        // Process all blocks
        for structure in body.into_inner() {
            if let hcl::Structure::Block(block) = structure {
                match block.identifier.as_str() {
                    "module" => {
                        if let Some(module_ref) =
                            parse_module_block(&block, file_path, repository)?
                        {
                            result.modules.push(module_ref);
                        } else {
                            tracing::warn!("Failed to parse module block: {}", block.identifier);
                            if !self.config.scan.continue_on_error {
                                return Err(crate::err!(HclParse {
                                    file: file_path.to_path_buf(),
                                    message: format!("Failed to parse module block: {}", block.identifier),
                                    line: None,
                                    column: None,
                                }));
                            }
                        }
                    }
                    "terraform" => {
                        match parse_terraform_block(&block, file_path, repository) {
                            Ok((runtimes, providers)) => {
                                result.providers.extend(providers);
                                result.runtimes.extend(runtimes);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse terraform block: {}", e);
                                if !self.config.scan.continue_on_error {
                                    return Err(crate::err!(HclParse {
                                        file: file_path.to_path_buf(),
                                        message: format!("Failed to parse terraform block: {}", e),
                                        line: None,
                                        column: None,
                                    }));
                                }
                            }
                        }
                    }
                    _ => {
                        // Ignore other block types (resource, data, variable, etc.)
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Parse a module block into a `ModuleRef`.
fn parse_module_block(
    block: &Block,
    file_path: &Path,
    repository: Option<&str>,
) -> Result<Option<ModuleRef>> {
    // Get module name from labels
    let name = block
        .labels
        .first()
        .map(|l| l.as_str().to_string())
        .unwrap_or_else(|| "unnamed".to_string());

    // Extract source attribute
    let source_str = match get_string_attribute(&block.body, "source") {
        Some(s) => s,
        None => {
            tracing::warn!(
                module = %name,
                file = %file_path.display(),
                "Module block missing source attribute"
            );
            return Ok(None);
        }
    };

    // Parse the source
    let source = super::parse_module_source(&source_str)?;



    // extract version constraint
    let version_constraint = match get_string_attribute(&block.body, "version") {
        None => None,
        Some(version_str) => {
            let constraint = Constraint::parse(&version_str).map_err(|e| {
                crate::err!(HclParse {
                    file: file_path.to_path_buf(),
                    message: format!("failed to parse version constraint: {}", e),
                    line: None,
                    column: None,
                })
            })?;
            Some(constraint)
        }
    };

    // Collect other attributes for reference
    let mut attributes = std::collections::HashMap::new();
    for attr in block.body.attributes() {
        let key = attr.key.as_str();
        if key != "source" && key != "version" {
            if let Some(value) = expression_to_string(&attr.expr) {
                attributes.insert(key.to_string(), value);
            }
        }
    }

    Ok(Some(ModuleRef {
        name,
        source,
        version_constraint,
        file_path: file_path.to_path_buf(),
        line_number: 0, // HCL-rs doesn't provide line numbers easily
        repository: repository.map(String::from),
        attributes,
    }))
}

/// Parse a terraform block for required_providers.
fn parse_terraform_block(
    block: &Block,
    file_path: &Path,
    repository: Option<&str>,
) -> Result<(Vec<RuntimeRef>, Vec<ProviderRef> )> {
    let mut providers = Vec::new();
    let mut runtimes = Vec::new();

    // Look for nested blocks
    for structure in block.body.clone().into_inner() {
        if let hcl::Structure::Block(nested_block) = &structure {
            if nested_block.identifier.as_str() == "required_providers" {
                // parse each provider in required_providers
                for attr in nested_block.body.attributes() {
                    let provider_name = attr.key.as_str().to_string();

                    // the value can be a string (version only) or an object (source + version)
                    let (source, version_constraint) = parse_provider_requirement(&attr.expr)
                        .map_err(|e| {
                            crate::err!(HclParse {
                                file: file_path.to_path_buf(),
                                message: format!("failed to parse provider '{}': {}", provider_name, e),
                                line: None,
                                column: None,
                            })
                        })?;

                    providers.push(ProviderRef {
                        name: provider_name,
                        source,
                        version_constraint,
                        file_path: file_path.to_path_buf(),
                        line_number: 0,
                        repository: repository.map(String::from),
                    });
                }
            }
        }
        if let hcl::Structure::Attribute(attribute) = &structure {
            if attribute.key() == "required_version" {
                let version = parse_required_version(attribute.expr(), file_path)?;
                runtimes.push(RuntimeRef {
                    name: "terraform".to_string(),
                    version,
                    source: RuntimeSource::Terraform, // TODO: support opentofu
                    file_path: file_path.to_path_buf(),
                    line_number: 0,
                    repository: repository.map(String::from),
                });
            }
        }
    }

    Ok((runtimes, providers))
}

fn parse_required_version(expr: &Expression, file_path: &Path) -> Result<Constraint> {
    if let Expression::String(version) = expr {
        return Constraint::parse(version).map_err(|e| {
            crate::err!(HclParse {
                file: file_path.to_path_buf(),
                message: format!("failed to parse required_version: {}", e),
                line: None,
                column: None,
            })
        });
    } else {
        tracing::warn!(
            file = %file_path.display(),
            "required version is not a string expression, but got {expr:?}"
        );
        return Err(crate::err!(HclParse {
            file: file_path.to_path_buf(),
            message: format!("required version is not a string expression, but got {expr:?}"),
            line: None,
            column: None,
        }));
    }
}

/// Parse a provider requirement expression.
fn parse_provider_requirement(
    expr: &Expression,
) -> Result<(Option<String>, Option<Constraint>)> {
    match expr {
        // Simple string version constraint before 0.13.0 terraform version (legacy) stuff eg.
        // terraform {
        //   required_providers {
        //     aws = ">= 4.0"      # Simple string - triggers Expression::String
        //     random = "~> 3.0"   # Simple string - triggers Expression::String
        //   }
        // }
        Expression::String(version_str) => {
            let constraint = Constraint::parse(version_str)?;
            Ok((None, Some(constraint)))
        }

        // Object with source and version
        Expression::Object(obj) => {
            let mut source = None;
            let mut version_constraint = None;

            for (key, value) in obj {
                let key_str = object_key_to_string(key);
                match key_str.as_str() {
                    "source" => {
                        source = expression_to_string(value);
                    }
                    "version" => {
                        if let Some(v) = expression_to_string(value) {
                            version_constraint = Some(Constraint::parse(&v)?);
                        }
                    }
                    _ => {
                        // Ignore other attributes (configuration_aliases, etc.)
                    }
                }
            }

            Ok((source, version_constraint))
        }

        _ => Ok((None, None)),
    }
}

/// Get a string attribute from a body.
fn get_string_attribute(body: &Body, key: &str) -> Option<String> {
    body.attributes()
        .find(|attr| attr.key.as_str() == key)
        .and_then(|attr| expression_to_string(&attr.expr))
}

/// Convert an expression to a string if possible.
fn expression_to_string(expr: &Expression) -> Option<String> {
    match expr {
        Expression::String(s) => Some(s.clone()),
        Expression::Number(n) => Some(n.to_string()),
        Expression::Bool(b) => Some(b.to_string()),
        Expression::TemplateExpr(t) => {
            // For template expressions, try to get the literal parts
            // This is a simplified approach; full template evaluation would be complex
            Some(format!("{t:?}"))
        }
        _ => None,
    }
}

/// Convert an object key to a string.
fn object_key_to_string(key: &hcl::ObjectKey) -> String {
    match key {
        hcl::ObjectKey::Identifier(id) => id.as_str().to_string(),
        hcl::ObjectKey::Expression(expr) => expression_to_string(expr).unwrap_or_default(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use comfy_table::ContentArrangement;

    use super::*;
    use crate::types::ModuleSource;

    fn create_test_parser() -> HclParser {
        HclParser::new(&Config::default())
    }

    #[test]
    fn test_parse_simple_module() {
        let parser = create_test_parser();
        let content = r#"
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "my-vpc"
  cidr = "10.0.0.0/16"
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), None)
            .unwrap();

        assert_eq!(result.modules.len(), 1);
        let module = &result.modules[0];
        assert_eq!(module.name, "vpc");
        assert!(matches!(module.source, ModuleSource::Registry { .. }));
        assert!(module.version_constraint.is_some());
    }

    #[test]
    fn test_parse_git_module() {
        let parser = create_test_parser();
        let content = r#"
module "example" {
  source = "git::https://github.com/example/terraform-module.git?ref=v1.0.0"
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), None)
            .unwrap();

        assert_eq!(result.modules.len(), 1);
        let module = &result.modules[0];
        assert!(matches!(module.source, ModuleSource::Git { .. }));
    }

    #[test]
    fn test_parse_local_module() {
        let parser = create_test_parser();
        let content = r#"
module "local" {
  source = "../modules/vpc"
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), None)
            .unwrap();

        assert_eq!(result.modules.len(), 1);
        let module = &result.modules[0];
        assert!(matches!(module.source, ModuleSource::Local { .. }));
    }

    #[test]
    fn test_parse_required_providers() {
        let parser = create_test_parser();
        let content = r#"
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 4.0, < 6.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), None)
            .unwrap();

        assert_eq!(result.providers.len(), 2);

        let aws = result.providers.iter().find(|p| p.name == "aws").unwrap();
        assert_eq!(aws.source.as_deref(), Some("hashicorp/aws"));
        assert!(aws.version_constraint.is_some());

        let random = result.providers.iter().find(|p| p.name == "random").unwrap();
        assert_eq!(random.source.as_deref(), Some("hashicorp/random"));
    }

    #[test]
    fn test_parse_multiple_modules() {
        let parser = create_test_parser();
        let content = r#"
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"
}

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.0"
}

module "rds" {
  source  = "terraform-aws-modules/rds/aws"
  version = "~> 6.0"
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), None)
            .unwrap();

        assert_eq!(result.modules.len(), 3);
    }

    #[test]
    fn test_parse_module_without_version() {
        let parser = create_test_parser();
        let content = r#"
module "no_version" {
  source = "terraform-aws-modules/vpc/aws"
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), None)
            .unwrap();

        assert_eq!(result.modules.len(), 1);
        assert!(result.modules[0].version_constraint.is_none());
    }

    #[test]
    fn test_parse_invalid_hcl() {
        let parser = create_test_parser();
        let content = "this is not valid { hcl";

        let result = parser.parse_content(content, Path::new("test.tf"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_terraform_file() {
        let parser = create_test_parser();

        assert!(parser.is_terraform_file(Path::new("main.tf")));
        assert!(parser.is_terraform_file(Path::new("variables.tf")));
        assert!(parser.is_terraform_file(Path::new("config.tf.json")));
        assert!(!parser.is_terraform_file(Path::new("readme.md")));
        assert!(!parser.is_terraform_file(Path::new("script.sh")));
    }

    #[test]
    fn test_should_skip() {
        let parser = create_test_parser();

        assert!(parser.should_skip(Path::new(".terraform")));
        assert!(parser.should_skip(Path::new(".git")));
        assert!(parser.should_skip(Path::new(".terragrunt-cache")));
        assert!(!parser.should_skip(Path::new("modules")));
        assert!(!parser.should_skip(Path::new("main.tf")));
    }

    #[test]
    fn test_parse_with_repository_context() {
        let parser = create_test_parser();
        let content = r#"
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"
}
"#;

        let result = parser
            .parse_content(content, Path::new("test.tf"), Some("my-repo"))
            .unwrap();

        assert_eq!(result.modules[0].repository.as_deref(), Some("my-repo"));
    }

    #[test]
    fn test_parse_required_version() {
        let parser = create_test_parser();

        let content = r#"
    terraform {
        required_version = "1.59.42"
    }
        "#;

        let result = parser.parse_content(content, Path::new("test.tf"), None).unwrap();
        assert_eq!(result.runtimes.len(), 1);

        let first_version = result.runtimes[0].version.ranges.first().unwrap();

        if let VersionRange::Exact(exact_version) = first_version {
            assert_eq!(exact_version.to_string(), semver::Version::parse("1.59.42").unwrap().to_string());
        } else {
            panic!("Expected exact version");
        }
    }
    #[test]
    fn test_parse_required_version_range() {
        let parser = create_test_parser();

        let content = r#"
    terraform {
        required_version = ">= 1.59.42, < 2.0.0"
    }
        "#;

        let result = parser.parse_content(content, Path::new("test.tf"), None).unwrap();
        assert_eq!(result.runtimes.len(), 1);

        let first_version = result.runtimes[0].version.ranges.first().unwrap();

        if let VersionRange::GreaterThanOrEqual(exact_version) = first_version {
            assert_eq!(exact_version.to_string(), semver::Version::parse("1.59.42").unwrap().to_string());
        } else {
            panic!("Expected exact version");
        }

        let second_version = result.runtimes[0].version.ranges.last().unwrap();
        if let VersionRange::LessThan(exact_version) = second_version {
            assert_eq!(exact_version.to_string(), semver::Version::parse("2.0.0").unwrap().to_string());
        } else {
            panic!("Expected less than version");
        }
    }


}

