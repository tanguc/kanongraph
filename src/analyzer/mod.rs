//! Constraint analysis module.
//!
//! This module provides analysis of Terraform module and provider
//! version constraints to detect conflicts, deprecated versions,
//! and risky patterns.
//!
//! # Analysis Types
//!
//! 1. **Constraint Conflicts**: Detects when two modules require
//!    incompatible versions of the same provider or module.
//!
//! 2. **Missing Constraints**: Flags modules without version constraints.
//!
//! 3. **Broad Constraints**: Identifies overly permissive constraints
//!    like `>= 0.0.0`.
//!
//! 4. **Deprecated Versions**: Detects outdated module versions.
//!
//! 5. **Risky Patterns**: Flags wildcards, pre-release versions, etc.
//!
//! # Example
//!
//! ```rust,no_run
//! use kanongraph::analyzer::Analyzer;
//! use kanongraph::Config;
//!
//! let config = Config::default();
//! let analyzer = Analyzer::new(&config);
//!
//! // Analyze parsed data
//! // let result = analyzer.analyze(&graph, &modules, &providers)?;
//! ```

mod conflict;
mod patterns;
mod deprecation;

pub use conflict::Analyzer;
pub use patterns::{PatternChecker, RiskyPattern};
