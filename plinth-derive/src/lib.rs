mod enums;
mod kind;

use enums::generate_enum;
use kind::generate_parameter_kind;
use proc_macro::TokenStream;
use syn::parse_macro_input;

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
