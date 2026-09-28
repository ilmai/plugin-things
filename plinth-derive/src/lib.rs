mod enums;
mod kind;
mod metadata;

use enums::generate_enum;
use kind::generate_parameter_kind;
use metadata::generate_bundle_metadata;
use proc_macro::TokenStream;
use syn::{parse::Nothing, parse_macro_input};

#[proc_macro_derive(Enum, attributes(name))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let output = generate_enum(input);
    output.into()
}

#[proc_macro_derive(ParameterKind)]
pub fn derive_parameter_kind(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let output = generate_parameter_kind(input);
    output.into()
}

/// Defines `NAME`, `VENDOR`, `VERSION`, and `CLAP_ID` and `VST3_CLASS_ID` (an `u128`) from
/// `[package.metadata.bundle]` and the package version in the calling package's Cargo.toml.
/// The keys are used by plinth-bundler too, so the plugin and bundler use the same consts.
#[proc_macro]
pub fn bundle_metadata(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as Nothing);
    generate_bundle_metadata().into()
}
