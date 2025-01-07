use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
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

    let fn_name = sig.ident.clone().to_string();

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

    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return Error::new_spanned(&sig, e).to_compile_error().into(),
    };

    let args_ = match MemoArgs::from_list(&attr_args) {
        Ok(parsed) => parsed,
        Err(e) => return e.with_span(&sig).write_errors().into(),
    };

    let (return_type, default) = match &sig.output {
        ReturnType::Type(_, ty) => (ty.clone(), false),
        ReturnType::Default => (parse_quote!(()), true),
    };

    let mut return_expr = quote! {
        #db_arg.storage()
            .values()
            .get(&node_id)
            .expect("value should exist. This is indicative of a bug in Isograph.")
            .as_any()
            .downcast_ref::<#return_type>()
            .expect("unexpected return type. This is indicative of a bug in Isograph.")
    };

    let mut new_sig = sig.clone();

    if args_.reference && !default {
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
    let output = quote! {
        #(#attrs)*
        #vis #new_sig {
            use ::pico_core::{storage::Storage, container::Container, database::Database};
            let param_id = ::pico_core::params::ParamId::intern(#db_arg, (#(#other_args.clone(),)*));
            let node_id = ::pico::memo::memo(#db_arg, #fn_name, param_id, |#db_arg, param_id| {
                let (#(#unpacked_args,)*) = param_id.get::<(#(#argument_types,)*), _>(#db_arg)
                    .expect("parameter should exist. This is indicative of a bug in Isograph.");
                let value: #return_type = (|| #block)();
                Box::new(value)
            });
            #return_expr
        }
    };

    output.into()
}
