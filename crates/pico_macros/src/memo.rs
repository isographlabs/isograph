use std::hash::{DefaultHasher, Hash, Hasher};

use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, Error, FnArg, GenericParam, ItemFn, Lifetime, PatType,
    ReturnType, Signature,
};

#[derive(Debug, FromMeta)]
struct MemoArgs {
    #[darling(default)]
    inner: bool,
    #[darling(default)]
    inner_ref: bool,
}

pub(crate) fn memo(args: TokenStream, item: TokenStream) -> TokenStream {
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

    let other_args = sig.inputs.iter().skip(1).map(|arg| match arg {
        FnArg::Typed(PatType { pat, .. }) => pat,
        _ => unreachable!(),
    });

    let argument_types = sig.inputs.iter().skip(1).map(|arg| match arg {
        FnArg::Typed(PatType { ty, .. }) => ty,
        _ => unreachable!(),
    });

    let param_ids_blocks = other_args
        .clone()
        .zip(argument_types.clone())
        .map(|(arg, ty)| match ArgType::parse(ty) {
            ArgType::Source | ArgType::MemoRef => {
                quote! {
                    param_ids.push(#arg.into());
                }
            }
            ArgType::Other => {
                let param_arg = match **ty {
                    syn::Type::Reference(_) => quote!(#arg),
                    _ => quote!(&#arg),
                };
                quote! {
                    let param_id: ::pico::ParamId = ::pico::macro_fns::hash(#param_arg).into();
                    if !::pico::macro_fns::param_exists(#db_arg, param_id) {
                        ::pico::macro_fns::intern_param(#db_arg, param_id, #arg.clone());
                    }
                    param_ids.push(param_id);
                }
            }
        });

    let return_type = match &sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        ReturnType::Default => parse_quote!(()),
    };

    let mut return_expr = quote! {
        ::pico::macro_fns::get_derived_node(#db_arg, derived_node_id)
            .expect("derived node must exist. This is indicative of a bug in Pico.")
            .value
            .as_any()
            .downcast_ref::<#return_type>()
            .expect("unexpected return type. This is indicative of a bug in Pico.")
    };

    let mut new_sig = sig.clone();

    let macro_args = match parse_macro_args(args, &sig) {
        Ok(parsed) => parsed,
        Err(e) => return e.to_compile_error().into(),
    };

    if macro_args.inner {
        return_expr = quote! { #return_expr.clone() };
    } else {
        let lifetime = get_fn_lifetime(&mut new_sig);
        if let Err(e) = check_db_lifetime(&mut new_sig.inputs[0], lifetime.clone()) {
            return e.to_compile_error().into();
        }
        if macro_args.inner_ref {
            new_sig.output = ReturnType::Type(
                parse_quote!(->),
                Box::new(parse_quote!(&#lifetime #return_type)),
            );
        } else {
            new_sig.output = ReturnType::Type(
                parse_quote!(->),
                Box::new(parse_quote!(::pico::MemoRef<#lifetime, #return_type>)),
            );
            return_expr = quote! { ::pico::MemoRef::new(#db_arg, derived_node_id) };
        }
    }

    let extract_parameters = other_args.clone().zip(argument_types.clone())
        .enumerate()
        .map(|(i, (arg, ty))| {
            match ArgType::parse(ty) {
                ArgType::Source => {
                    let maybe_ref = if matches!(**ty, syn::Type::Reference(_)) {
                        quote! { &param_id.into() }
                    } else {
                        quote! { param_id.into() }
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_id = derived_node_id.params[#i];
                            #maybe_ref
                        };
                    }
                }
                ArgType::MemoRef => {
                    let maybe_ref = if matches!(**ty, syn::Type::Reference(_)) {
                        quote! { &::pico::MemoRef::new(#db_arg, param_id.into()) }
                    } else {
                        quote! { ::pico::MemoRef::new(#db_arg, param_id.into()) }
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_id = derived_node_id.params[#i];
                            #maybe_ref
                        };
                    }
                }
                ArgType::Other => {
                    let target_type = if let syn::Type::Reference(ref reference) = **ty {
                        &reference.elem
                    } else {
                        ty
                    };
                    let binding_expr = match **ty {
                        syn::Type::Reference(_) => quote!(inner),
                        _ => quote!(inner.clone()),
                    };
                    quote! {
                        let #arg = {
                            let param_ref = ::pico::macro_fns::get_param(#db_arg, derived_node_id.params[#i])
                                .expect("param should exist. This is indicative of a bug in Pico.");
                            let inner = param_ref.downcast_ref::<#target_type>()
                                .expect("param type must be correct. This is indicative of a bug in Pico.");
                            #binding_expr
                        };
                    }
                }
            }
        });

    let output = quote! {
        #(#attrs)*
        #vis #new_sig {
            let mut param_ids = ::pico::macro_fns::init_param_vec();
            #(
                #param_ids_blocks
            )*
            let derived_node_id = ::pico::DerivedNodeId::new(#fn_hash.into(), param_ids);
            ::pico::memo(#db_arg, derived_node_id, ::pico::InnerFn::new(|#db_arg, derived_node_id| {
                #(
                    #extract_parameters
                )*
                let value: #return_type = (|| #block)();
                Box::new(value)
            }));
            #return_expr
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
    MemoRef,
    Other,
}

impl ArgType {
    pub fn parse(ty: &syn::Type) -> Self {
        if type_is(ty, "SourceId") {
            return ArgType::Source;
        }
        if type_is(ty, "MemoRef") {
            return ArgType::MemoRef;
        }
        ArgType::Other
    }
}

fn type_is(ty: &syn::Type, target: &'static str) -> bool {
    let inner = if let syn::Type::Reference(r) = ty {
        &*r.elem
    } else {
        ty
    };

    if let syn::Type::Path(type_path) = inner {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == target;
        }
    }
    false
}

fn get_fn_lifetime(sig: &mut Signature) -> Lifetime {
    let lt = sig.generics.params.iter().find_map(|param| {
        if let GenericParam::Lifetime(lt) = param {
            Some(lt.lifetime.clone())
        } else {
            None
        }
    });
    if lt.is_none() {
        sig.generics.params.push(parse_quote!('db));
    }
    lt.unwrap_or(parse_quote!('db))
}

fn check_db_lifetime(arg: &mut FnArg, lifetime: Lifetime) -> Result<(), syn::Error> {
    if let FnArg::Typed(PatType { ty, .. }) = arg {
        if let syn::Type::Reference(ref mut reference) = **ty {
            if let Some(db_lifetime) = &reference.lifetime {
                if db_lifetime != &lifetime {
                    return Err(Error::new_spanned(
                        ty,
                        format!("Expected lifetime {lifetime}"),
                    ));
                }
            } else {
                reference.lifetime = Some(lifetime);
            }
        } else {
            return Err(Error::new_spanned(ty, "Expected a reference"));
        }
    }
    Ok(())
}

fn parse_macro_args(args: TokenStream, sig: &Signature) -> Result<MemoArgs, syn::Error> {
    let attr_args =
        NestedMeta::parse_meta_list(args.into()).map_err(|e| Error::new_spanned(sig, e))?;
    MemoArgs::from_list(&attr_args).map_err(|e| e.with_span(&sig).into())
}
