use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, PatType, ReturnType};

pub(crate) fn memo(_args: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        sig,
        vis,
        block,
        attrs,
    } = parse_macro_input!(item as ItemFn);

    let fn_name = sig.ident.clone().to_string();

    let inputs = sig.inputs.iter().collect::<Vec<_>>();

    if inputs.is_empty() {
        return syn::Error::new_spanned(&sig, "Function must have at least one argument")
            .to_compile_error()
            .into();
    }

    let db_arg = match &inputs[0] {
        FnArg::Typed(PatType { pat, .. }) => pat.clone(),
        _ => {
            return syn::Error::new_spanned(inputs[0], "Expected a typed argument")
                .to_compile_error()
                .into()
        }
    };

    let other_args = inputs.iter().skip(1).map(|arg| match arg {
        FnArg::Typed(PatType { pat, .. }) => pat,
        _ => unreachable!(),
    });

    let argument_types = inputs.iter().skip(1).map(|arg| match arg {
        FnArg::Typed(PatType { ty, .. }) => ty,
        _ => unreachable!(),
    });

    let return_type = match &sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        ReturnType::Default => {
            return syn::Error::new_spanned(
                &sig.output,
                "Memoized function must have a return type",
            )
            .to_compile_error()
            .into()
        }
    };

    let (return_expr, return_type) = if let syn::Type::Reference(type_ref) = *return_type {
        let base_type = type_ref.elem;
        (
            quote! {
                #db_arg
                    .values
                    .get(&node_id)
                    .expect("value should exist. This is indicative of a bug in Isograph.")
                    .as_any()
                    .downcast_ref::<#base_type>()
                    .expect("unexpected return type. This is indicative of a bug in Isograph.")
            },
            base_type,
        )
    } else {
        (
            quote! {
                #db_arg
                    .values
                    .get(&node_id)
                    .expect("value should exist. This is indicative of a bug in Isograph.")
                    .as_any()
                    .downcast_ref::<#return_type>()
                    .expect("unexpected return type. This is indicative of a bug in Isograph.")
                    .clone()
            },
            return_type,
        )
    };

    let output = if inputs.len() == 2 {
        let arg = other_args
            .clone()
            .next()
            .expect("second argument should exist. This is indicative of a bug in Isograph.");
        let arg_type = argument_types
            .clone()
            .next()
            .expect("second argument should exist. This is indicative of a bug in Isograph.");
        quote! {
            #(#attrs)*
            #vis #sig {
                let param_id = pico::params::param_id(#db_arg, #arg);
                let node_id = #db_arg.memo(#fn_name, param_id, |#db_arg, param_id| {
                    let #arg = #db_arg
                        .params
                        .get(&param_id)
                        .expect("param should exists. This is indicative of a bug in Isograph.")
                        .downcast_ref::<#arg_type>()
                        .expect("unexpected argument type. This is indicative of a bug in Isograph.")
                        .clone();
                    let value: #return_type = (|| #block)();
                    Box::new(value)
                });
                #return_expr
            }
        }
    } else {
        let unpacked_args = other_args.clone();
        quote! {
            #(#attrs)*
            #vis #sig {
                let param_id = pico::params::param_id(#db_arg, (#(#other_args.clone(),)*));
                let node_id = #db_arg.memo(#fn_name, param_id, |#db_arg, param_id| {
                    let (#(#unpacked_args,)*) = #db_arg
                        .params
                        .get(&param_id)
                        .expect("param should exists. This is indicative of a bug in Isograph.")
                        .downcast_ref::<(#(#argument_types),*)>()
                        .expect("unexpected argument type. This is indicative of a bug in Isograph.")
                        .clone();
                    let value: #return_type = (|| #block)();
                    Box::new(value)
                });
                #return_expr
            }
        }
    };

    output.into()
}
