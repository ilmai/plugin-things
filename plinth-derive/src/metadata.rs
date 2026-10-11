use std::path::{Path, PathBuf};

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct BundleMetadata {
    name: String,
    vendor: String,
    clap_id: String,
    vst3_class_id: Uuid,
}

pub fn generate_bundle_metadata() -> TokenStream {
    let manifest_path =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let BundleMetadata {
        name,
        vendor,
        clap_id,
        vst3_class_id,
    } = match read_bundle_metadata(&manifest_path) {
        Ok(bundle) => bundle,
        Err(message) => return quote! { compile_error!(#message); },
    };
    let vst3_class_id = Literal::u128_unsuffixed(vst3_class_id.as_u128());
    let manifest_path = manifest_path.to_string_lossy();
    quote! {
        // Track `Cargo.toml`, so changes in [package.metadata] force a rebuild
        const _: &[u8] = include_bytes!(#manifest_path);

        pub const NAME: &str = #name;
        pub const VENDOR: &str = #vendor;
        pub const VERSION: &str = env!("CARGO_PKG_VERSION");
        pub const CLAP_ID: &str = #clap_id;
        pub const VST3_CLASS_ID: u128 = #vst3_class_id;
    }
}

fn read_bundle_metadata(manifest_path: &Path) -> Result<BundleMetadata, String> {
    let manifest = std::fs::read_to_string(manifest_path)
        .map_err(|error| format!("Failed to read '{}': {error}", manifest_path.display()))?;
    let manifest = toml::from_str::<toml::Table>(&manifest)
        .map_err(|error| format!("Failed to parse '{}': {error}", manifest_path.display()))?;
    let manifest = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("bundle"))
        .cloned()
        .ok_or("Cargo.toml has no [package.metadata.bundle]")?;
    manifest
        .try_into()
        .map_err(|error| format!("Invalid [package.metadata.bundle]: {error}"))
}
