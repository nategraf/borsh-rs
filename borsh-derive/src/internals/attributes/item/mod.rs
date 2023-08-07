use crate::internals::attributes::{BORSH, INIT, SKIP, USE_DISCRIMINANT};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{spanned::Spanned, Attribute, DeriveInput, Expr, ItemEnum, Path};

pub fn check_item_attributes(derive_input: &DeriveInput) -> Result<(), TokenStream> {
    for attr in &derive_input.attrs {
        if attr.path().is_ident(SKIP.0) {
            return Err(syn::Error::new(
                derive_input.ident.span(),
                "`borsh_skip` is not allowed as derive input attribute",
            )
            .to_compile_error());
        }
        if attr.path().is_ident(BORSH.0) {
            attr.parse_nested_meta(|meta| {
                if !meta.path.is_ident(USE_DISCRIMINANT) {
                    return Err(syn::Error::new(
                        meta.path.span(),
                        "`use_discriminant` is the only supported attribute for `borsh`",
                    ));
                }
                if meta.path.is_ident(USE_DISCRIMINANT) {
                    let _expr: Expr = meta.value()?.parse()?;
                    if let syn::Data::Struct(ref _data) = derive_input.data {
                        return Err(syn::Error::new(
                            derive_input.ident.span(),
                            "borsh(use_discriminant=<bool>) does not support structs",
                        ));
                    }
                }

                Ok(())
            })
            .map_err(|err| err.to_compile_error())?;
        }
    }
    Ok(())
}

pub fn contains_use_discriminant(input: &ItemEnum) -> Result<bool, syn::Error> {
    if input.variants.len() > 256 {
        return Err(syn::Error::new(
            input.span(),
            "up to 256 enum variants are supported",
        ));
    }

    let attrs = &input.attrs;
    let mut use_discriminant = None;
    for attr in attrs {
        if attr.path().is_ident(BORSH.0) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(USE_DISCRIMINANT) {
                    let value_expr: Expr = meta.value()?.parse()?;
                    let value = value_expr.to_token_stream().to_string();
                    match value.as_str() {
                        "true" => {
                            use_discriminant = Some(true);
                        }
                        "false" => use_discriminant = Some(false),
                        _ => {
                            return Err(syn::Error::new(
                                value_expr.span(),
                                "`use_discriminant` accepts only `true` or `false`",
                            ));
                        }
                    };
                }

                Ok(())
            })?;
        }
    }
    let has_explicit_discriminants = input
        .variants
        .iter()
        .any(|variant| variant.discriminant.is_some());
    if has_explicit_discriminants && use_discriminant.is_none() {
        return Err(syn::Error::new(
                input.ident.span(),
                "You have to specify `#[borsh(use_discriminant=true)]` or `#[borsh(use_discriminant=false)]` for all enums with explicit discriminant",
            ));
    }
    Ok(use_discriminant.unwrap_or(false))
}

pub(crate) fn contains_initialize_with(attrs: &[Attribute]) -> Option<Path> {
    for attr in attrs.iter() {
        if attr.path() == INIT {
            let mut res = None;
            let _ = attr.parse_nested_meta(|meta| {
                res = Some(meta.path);
                Ok(())
            });
            return res;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::internals::test_helpers::{
        local_insta_assert_debug_snapshot, local_insta_assert_snapshot,
    };
    use quote::{quote, ToTokens};
    use syn::ItemEnum;

    use super::*;
    #[test]
    fn test_check_use_discriminant() {
        let item_enum: ItemEnum = syn::parse2(quote! {
            #[derive(BorshDeserialize, Debug)]
            #[borsh(use_discriminant = false)]
            enum AWithUseDiscriminantFalse {
                X,
                Y,
            }
        })
        .unwrap();
        let actual = contains_use_discriminant(&item_enum);
        assert!(!actual.unwrap());
    }

    #[test]
    fn test_check_use_discriminant_true() {
        let item_enum: ItemEnum = syn::parse2(quote! {
            #[derive(BorshDeserialize, Debug)]
            #[borsh(use_discriminant = true)]
            enum AWithUseDiscriminantTrue {
                X,
                Y,
            }
        })
        .unwrap();
        let actual = contains_use_discriminant(&item_enum);
        assert!(actual.unwrap());
    }

    #[test]
    fn test_check_use_discriminant_wrong_value() {
        let item_enum: ItemEnum = syn::parse2(quote! {
            #[derive(BorshDeserialize, Debug)]
            #[borsh(use_discriminant = 111)]
            enum AWithUseDiscriminantFalse {
                X,
                Y,
            }
        })
        .unwrap();
        let actual = contains_use_discriminant(&item_enum);
        let err = match actual {
            Ok(..) => unreachable!("expecting error here"),
            Err(err) => err,
        };
        local_insta_assert_debug_snapshot!(err);
    }
    #[test]
    fn test_check_use_discriminant_on_struct() {
        let item_enum: DeriveInput = syn::parse2(quote! {
            #[derive(BorshDeserialize, Debug)]
            #[borsh(use_discriminant = false)]
            struct AWithUseDiscriminantFalse {
                x: X,
                y: Y,
            }
        })
        .unwrap();
        let actual = check_item_attributes(&item_enum);
        local_insta_assert_snapshot!(actual.unwrap_err().to_token_stream().to_string());
    }
    #[test]
    fn test_check_use_borsh_skip_on_whole_struct() {
        let item_enum: DeriveInput = syn::parse2(quote! {
            #[derive(BorshDeserialize, Debug)]
            #[borsh_skip]
            struct AWithUseDiscriminantFalse {
                 x: X,
                 y: Y,
            }
        })
        .unwrap();
        let actual = check_item_attributes(&item_enum);
        local_insta_assert_snapshot!(actual.unwrap_err().to_token_stream().to_string());
    }
    #[test]
    fn test_check_use_borsh_invalid_on_whole_struct() {
        let item_enum: DeriveInput = syn::parse2(quote! {
            #[derive(BorshDeserialize, Debug)]
            #[borsh(invalid)]
            enum AWithUseDiscriminantFalse {
                X,
                Y,
            }
        })
        .unwrap();
        let actual = check_item_attributes(&item_enum);
        local_insta_assert_snapshot!(actual.unwrap_err().to_token_stream().to_string());
    }
}