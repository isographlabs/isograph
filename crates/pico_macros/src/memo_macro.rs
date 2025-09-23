use std::hash::{DefaultHasher, Hash, Hasher};

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Error, FnArg, ItemFn, PatType, ReturnType, Signature, parse_macro_input, parse_quote};

pub(crate) fn memo_macro(_args: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        sig,
        vis,
        block,
        attrs,
    } = parse_macro_input!(item as ItemFn);

    let fn_hash = hash(&sig);

    if sig.inputs.is_empty() {
        return Error::new_spanned(
            &sig,
            "Memoized function must have at least one argument (&Database)",
        )
        .to_compile_error()
        .into();
    }

    let db_arg = match &sig.inputs[0] {
        FnArg::Typed(PatType { pat, .. }) => pat,
        _ => unreachable!(),
    };

    let args = sig.inputs.iter().skip(1).map(|arg| match arg {
        FnArg::Typed(PatType { pat, ty, .. }) => (pat, ty),
        _ => unreachable!(),
    });

    let param_ids_blocks = args.clone().map(|(arg, ty)| match ArgType::parse(ty) {
        ArgType::Source => {
            let param_arg = match **ty {
                syn::Type::Reference(_) => quote!((*(#arg))),
                _ => quote!(#arg),
            };
            quote! {
                param_ids.push(#param_arg.into());
            }
        }
        ArgType::Other => {
            let intern_param = match **ty {
                syn::Type::Reference(_) => {
                    quote!(::pico::macro_fns::intern_borrowed_param(#db_arg, #arg))
                }
                _ => quote!(::pico::macro_fns::intern_owned_param(#db_arg, #arg)),
            };
            quote! {
                let param_id = #intern_param;
                param_ids.push(param_id);
            }
        }
    });

    let return_type = match &sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        ReturnType::Default => parse_quote!(()),
    };

    let mut new_sig = sig.clone();
    new_sig.output = ReturnType::Type(
        parse_quote!(->),
        Box::new(parse_quote!(::pico::MemoRef<#return_type>)),
    );

    let extract_parameters = args
        .enumerate()
        .map(|(i, (arg, ty))| {
            match ArgType::parse(ty) {
                ArgType::Source => {
                    let binding_expr = match **ty {
                        syn::Type::Reference(_) => quote!(&param_id.into()),
                        _ => quote!(param_id.into()),
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_id = derived_node_id.params[#i];
                            #binding_expr
                        };
                    }
                }
                ArgType::Other => {
                    let (target_type, binding_expr) = match **ty {
                        syn::Type::Reference(ref reference) => (&reference.elem, quote!(inner)),
                        _ => (ty, quote!(inner.clone())),
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_ref = ::pico::macro_fns::get_param(#db_arg, derived_node_id.params[#i])?;
                            let inner = param_ref
                                .downcast_ref::<#target_type>()
                                .expect("Unexpected param type. This is indicative of a bug in Pico.");
                            #binding_expr
                        };
                    }
                }
            }
        });

    let fn_name = sig.ident.to_string();
    let output = quote! {
        #(#attrs)*
        #vis #new_sig {
            let _memo_span = ::tracing::debug_span!(#fn_name).entered();
            let mut param_ids = ::pico::macro_fns::init_param_vec();
            #(
                #param_ids_blocks
            )*
            let derived_node_id = ::pico::DerivedNodeId::new(#fn_hash.into(), param_ids);
            let did_recalculate = ::pico::execute_memoized_function(
                #db_arg,
                derived_node_id,
                ::pico::InnerFn::new(|#db_arg, derived_node_id| {
                    use ::pico::Database;
                    #(
                        #extract_parameters
                    )*
                    let value: #return_type = (|| #block)();
                    Some(Box::new(value))
                })
            );
            debug_assert!(
                !matches!(did_recalculate, pico::DidRecalculate::Error),
                "Unexpected memo result. This is indicative of a bug in Pico."
            );
            ::pico::MemoRef::new(#db_arg, derived_node_id)
        }
    };

    output.into()
}

fn hash(input: &Signature) -> u64 {
    let mut s = DefaultHasher::new();
    input.to_token_stream().to_string().hash(&mut s);
    s.finish()
}

enum ArgType {
    Source,
    Other,
}

impl ArgType {
    pub fn parse(ty: &syn::Type) -> Self {
        if type_is(ty, "SourceId") {
            return ArgType::Source;
        }
        ArgType::Other
    }
}

fn type_is(ty: &syn::Type, target: &'static str) -> bool {
    let inner = match ty {
        syn::Type::Reference(r) => &*r.elem,
        _ => ty,
    };
    if let syn::Type::Path(type_path) = inner
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == target;
    }
    false
}
