//! Error types for MonPhare.
//!
//! This module defines a comprehensive error hierarchy using `thiserror`
//! for proper error handling throughout the application. All errors
//! include context and can be easily propagated using the `?` operator.
//!
//! # Error Categories
//!
//! - **Parse errors**: HCL parsing failures, invalid syntax
//! - **IO errors**: File system operations, network issues
//! - **Git errors**: Repository cloning, authentication
//! - **Config errors**: Invalid configuration files
//! - **Analysis errors**: Constraint conflicts, graph building
//!
//! # Example
//!
//! ```rust
//! use monphare::error::{MonPhareError, Result};
//!
//! fn parse_file(path: &str) -> Result<()> {
//!     let content = std::fs::read_to_string(path)
//!         .map_err(|e| MonPhareError::Io {
//!             path: path.into(),
//!             source: e,
//!             src_path: file!(),
//!             src_line: line!(),
//!         })?;
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;
use thiserror::Error;

/// Macro to create errors with automatic source location tracking.
///
/// Usage:
/// ```ignore
/// return Err(err!(ConfigMissing { key: "api_key".to_string() }));
/// ```
#[macro_export]
macro_rules! err {
    ($variant:ident { $($field:ident: $value:expr),* $(,)? }) => {
        $crate::error::MonPhareError::$variant {
            $($field: $value,)*
            src_path: file!(),
            src_line: line!(),
        }
    };
}

/// A specialized Result type for MonPhare operations.
pub type Result<T> = std::result::Result<T, MonPhareError>;

/// The main error type for MonPhare.
///
/// This enum covers all possible error conditions that can occur
/// during scanning, parsing, analysis, and reporting.
#[derive(Error, Debug)]
pub enum MonPhareError {
    // =========================================================================
    // I/O and File System Errors
    // =========================================================================
    /// I/O error with path context.
    #[error("I/O error at '{path}' ({src_path}:{src_line}): {source}")]
    Io {
        /// The path where the error occurred
        path: PathBuf,
        /// The underlying I/O error
        #[source]
        source: std::io::Error,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// File not found.
    #[error("File not found: {path} ({src_path}:{src_line})")]
    FileNotFound {
        /// The missing file path
        path: PathBuf,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Directory not found.
    #[error("Directory not found: {path} ({src_path}:{src_line})")]
    DirectoryNotFound {
        /// The missing directory path
        path: PathBuf,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Permission denied.
    #[error("Permission denied: {path} ({src_path}:{src_line})")]
    PermissionDenied {
        /// The path that couldn't be accessed
        path: PathBuf,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // HCL Parsing Errors
    // =========================================================================
    /// HCL parsing error.
    #[error("Failed to parse HCL in '{file}' \n\t({src_path}:{src_line}): {message}")]
    HclParse {
        /// The file being parsed
        file: PathBuf,
        /// Error message
        message: String,
        /// Line number (if available)
        line: Option<usize>,
        /// Column number (if available)
        column: Option<usize>,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Invalid HCL structure (e.g., missing required attributes).
    #[error("Invalid HCL structure in '{file}' ({src_path}:{src_line}): {message}")]
    HclStructure {
        /// The file with the invalid structure
        file: PathBuf,
        /// Description of the structural issue
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Module source parsing error.
    #[error("Failed to parse module source '{module_source}' ({src_path}:{src_line}): {message}")]
    ModuleSourceParse {
        /// The source string that failed to parse
        module_source: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Version and Constraint Errors
    // =========================================================================
    /// Version parsing error.
    #[error("Failed to parse version '{version}' ({src_path}:{src_line}): {source}")]
    VersionParse {
        /// The version string that failed to parse
        version: String,
        /// The underlying semver error
        #[source]
        source: semver::Error,

        /// Source file path
        src_path: &'static str,

        /// Source line number
        src_line: u32,
    },

    /// Invalid constraint syntax.
    #[error("Invalid version constraint '{constraint}' ({src_path}:{src_line}): {message}")]
    ConstraintParse {
        /// The constraint string that failed to parse
        constraint: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Git Errors
    // =========================================================================
    /// Git operation error.
    #[error("Git error ({src_path}:{src_line}): {message}")]
    Git {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Git authentication error.
    #[error("Git authentication failed for '{url}' ({src_path}:{src_line}): {message}")]
    GitAuth {
        /// The repository URL
        url: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Git clone error.
    #[error("Failed to clone repository '{url}' ({src_path}:{src_line}): {message}")]
    GitClone {
        /// The repository URL
        url: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Invalid Git URL.
    #[error("Invalid Git URL '{url}' ({src_path}:{src_line}): {message}")]
    InvalidGitUrl {
        /// The invalid URL
        url: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Unsupported Git provider.
    #[error("Unsupported Git provider for URL '{url}' ({src_path}:{src_line})")]
    UnsupportedGitProvider {
        /// The URL with unsupported provider
        url: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Configuration Errors
    // =========================================================================
    /// Configuration parsing error.
    #[error("Failed to parse configuration ({src_path}:{src_line}): {message}")]
    ConfigParse {
        /// Error message
        message: String,
        /// The underlying error (if any)
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Invalid configuration value.
    #[error("Invalid configuration value for '{key}' ({src_path}:{src_line}): {message}")]
    ConfigValue {
        /// The configuration key
        key: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Missing required configuration.
    #[error("Missing required configuration: {key} ({src_path}:{src_line})")]
    ConfigMissing {
        /// The missing configuration key
        key: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Graph Errors
    // =========================================================================
    /// Graph building error.
    #[error("Failed to build dependency graph ({src_path}:{src_line}): {message}")]
    GraphBuild {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Circular dependency detected.
    #[error("Circular dependency detected ({src_path}:{src_line}): {cycle}")]
    CircularDependency {
        /// Description of the cycle
        cycle: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Analysis Errors
    // =========================================================================
    /// Analysis error.
    #[error("Analysis error ({src_path}:{src_line}): {message}")]
    Analysis {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Report Errors
    // =========================================================================
    /// Report generation error.
    #[error("Failed to generate report ({src_path}:{src_line}): {message}")]
    ReportGeneration {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Template rendering error.
    #[error("Template rendering error ({src_path}:{src_line}): {message}")]
    TemplateRender {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Network Errors
    // =========================================================================
    /// HTTP request error.
    #[error("HTTP request failed ({src_path}:{src_line}): {message}")]
    Http {
        /// Error message
        message: String,
        /// HTTP status code (if available)
        status_code: Option<u16>,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Network timeout.
    #[error("Network timeout ({src_path}:{src_line}): {message}")]
    Timeout {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // API Errors (for future SaaS integration)
    // =========================================================================
    /// API error related to VCS platforms.
    #[error("VCS API error ({platform}) ({src_path}:{src_line}): {message}")]
    VcsApi {
        /// The VCS platform (e.g., "github", "gitlab")
        platform: String,
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    // =========================================================================
    // Generic Errors
    // =========================================================================
    /// Internal error (should not happen in normal operation).
    #[error("Internal error ({src_path}:{src_line}): {message}")]
    Internal {
        /// Error message
        message: String,
        /// Source file path
        src_path: &'static str,
        /// Source line number
        src_line: u32,
    },

    /// Multiple errors occurred.
    #[error("Multiple errors occurred ({count} total)")]
    Multiple {
        /// Number of errors
        count: usize,
        /// The individual errors
        errors: Vec<MonPhareError>,
    },
}

impl MonPhareError {
    /// Creates an `Io` error.
    #[must_use]
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error, src_path: &'static str, src_line: u32) -> Self {
        Self::Io { path: path.into(), source, src_path, src_line }
    }

    /// Creates an `HclParse` error.
    #[must_use]
    pub fn hcl_parse(file: PathBuf, message: String, line: Option<usize>, column: Option<usize>, src_path: &'static str, src_line: u32) -> Self {
        Self::HclParse { file, message, line, column, src_path, src_line }
    }

    /// Creates a `Git` error.
    #[must_use]
    pub fn git(message: String, src_path: &'static str, src_line: u32) -> Self {
        Self::Git { message, src_path, src_line }
    }

    /// Creates a `ConfigParse` error.
    #[must_use]
    pub fn config_parse(message: String, source: Option<Box<dyn std::error::Error + Send + Sync>>, src_path: &'static str, src_line: u32) -> Self {
        Self::ConfigParse { message, source, src_path, src_line }
    }

    /// Creates an `Internal` error.
    #[must_use]
    pub fn internal(message: String, src_path: &'static str, src_line: u32) -> Self {
        Self::Internal { message, src_path, src_line }
    }

    /// Determines if the error is recoverable (e.g., should continue scanning other repositories).
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::GitClone { .. }
            | Self::GitAuth { .. }
            | Self::InvalidGitUrl { .. }
            | Self::UnsupportedGitProvider { .. }
            | Self::HclParse { .. }
            | Self::HclStructure { .. }
            | Self::ModuleSourceParse { .. }
            | Self::VersionParse { .. }
            | Self::ConstraintParse { .. }
            | Self::ConfigParse { .. }
            | Self::ConfigValue { .. }
            | Self::ConfigMissing { .. }
            | Self::Http { .. }
            | Self::Timeout { .. }
            | Self::VcsApi { .. } => true,
            _ => false,
        }
    }

    /// Returns the appropriate exit code for the error.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Io { source, .. } if source.kind() == std::io::ErrorKind::PermissionDenied => 13,
            Self::FileNotFound { .. } => 14,
            Self::DirectoryNotFound { .. } => 15,
            Self::PermissionDenied { .. } => 13,
            Self::GitAuth { .. } => 16,
            Self::GitClone { .. } => 17,
            Self::ConfigParse { .. } => 18,
            Self::ConfigValue { .. } => 19,
            Self::ConfigMissing { .. } => 20,
            Self::Multiple { .. } => 21,
            Self::VcsApi { .. } => 22, // New exit code for VCS API errors
            _ => 1, // Generic unhandled error
        }
    }

    /// Consolidates multiple errors into a single `MonPhareError::Multiple` if there's more than one.
    /// Otherwise, returns the single error or `Ok(())` if no errors.
    pub fn collect(errors: Vec<Self>) -> Result<()> {
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(Self::Multiple {
                count: errors.len(),
                errors,
            })
        }
    }
}

/// Extension trait for `Result` to add context to errors.
pub trait ResultExt<T, E> {
    /// Adds a file path context to an I/O error.
    fn with_path(self, path: impl Into<PathBuf>) -> Result<T>;

    /// Converts a general error into an `HclParse` error with context.
    fn to_hcl_parse_error(self, file: impl Into<PathBuf>, message: String, line: Option<usize>, column: Option<usize>) -> Result<T>;

    /// Converts a general error into a `Git` error with context.
    fn to_git_error(self, message: String) -> Result<T>;

    /// Converts a general error into a `ConfigParse` error with context.
    fn to_config_parse_error(self, message: String) -> Result<T>;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn with_path(self, path: impl Into<PathBuf>) -> Result<T> {
        self.map_err(|e| MonPhareError::Io {
            path: path.into(),
            source: *e.into().downcast::<std::io::Error>().unwrap_or_else(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
            }),
            src_path: file!(),
            src_line: line!(),
        })
    }

    fn to_hcl_parse_error(self, file: impl Into<PathBuf>, message: String, line: Option<usize>, column: Option<usize>) -> Result<T> {
        self.map_err(|_| MonPhareError::hcl_parse(file.into(), message, line, column, file!(), line!()))
    }

    fn to_git_error(self, message: String) -> Result<T> {
        self.map_err(|_| MonPhareError::git(message, file!(), line!()))
    }

    fn to_config_parse_error(self, message: String) -> Result<T> {
        self.map_err(|e| MonPhareError::config_parse(message, Some(e.into()), file!(), line!()))
    }
}

// Add the From implementations back
impl From<std::io::Error> for MonPhareError {
    fn from(source: std::io::Error) -> Self {
        // This conversion is typically used when a PathBuf is not readily available
        // For errors where a path is known, prefer using MonPhareError::io(path, source, file!(), line!())
        Self::Io {
            path: PathBuf::new(),
            source,
            src_path: file!(),
            src_line: line!(),
        }
    }
}

impl From<serde_json::Error> for MonPhareError {
    fn from(source: serde_json::Error) -> Self {
        Self::Internal {
            message: format!("JSON serialization/deserialization error: {}", source),
            src_path: file!(),
            src_line: line!(),
        }
    }
}

/// A utility for collecting multiple errors during parsing or processing.
#[derive(Debug, Default)]
pub struct ErrorCollector {
    errors: Vec<MonPhareError>,
}

impl ErrorCollector {
    /// Create a new error collector.
    #[must_use]
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error to the collection.
    pub fn add(&mut self, error: MonPhareError) {
        self.errors.push(error);
    }

    /// Get the number of collected errors.
    #[must_use]
    pub fn count(&self) -> usize {
        self.errors.len()
    }

    /// Check if there are any errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Convert to a Result, returning Multiple error if there are any errors.
    pub fn into_result(self) -> Result<()> {
        MonPhareError::collect(self.errors)
    }
}

