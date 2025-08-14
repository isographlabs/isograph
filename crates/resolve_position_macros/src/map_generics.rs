use std::collections::HashMap;

use syn::{
    visit_mut::{self, VisitMut},
    Error,
};

struct GenericReplacer<'a> {
    generics_map: &'a HashMap<syn::Ident, syn::GenericArgument>,
}

impl<'a> VisitMut for GenericReplacer<'a> {
    fn visit_type_mut(&mut self, inner_type: &mut syn::Type) {
        match inner_type {
            syn::Type::Path(type_path) => {
                // Check if this is a simple generic parameter (like T)
                if type_path.qself.is_none()
                    && type_path.path.segments.len() == 1
                    && type_path.path.segments[0].arguments.is_empty()
                {
                    let ident = &type_path.path.segments[0].ident;

                    // If we have a mapping for this identifier, replace the entire type
                    if let Some(syn::GenericArgument::Type(replacement_type)) =
                        self.generics_map.get(ident)
                    {
                        *inner_type = replacement_type.clone();
                        return; // Don't recurse into the replacement
                    }
                }

                // Continue visiting nested types
                visit_mut::visit_type_path_mut(self, type_path);
            }
            _ => {
                // For all other type variants, continue visiting
                visit_mut::visit_type_mut(self, inner_type);
            }
        }
    }

    fn visit_generic_argument_mut(&mut self, arg: &mut syn::GenericArgument) {
        match arg {
            syn::GenericArgument::Lifetime(lifetime) => {
                // Check if we have a mapping for this lifetime
                if let Some(replacement) = self.generics_map.get(&lifetime.ident) {
                    *arg = replacement.clone();
                }
            }
            _ => {
                // Continue visiting for other generic arguments
                visit_mut::visit_generic_argument_mut(self, arg);
            }
        }
    }
}

pub(crate) fn replace_generics_in_type(
    mut inner_type: syn::Type,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> syn::Type {
    let mut replacer = GenericReplacer { generics_map };
    replacer.visit_type_mut(&mut inner_type);
    inner_type
}

pub(crate) fn validate_and_map_generics(
    input_generics: syn::Generics,
    self_type_generics: Option<syn::AngleBracketedGenericArguments>,
) -> Result<HashMap<syn::Ident, syn::GenericArgument>, proc_macro2::TokenStream> {
    // Extract the struct's generic parameters
    let struct_generics = input_generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Type(type_param) => type_param.ident.clone(),
            syn::GenericParam::Lifetime(lifetime_def) => lifetime_def.lifetime.ident.clone(),
            syn::GenericParam::Const(const_param) => const_param.ident.clone(),
        })
        .collect::<Vec<_>>();

    // Validate count matches
    let provided_count = self_type_generics
        .as_ref()
        .map(|generics| generics.args.len())
        .unwrap_or(0);
    let expected_count = struct_generics.len();

    if provided_count != expected_count {
        return Err(Error::new_spanned(
            &self_type_generics,
            format!(
                "Generic parameter count mismatch: expected {} ({}), got {}",
                expected_count,
                struct_generics
                    .iter()
                    .map(|g| g.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                provided_count
            ),
        )
        .to_compile_error());
    }

    if struct_generics.is_empty() {
        return Ok(HashMap::new());
    }

    Ok(struct_generics
        .into_iter()
        .zip(
            self_type_generics
                .expect("Expected self type generics to not be empty at this point")
                .args,
        )
        .collect())
}
