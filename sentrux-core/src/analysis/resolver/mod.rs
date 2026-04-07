//! Import resolution — resolves import specifiers to file paths.
//!
//! Unified suffix-index resolver for ALL languages. No tier split.
//! JS/TS path aliases (tsconfig.json) handled via plugin-declared config.

pub mod helpers;
pub mod jdeps;
pub mod suffix;
