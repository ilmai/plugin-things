use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, LitStr, Meta, Expr, Lit};

pub fn generate_enum(input: DeriveInput) -> TokenStream {
    let enum_id = input.ident.clone();
    let variants = parse_variants(&input);
    
    let variant_count = variants.len();
    let fmt_cases = generate_fmt_cases(&variants);
    let from_usize_cases = generate_from_usize_cases(&variants);
    let from_string_cases = generate_from_string_cases(&variants);
    let to_usize_cases = generate_to_usize_cases(&variants);
    let to_string_cases = generate_to_string_cases(&variants);

    quote! {
        impl std::fmt::Display for #enum_id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let name = match self {
                    #(#fmt_cases)*
                };

                write!(f, "{name}")
            }
        }

        impl ::plinth_plugin::Enum for #enum_id {
            const COUNT: usize = #variant_count;
    
            fn from_usize(value: usize) -> Option<Self> {
                match value {
                    #(#from_usize_cases)*
                    _ => None
                }
            }

            fn from_string(string: &str) -> Option<Self> {
                match string {
                    #(#from_string_cases)*
                    _ => None
                }
            }

            fn to_usize(&self) -> usize {
                match self {
                    #(#to_usize_cases)*
                }
            }

            fn to_string(&self) -> String {
                match self {
                    #(#to_string_cases)*
                }
            }
        }
    }
}

struct Variant {
    id: Ident,
    name: Option<LitStr>,
}

fn parse_variants(input: &DeriveInput) -> Vec<Variant> {
    let syn::Data::Enum(ref body) = input.data else {
        panic!("Macro can only be used on enums");
    };

    body.variants.iter()
        .map(|variant| {
            if !variant.fields.is_empty() {
                panic!("Macro can only be used on enums that doesn't contain fields");
            }

            let mut name = None;

            for attr in variant.attrs.iter() {
                if attr.path().is_ident("name") {
                    let Meta::NameValue(name_value) = &attr.meta else {
                        panic!("Name syntax error");
                    };

                    let Expr::Lit(lit) = &name_value.value else {
                        panic!("Name syntax error");
                    };
                    
                    let Lit::Str(str_lit) = &lit.lit else {
                        panic!("Name syntax error");
                    };

                    name = Some(str_lit.clone());
                }
            }

            Variant {
                id: variant.ident.clone(),
                name,
            }
        })
        .collect()
}

fn generate_fmt_cases(variants: &[Variant]) -> Vec<TokenStream> {
    variants.iter().map(|variant| {
        let id = &variant.id;
        let name = variant.name
            .as_ref()
            .map(|name| name.value())
            .unwrap_or(variant.id.to_string());

        quote! {
            Self::#id => #name,
        }
    }).collect()
}

fn generate_from_usize_cases(variants: &[Variant]) -> Vec<TokenStream> {
    variants.iter().enumerate().map(|(index, variant)| {
        let id = &variant.id;

        quote! {
            #index => Some(Self::#id),
        }
    }).collect()
}

fn generate_from_string_cases(variants: &[Variant]) -> Vec<TokenStream> {
    variants.iter().map(|variant| {
        let id = &variant.id;
        let id_string = id.to_string();
        let name = variant.name.clone().unwrap_or_else(|| LitStr::new(&id_string, variant.id.span()));

        quote! {
            #name => Some(Self::#id),
        }
    }).collect()
}

fn generate_to_usize_cases(variants: &[Variant]) -> Vec<TokenStream> {
    variants.iter().enumerate().map(|(index, variant)| {
        let id = &variant.id;

        quote! {
            Self::#id => #index,
        }
    }).collect()
}

fn generate_to_string_cases(variants: &[Variant]) -> Vec<TokenStream> {
    variants.iter().map(|variant| {
        let id = &variant.id;
        let id_string = id.to_string();
        let name = variant.name.clone().unwrap_or_else(|| LitStr::new(&id_string, variant.id.span()));

        quote! {
            Self::#id => #name.to_string(),
        }
    }).collect()
}
