//! Language plugin system — runtime-loaded tree-sitter grammars.
//!
//! Plugins live in ~/.sentrux/plugins/<lang>/ and follow the Sentrux Plugin Spec:
//! - plugin.toml (manifest with metadata, capabilities, checksums)
//! - grammars/<platform>.so|.dylib (compiled tree-sitter grammar)
//! - queries/tags.scm (tree-sitter queries for structural extraction)
//!
//! Plugins are loaded at startup and registered alongside built-in languages.
//! Plugin languages take priority over built-in (allows user overrides).

pub mod loader;
pub mod manifest;
pub mod profile;

pub use loader::{LoadedPlugin, PluginLoadError, load_all_plugins, plugins_dir};
pub use manifest::PluginManifest;
pub use profile::{LanguageProfile, LanguageSemantics, LanguageThresholds, ComplexityKeywords, DEFAULT_PROFILE};
