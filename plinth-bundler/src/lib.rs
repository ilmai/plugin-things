//! Bundles plinth plugins as CLAP, VST3 and AUv2 (optional, macOS only) from a plugin
//! workspace's `xtask`:
//!
//! # Example
//!
//! ```no_run
//! // In your plugin workspace's xtask bin:
//! fn main() -> plinth_bundler::Result<()> {
//!     plinth_bundler::main()
//! }
//! ```
//!
//! # Requirements
//!
//! Your plugin's package must set following features and bundle metadata:
//!
//! ```toml
//! # In your plugin's Cargo.toml:
//! [features]
//! default = ["vst3", "clap"]
//! vst3 = [] # gates `export_vst3!`
//! clap = [] # gates `export_clap!`
//! # Optional: only needed with `auv2-id` on macOS
//! auv2 = ["clap", "dep:clap-wrapper"] # gates `clap_wrapper::export_auv2!`
//!
//! [package.metadata.bundle]
//! name = "My Plugin"                         # bundle and executable name
//! vendor = "My Company"                      # bundle vendor
//! bundle-id-prefix = "com.company.plugin"    # macOS only: + ".clap", ".vst3", ".component"
//! auv2-id = "aumu:mypl:MyCo"                 # macOS only: required when building AUv2 bundles
//! clap-id = "com.my-company.plugin"          # used by bundle_metadata!(), see below
//! vst3-class-id = "<GUID, dashes optional>"  # used by bundle_metadata!(), see below
//! ```
//!
//! The AUv2 bundle wraps the plugin's CLAP entry point via `clap-wrapper`, so with the `auv2`
//! feature enabled your plugin has to export both a CLAP and an AUv2 entry point:
//!
//! ```ignore
//! #[cfg(feature = "vst3")]
//! plinth_plugin::export_vst3!(MyPlugin);
//! #[cfg(feature = "clap")]
//! plinth_plugin::export_clap!(MyPlugin);
//! #[cfg(all(target_os = "macos", feature = "auv2"))]
//! clap_wrapper::export_auv2!();
//! ```
//!
//! `clap-id` and `vst3-class-id` aren't used for bundling: `plinth_derive::bundle_metadata!()`
//! creates constants from them, together with the name, vendor and version, so the plugin and
//! bundler can use the same values from Cargo.toml.

mod bundle;
mod config;

pub use bundle::main;
pub use anyhow::Result;
