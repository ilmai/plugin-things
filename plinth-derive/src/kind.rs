use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

const ID_MASK: u32 = i32::MAX as u32;

pub fn generate_parameter_kind(input: DeriveInput) -> TokenStream {
    let enum_id = input.ident.clone();
    let variants = parse_variants(&input);
    
    let match_cases = generate_match_cases(enum_id.clone(), &variants);

    quote! {
        impl ::plinth_plugin::parameters::kind::ParameterKind for #enum_id {
        }

        impl Into<::plinth_plugin::ParameterId> for #enum_id {
            fn into(self) -> ::plinth_plugin::ParameterId {
                match self {
                    #(#match_cases)*
                }
            }
        }
    }
}

struct Variant {
    id: Ident,
    fields: Vec<Ident>,
}


fn parse_variants(input: &DeriveInput) -> Vec<Variant> {
    let syn::Data::Enum(ref body) = input.data else {
        panic!("Macro can only be used on enums");
    };

    body.variants.iter()
        .map(|variant| {
            let fields = variant.fields.iter()
                .map(|field| {
                    field.ident.clone().expect("Macro can't be used on tuple enums")
                })
                .collect();

            Variant {
                id: variant.ident.clone(),
                fields,
            }
        })
        .collect()
}

fn generate_match_cases(enum_id: Ident, variants: &[Variant]) -> Vec<TokenStream> {
    variants.iter().enumerate().map(|(index, variant)| {
        let variant_id = &variant.id;
        let fields = &variant.fields;

        if fields.is_empty() {
            quote! {
                #enum_id::#variant_id => {
                    ::plinth_plugin::xxhash_rust::xxh32::xxh32(&#index.to_le_bytes(), 0) & #ID_MASK
                }
            }
        } else {
            let field_hashes: Vec<_> = fields.iter()
                .map(|field_id| {
                    quote! {
                        let hash = ::plinth_plugin::xxhash_rust::xxh32::xxh32(&#field_id.to_le_bytes(), hash);
                    }
                })
                .collect();

            quote! {
                #enum_id::#variant_id { #(#fields),* } => {
                    let hash = ::plinth_plugin::xxhash_rust::xxh32::xxh32(&#index.to_le_bytes(), 0);
                    #(#field_hashes)*
                    hash & #ID_MASK
                }
            }
        }
    }).collect()
}
