use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::Signature;
use syn::{
    parse_macro_input, parse_quote, Error, FnArg, GenericParam, ItemFn, PatType, ReturnType,
};

#[derive(Debug, FromMeta)]
struct MemoArgs {
    #[darling(default)]
    reference: bool,
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
            "Memoized function must have at least one argument (&mut Database)",
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

    let unpacked_args = sig.inputs.iter().skip(1).map(|arg| match arg {
        FnArg::Typed(PatType { pat, .. }) => pat.clone(),
        _ => unreachable!(),
    });

    let get_ref_types = argument_types.clone().map(|ty| match **ty {
        syn::Type::Reference(ref ref_type) => ref_type.elem.clone(),
        _ => ty.clone(),
    });

    let reconstructed_args = argument_types
        .clone()
        .zip(unpacked_args.clone())
        .map(|(ty, arg)| {
            match **ty {
                syn::Type::Reference(_) => quote!(#arg),
                _ => quote!(#arg.clone()), // Clone
            }
        });

    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return Error::new_spanned(&sig, e).to_compile_error().into(),
    };

    let args_ = match MemoArgs::from_list(&attr_args) {
        Ok(parsed) => parsed,
        Err(e) => return e.with_span(&sig).write_errors().into(),
    };

    let return_type = match &sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        ReturnType::Default => parse_quote!(()),
    };

    let mut return_expr = quote! {
        #db_arg
            .storage()
            .get_derived_node(derived_node_id)
            .expect("derived node must exist. This is indicative of a bug in Pico.")
            .value
            .as_any()
            .downcast_ref::<#return_type>()
            .expect("unexpected return type. This is indicative of a bug in Pico.")
    };

    let mut new_sig = sig.clone();

    if args_.reference {
        let lifetime = new_sig.generics.params.iter().find_map(|param| {
            if let GenericParam::Lifetime(lt) = param {
                Some(&lt.lifetime)
            } else {
                None
            }
        });

        if let Some(lt) = lifetime {
            new_sig.output =
                ReturnType::Type(parse_quote!(->), Box::new(parse_quote!(&#lt #return_type)));
        } else {
            new_sig.generics.params.push(parse_quote!('db));
            if let FnArg::Typed(PatType { ty, .. }) = &mut new_sig.inputs[0] {
                if let syn::Type::Reference(ref mut reference) = **ty {
                    reference.lifetime = Some(parse_quote!('db));
                } else {
                    return Error::new_spanned(ty, "Expected a mutable reference type")
                        .to_compile_error()
                        .into();
                }
            }
            new_sig.output =
                ReturnType::Type(parse_quote!(->), Box::new(parse_quote!(&'db #return_type)));
        }
    } else {
        return_expr = quote! { #return_expr.clone() };
    }

    let unpacked_args = other_args.clone();
    let inner_args = other_args.clone();
    let output = quote! {
        #(#attrs)*
        #vis #new_sig {
            use ::pico_core::database::Database;
            let param_id = ::pico_core::params::ParamId::intern(#db_arg, (#(#other_args.clone(),)*));
            let derived_node_id = ::pico_core::node::DerivedNodeId::new(#fn_hash.into(), param_id);
            ::pico::memo::memo(#db_arg, derived_node_id, |#db_arg, param_id| {
                let param_ref = #db_arg
                    .storage()
                    .get_param(param_id)
                    .expect("param should exist. This is indicative of a bug in Pico.");
                let (#(#unpacked_args,)*) = {
                    let (#(#inner_args,)*) = param_ref
                        .downcast_ref::<(#(#get_ref_types,)*)>()
                        .expect("param type must to be correct. This is indicative of a bug in Pico.");
                    (
                        #(#reconstructed_args,)*
                    )
                };
                let value: #return_type = (|| #block)();
                Box::new(value)
            });
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
