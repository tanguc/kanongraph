//! HCL parsing module for Terraform/OpenTofu files.
//!
//! This module handles parsing of HCL (HashiCorp Configuration Language) files
//! to extract module blocks, provider requirements, and version constraints.
//!
//! # Supported Constructs
//!
//! - `module` blocks with source and version attributes
//! - `terraform.required_providers` blocks
//! - `terraform.required_version` constraints
//!
//! # Example
//!
//! ```rust,ignore
//! use kanongraph::parser::HclParser;
//! use kanongraph::Config;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::default();
//!     let parser = HclParser::new(&config);
//!     
//!     let result = parser.parse_directory("./terraform").await?;
//!     println!("Found {} modules", result.modules.len());
//!     Ok(())
//! }
//! ```

mod hcl;
mod source;

pub use hcl::HclParser;
pub use source::parse_module_source;

use crate::types::ParsedHcl;

/// File extensions to scan for Terraform/OpenTofu files.
pub const TERRAFORM_EXTENSIONS: &[&str] = &[".tf", ".tf.json"];

/// Files to skip during scanning.
pub const SKIP_FILES: &[&str] = &[".terraform", ".terragrunt-cache", "terraform.tfstate"];

/// Trait for parsing HCL content.
///
/// This trait allows for different parsing implementations
/// (e.g., for testing with mock parsers).
pub trait Parser: Send + Sync {
    /// Parse a single file's contents.
    ///
    /// # Errors
    ///
    /// Returns an error if the HCL content is invalid.
    fn parse_content(
        &self,
        content: &str,
        file_path: &std::path::Path,
        repository: Option<&str>,
    ) -> crate::Result<ParsedHcl>;
}

