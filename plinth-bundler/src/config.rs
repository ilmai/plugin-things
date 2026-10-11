use serde::Deserialize;

/// Rust representation of `[package.metadata.bundle]` of a plugin package's Cargo.toml.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct BundleConfig {
    /// Display name, also used as bundle and executable name.
    pub name: String,
    /// Display name for bundle infos.
    pub vendor: String,
    /// Base identifier: each macOS bundle appends its extension (e.g. `.component`).
    /// Required for macOS only.
    pub bundle_id_prefix: Option<String>,
    /// `type:subtype:manufacturer` for the AudioComponents plist entry.
    /// Required for macOS only. Without it, no AUv2 gets build.
    pub auv2_id: Option<String>,
}
