use std::hash::{DefaultHasher, Hash, Hasher};

use darling::{Error as DarlingError, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse, parse_macro_input, parse_quote, visit_mut::VisitMut, Error, Expr, FnArg, Ident, ItemFn,
    Lit, Meta, Pat, PatIdent, PatType, ReturnType, Signature, Type,
};

#[derive(Debug)]
struct DbArg(pub Ident);

impl FromMeta for DbArg {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        match item {
            Meta::Path(path) => {
                // bare identifier
                if let Some(ident) = path.get_ident() {
                    Ok(DbArg(ident.clone()))
                } else {
                    Err(DarlingError::custom("Expected identifier").with_span(path))
                }
            }
            Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(expr_lit) => {
                    if let Lit::Str(litstr) = &expr_lit.lit {
                        let ident = Ident::new(&litstr.value(), litstr.span());
                        Ok(DbArg(ident))
                    } else {
                        Err(DarlingError::custom("Expected string literal")
                            .with_span(&expr_lit.lit))
                    }
                }
                Expr::Path(expr_path) => {
                    if let Some(segment) = expr_path.path.segments.last() {
                        Ok(DbArg(segment.ident.clone()))
                    } else {
                        Err(DarlingError::custom("Empty path for db").with_span(&expr_path.path))
                    }
                }
                other => {
                    Err(DarlingError::custom("Unsupported expression for db").with_span(other))
                }
            },
            _ => Err(DarlingError::custom("Unsupported meta for db").with_span(item)),
        }
    }
}

#[derive(Debug, FromMeta)]
#[darling(derive_syn_parse)]
struct MemoArgs {
    #[darling(default)]
    db: Option<DbArg>,
}

pub(crate) fn memo_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args_: MemoArgs = match parse(attr) {
        Ok(v) => v,
        Err(e) => {
            return e.to_compile_error().into();
        }
    };

    let ItemFn {
        mut sig,
        vis,
        mut block,
        attrs,
    } = parse_macro_input!(item as ItemFn);

    let fn_hash = hash(&sig);

    if sig.inputs.is_empty() {
        return Error::new_spanned(
            &sig,
            "Memoized function must have at least one argument (db or &self)",
        )
        .to_compile_error()
        .into();
    }

    let db_pos = get_db_position(&sig, &args_);

    let (db_arg, closure_db_arg) = match &sig.inputs[db_pos] {
        FnArg::Receiver(rcv) => {
            if rcv.reference.is_none() {
                return Error::new_spanned(rcv, "Receiver must be a reference")
                    .to_compile_error()
                    .into();
            }
            if rcv.mutability.is_some() {
                return Error::new_spanned(rcv, "Receiver should not be mutable")
                    .to_compile_error()
                    .into();
            }
            (quote!(self), quote!(__self))
        }
        FnArg::Typed(PatType { pat, .. }) => {
            let tok = pat.to_token_stream();
            (tok.clone(), tok)
        }
    };

    let args = sig
        .inputs
        .iter()
        .cloned()
        .enumerate()
        .filter_map(|(i, arg)| if db_pos == i { None } else { Some(arg) })
        .map(|arg| match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => (pat, ty),
            // hack to transform `self`` to fake `__self: &Self`` argument and use it as regular parameter
            FnArg::Receiver(_) => {
                let pat_ident = Pat::Ident(PatIdent {
                    attrs: Vec::new(),
                    by_ref: None,
                    mutability: None,
                    ident: Ident::new("__self", Span::call_site()),
                    subpat: None,
                });
                (Box::new(pat_ident), Box::new(parse_quote!(&Self)))
            }
        });

    let param_ids_blocks = args.clone().map(|(arg, ty)| match ArgType::parse(&ty) {
        ArgType::Source | ArgType::MemoRef => {
            let param_arg = match *ty {
                Type::Reference(_) => quote!((*(#arg))),
                _ => quote!(#arg),
            };
            quote! {
                param_ids.push(#param_arg.into());
            }
        }
        ArgType::Receiver => {
            let intern_param = match *ty {
                Type::Reference(_) => {
                    quote!(::pico::macro_fns::intern_borrowed_param(#db_arg, self))
                }
                _ => unreachable!(),
            };
            quote! {
                let param_id = #intern_param;
                param_ids.push(param_id);
            }
        }
        ArgType::Other => {
            let intern_param = match *ty {
                Type::Reference(_) => {
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

    sig.output = ReturnType::Type(
        parse_quote!(->),
        Box::new(parse_quote!(::pico::MemoRef<#return_type>)),
    );

    let extract_parameters = args
        .enumerate()
        .map(|(i, (arg, ty))| {
            match ArgType::parse(&ty) {
                ArgType::Source => {
                    let binding_expr = match *ty {
                        Type::Reference(_) => quote!(&param_id.into()),
                        _ => quote!(param_id.into()),
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_id = derived_node_id.params[#i];
                            #binding_expr
                        };
                    }
                }
                ArgType::MemoRef => {
                    let binding_expr = match *ty {
                        Type::Reference(_) => quote!(&::pico::MemoRef::new(#db_arg, param_id.into())),
                        _ => quote!(::pico::MemoRef::new(#closure_db_arg, param_id.into())),
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_id = derived_node_id.params[#i];
                            #binding_expr
                        };
                    }
                }
                ArgType::Other | ArgType::Receiver => {
                    let (target_type, binding_expr) = match *ty {
                        Type::Reference(ref reference) => (&reference.elem, quote!(inner)),
                        _ => (&ty, quote!(inner.clone())),
                    };
                    quote! {
                        let #arg: #ty = {
                            let param_ref = ::pico::macro_fns::get_param(#closure_db_arg, derived_node_id.params[#i])?;
                            let inner = param_ref
                                .downcast_ref::<#target_type>()
                                .expect("Unexpected param type. This is indicative of a bug in Pico.");
                            #binding_expr
                        };
                    }
                }
            }
        });

    let mut replacer = IdentReplacer::new("self", "__self");
    replacer.visit_block_mut(&mut block);

    let output = quote! {
        #(#attrs)*
        #vis #sig {
            let mut param_ids = ::pico::macro_fns::init_param_vec();
            #(
                #param_ids_blocks
            )*
            let derived_node_id = ::pico::DerivedNodeId::new(#fn_hash.into(), param_ids);
            let did_recalculate = ::pico::execute_memoized_function(
                #db_arg,
                derived_node_id,
                ::pico::InnerFn::new(|#closure_db_arg, derived_node_id| {
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
    MemoRef,
    Receiver,
    Other,
}

impl ArgType {
    pub fn parse(ty: &Type) -> Self {
        if type_is(ty, "SourceId") {
            return ArgType::Source;
        }
        if type_is(ty, "MemoRef") {
            return ArgType::MemoRef;
        }
        if type_is(ty, "Self") {
            return ArgType::Receiver;
        }
        ArgType::Other
    }
}

fn type_is(ty: &Type, target: &'static str) -> bool {
    let inner = match ty {
        Type::Reference(r) => &*r.elem,
        _ => ty,
    };
    if let Type::Path(type_path) = inner {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == target;
        }
    }
    false
}

fn get_db_position(sig: &Signature, args: &MemoArgs) -> usize {
    args.db
        .as_ref()
        .and_then(|db_arg| {
            sig.inputs.iter().position(|arg| match arg {
                FnArg::Typed(PatType { pat, .. }) => {
                    matches!(&**pat, Pat::Ident(pi) if pi.ident == db_arg.0)
                }
                _ => false,
            })
        })
        .unwrap_or(0)
}

struct IdentReplacer {
    pub from: &'static str,
    pub to: &'static str,
}

impl IdentReplacer {
    pub fn new(from: &'static str, to: &'static str) -> Self {
        Self { from, to }
    }
}

impl VisitMut for IdentReplacer {
    fn visit_ident_mut(&mut self, ident: &mut Ident) {
        if ident == self.from {
            *ident = Ident::new(self.to, ident.span());
        }
    }
}
