//! Risky pattern detection for version constraints.
//!
//! This module identifies potentially problematic patterns in
//! Terraform version constraints.

use crate::config::Config;
use regex::Regex;
use std::sync::LazyLock;

/// Patterns that indicate potential issues with version constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskyPattern {
    /// Wildcard constraint (e.g., "*")
    Wildcard,
    /// Pre-release version (e.g., "1.0.0-beta")
    PreRelease,
    /// Exact version without flexibility (e.g., "= 1.0.0")
    ExactVersion,
    /// No upper bound (e.g., ">= 1.0" without "< X.0")
    NoUpperBound,
}

// Regex patterns for detecting risky patterns
static WILDCARD_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\*\s*$").expect("Invalid regex"));

static PRERELEASE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-(?:alpha|beta|rc|dev|pre)\d*").expect("Invalid regex"));

static EXACT_VERSION_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*=?\s*\d+\.\d+\.\d+\s*$").expect("Invalid regex"));

/// Checker for risky version constraint patterns.
pub struct PatternChecker {
    /// Whether to check for exact versions
    check_exact: bool,
    /// Whether to check for pre-release versions
    check_prerelease: bool,
    /// Whether to check for missing upper bounds
    check_upper_bound: bool,
}

impl PatternChecker {
    /// Create a new pattern checker with the given configuration.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            check_exact: config.analysis.check_exact_versions,
            check_prerelease: config.analysis.check_prerelease,
            check_upper_bound: config.analysis.check_upper_bound,
        }
    }

    /// Check a constraint string for risky patterns.
    ///
    /// Returns a list of all risky patterns found in the constraint.
    #[must_use]
    pub fn check(&self, constraint: &str) -> Vec<RiskyPattern> {
        let mut patterns = Vec::new();

        // Check for wildcard
        if WILDCARD_PATTERN.is_match(constraint) {
            patterns.push(RiskyPattern::Wildcard);
        }

        // Check for pre-release
        if self.check_prerelease && PRERELEASE_PATTERN.is_match(constraint) {
            patterns.push(RiskyPattern::PreRelease);
        }

        // Check for exact version
        if self.check_exact && EXACT_VERSION_PATTERN.is_match(constraint) {
            patterns.push(RiskyPattern::ExactVersion);
        }

        // Check for no upper bound
        if self.check_upper_bound && self.has_no_upper_bound(constraint) {
            patterns.push(RiskyPattern::NoUpperBound);
        }

        patterns
    }

    /// Check if a constraint has no upper bound.
    fn has_no_upper_bound(&self, constraint: &str) -> bool {
        // Pessimistic constraints (~>) have implicit upper bounds
        if constraint.contains("~>") {
            return false;
        }

        // Check if there's a >= without a corresponding <
        let has_lower = constraint.contains(">=") || constraint.contains('>');
        let has_upper = constraint.contains("<=") || constraint.contains('<');

        has_lower && !has_upper
    }
}

impl Default for PatternChecker {
    fn default() -> Self {
        Self {
            check_exact: true,
            check_prerelease: true,
            check_upper_bound: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_checker() -> PatternChecker {
        PatternChecker::default()
    }

    #[test]
    fn test_detect_wildcard() {
        let checker = default_checker();
        let patterns = checker.check("*");
        assert!(patterns.contains(&RiskyPattern::Wildcard));
    }

    #[test]
    fn test_detect_prerelease() {
        let checker = default_checker();

        let patterns = checker.check("1.0.0-beta");
        assert!(patterns.contains(&RiskyPattern::PreRelease));

        let patterns = checker.check("2.0.0-alpha1");
        assert!(patterns.contains(&RiskyPattern::PreRelease));

        let patterns = checker.check("3.0.0-rc1");
        assert!(patterns.contains(&RiskyPattern::PreRelease));
    }

    #[test]
    fn test_detect_exact_version() {
        let checker = default_checker();

        let patterns = checker.check("1.0.0");
        assert!(patterns.contains(&RiskyPattern::ExactVersion));

        let patterns = checker.check("= 1.0.0");
        assert!(patterns.contains(&RiskyPattern::ExactVersion));
    }

    #[test]
    fn test_detect_no_upper_bound() {
        let checker = default_checker();

        let patterns = checker.check(">= 1.0");
        assert!(patterns.contains(&RiskyPattern::NoUpperBound));

        let patterns = checker.check("> 1.0.0");
        assert!(patterns.contains(&RiskyPattern::NoUpperBound));
    }

    #[test]
    fn test_pessimistic_has_upper_bound() {
        let checker = default_checker();

        let patterns = checker.check("~> 1.0");
        assert!(!patterns.contains(&RiskyPattern::NoUpperBound));
    }

    #[test]
    fn test_range_has_upper_bound() {
        let checker = default_checker();

        let patterns = checker.check(">= 1.0, < 2.0");
        assert!(!patterns.contains(&RiskyPattern::NoUpperBound));
    }

    #[test]
    fn test_no_risky_patterns() {
        let checker = default_checker();

        let patterns = checker.check("~> 5.0");
        // Should only potentially have exact version if it matches
        assert!(!patterns.contains(&RiskyPattern::Wildcard));
        assert!(!patterns.contains(&RiskyPattern::PreRelease));
        assert!(!patterns.contains(&RiskyPattern::NoUpperBound));
    }

    #[test]
    fn test_config_disables_checks() {
        let checker = PatternChecker {
            check_exact: false,
            check_prerelease: false,
            check_upper_bound: false,
        };

        let patterns = checker.check("1.0.0-beta");
        assert!(!patterns.contains(&RiskyPattern::PreRelease));
        assert!(!patterns.contains(&RiskyPattern::ExactVersion));
    }
}

