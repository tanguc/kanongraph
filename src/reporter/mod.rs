//! Report generation module.
//!
//! This module provides report generation in multiple formats:
//! - JSON: Machine-readable structured output
//! - Text: Human-readable CLI output
//! - HTML: Self-contained visual reports
//!
//! # Example
//!
//! ```rust,no_run
//! use driftops::reporter::Reporter;
//! use driftops::{Config, ScanResult};
//!
//! let config = Config::default();
//! let reporter = Reporter::new(&config);
//!
//! // Generate reports in different formats
//! // let json = reporter.generate(&result, ReportFormat::Json)?;
//! // let text = reporter.generate(&result, ReportFormat::Text)?;
//! // let html = reporter.generate(&result, ReportFormat::Html)?;
//! ```

mod html;
mod json;
mod text;

use crate::config::Config;
use crate::error::Result;
use crate::types::{ReportFormat, ScanResult};

pub use html::HtmlReporter;
pub use json::JsonReporter;
pub use text::TextReporter;

/// Report generator that supports multiple output formats.
pub struct Reporter {
    config: Config,
}

impl Reporter {
    /// Create a new reporter with the given configuration.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Generate a report in the specified format.
    ///
    /// # Errors
    ///
    /// Returns an error if report generation fails.
    pub fn generate(&self, result: &ScanResult, format: ReportFormat) -> Result<String> {
        match format {
            ReportFormat::Json => JsonReporter::new(&self.config).generate(result),
            ReportFormat::Text => TextReporter::new(&self.config).generate(result),
            ReportFormat::Html => HtmlReporter::new(&self.config).generate(result),
        }
    }
}

/// Trait for report generators.
pub trait ReportGenerator {
    /// Generate a report from scan results.
    ///
    /// # Errors
    ///
    /// Returns an error if generation fails.
    fn generate(&self, result: &ScanResult) -> Result<String>;
}

