# `#[memo]` on trait method signatures

`#[memo]` currently parses `ItemFn`, so it requires a body. A trait method `fn extract_iso_literals(...) -> T;` fails to parse. Origin of a memoized trait method: isograph `CompilationProfile` in `crates/isograph_schema/src/compilation_profile.rs`, implemented in `crates/graphql_network_protocol/src/graphql_network_protocol.rs`. The trait writes `&T`. The impl writes `T` with `#[memo]`. Delta: `#[memo]` on the trait method rewrites `T` to `&T` the same way the impl rewrite does, so both sides write `T`.

```rust
// from crates/isograph_schema/src/compilation_profile.rs
    fn deprecated_parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> &DiagnosticResult<(
        WithNonFatalDiagnostics<DeprecatedParseTypeSystemOutcome<Self>>,
        // TODO just seems awkward that we return fetchable types
        BTreeMap<EntityName, RootOperationName>,
    )>;

    fn parse_nested_data_model_schema(db: &IsographDatabase<Self>) -> &NestedDataModelSchema<Self>;
```

```rust
// from crates/graphql_network_protocol/src/graphql_network_protocol.rs
    #[memo]
    fn parse_nested_data_model_schema(
        db: &IsographDatabase<Self>,
    ) -> isograph_schema::NestedDataModelSchema<Self> {
        parse_nested_schema(db)
    }
```

i2's first consumer is `HostLanguage::extract_iso_literals` in extract-iso-literals-from-file.md.

Two shippable changes: the macro accepts a signature with no body, then the HostLanguage docs write `#[memo]` on the trait method.

## What the user does

No user-facing change. Authors of a memoized trait method write `#[memo]` on the trait method and on the impl method. Both write return type `T`. Callers still receive `&T` (or `MemoRef<T>` for `#[memo(raw)]`). Tests call a trait method through the implementing type and assert the same reuse as a free `#[memo]` function.

## Change 1: `#[memo]` on `fn ...;`

`memo_macro` parses `MemoInput` instead of `ItemFn`. A semicolon is a trait-method signature. A block is a free function, an impl method, or a trait method with a default body. The return-type rewrite is `apply_return_rewrite`. A body still generates the current memo wrapper. No body emits the rewritten signature and a semicolon.

`pico_macros` depends on `prelude`, same as `resolve_position_macros`.

```rust
// from crates/pico_macros/Cargo.toml
prelude = { path = "../prelude" }
```

```rust
// from crates/pico_macros/src/memo_macro.rs
use prelude::Postfix;
use syn::{
    Attribute, Block, Error, FnArg, GenericParam, Lifetime, LifetimeParam, PatType, Receiver,
    ReturnType, Signature, Visibility,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote, token,
};

struct MemoInput {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    body: Option<Block>,
}

impl Parse for MemoInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;
        if input.peek(token::Semi) {
            input.parse::<token::Semi>()?;
            MemoInput {
                attrs,
                vis,
                sig,
                body: None,
            }
            .wrap_ok()
        } else {
            let block = input.parse()?;
            MemoInput {
                attrs,
                vis,
                sig,
                body: block.wrap_some(),
            }
            .wrap_ok()
        }
    }
}
```

`body: None` is the trait method with no default. `body: Some` is every case that already works. `vis` is inherited on trait methods and on trait impl methods.

```rust
// from crates/pico_macros/src/memo_macro.rs
#[derive(Default, deluxe::ParseMetaItem)]
#[deluxe(default)]
struct LegacyMemoArgs {
    #[deluxe(default)]
    raw: bool,
}

pub(crate) fn memo_macro(args: TokenStream, item: TokenStream) -> TokenStream {
    let MemoInput {
        attrs,
        vis,
        mut sig,
        body,
    } = parse_macro_input!(item as MemoInput);

    let LegacyMemoArgs { raw } = match deluxe::parse::<LegacyMemoArgs>(args) {
        Ok(args) => args,
        Err(err) => return err.into_compile_error().to(),
    };

    if sig.inputs.is_empty() {
        return Error::new_spanned(
            &sig,
            "Memoized function must have at least one argument (&Database)",
        )
        .to_compile_error()
        .to();
    }

    if let Some(err) = sig.inputs.iter().find_map(|arg| match arg {
        FnArg::Receiver(receiver) => reject_receiver(receiver).wrap_some(),
        FnArg::Typed(_) => None,
    }) {
        return err.to_compile_error().to();
    }

    let fn_hash = hash(&sig);

    let written_return_type = match apply_return_rewrite(&mut sig, raw) {
        Ok(ty) => ty,
        Err(err) => return err.to_compile_error().to(),
    };

    match body {
        None => quote! {
            #(#attrs)*
            #vis #sig;
        }
        .to(),
        Some(block) => memoized_fn(attrs, vis, sig, block, written_return_type, fn_hash, raw),
    }
}

fn reject_receiver(receiver: &Receiver) -> Error {
    Error::new_spanned(
        receiver,
        "Memoized function cannot take self. First argument must be a reference to the database.",
    )
}
```

`memo_macro` calls `memoized_fn` when `body` is `Some`. It returns the proc-macro `TokenStream` of the wrapper function. Origin of the body: the `output` quote in `memo_macro` today. Deltas: `sig` is already rewritten (`new_sig` is `sig`); `return_type` is `written_return_type`; `FnArg::Receiver(_) => unreachable!()` is `filter_map` of `FnArg::Typed` only; `fn_hash` and `raw` are arguments (`hash` still runs on the signature before rewrite).

```rust
// from crates/pico_macros/src/memo_macro.rs
fn memoized_fn(
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    block: Block,
    written_return_type: syn::Type,
    fn_hash: u64,
    raw: bool,
) -> TokenStream {
    let db_arg = match &sig.inputs[0] {
        FnArg::Typed(PatType { pat, .. }) => pat,
        FnArg::Receiver(receiver) => {
            return reject_receiver(receiver).to_compile_error().to();
        }
    };

    let args = sig.inputs.iter().skip(1).filter_map(|arg| match arg {
        FnArg::Typed(PatType { pat, ty, .. }) => (pat, ty).wrap_some(),
        FnArg::Receiver(_) => None,
    });

    let param_ids_blocks = args.clone().map(|(arg, ty)| match ArgType::parse(ty) {
        ArgType::Source | ArgType::MemoRef => {
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

    let memo_return_expr = if raw {
        quote!(memo_ref)
    } else {
        quote!(memo_ref.lookup(#db_arg))
    };

    let extract_parameters = args.enumerate().map(|(i, (arg, ty))| match ArgType::parse(ty) {
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
        ArgType::MemoRef => {
            let binding_expr = match **ty {
                syn::Type::Reference(_) => quote!(&::pico::MemoRef::new(param_id.into())),
                _ => quote!(::pico::MemoRef::new(param_id.into())),
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
    });

    let fn_name = sig.ident.to_string();
    quote! {
        #(#attrs)*
        #vis #sig {
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
                    let value: #written_return_type = (|| #block)();
                    Some(Box::new(value))
                })
            );
            debug_assert!(
                !matches!(did_recalculate, pico::DidRecalculate::Error),
                "Unexpected memo result. This is indicative of a bug in Pico."
            );
            let memo_ref = ::pico::MemoRef::new(derived_node_id);
            #memo_return_expr
        }
    }
    .to()
}
```

```rust
// from crates/pico_macros/src/memo_macro.rs
fn apply_return_rewrite(sig: &mut Signature, raw: bool) -> Result<syn::Type, Error> {
    let written_return_type = match &sig.output {
        ReturnType::Type(_, ty) => ty.as_ref().clone(),
        ReturnType::Default => parse_quote!(()),
    };

    let emitted_return_type = if raw {
        parse_quote!(::pico::MemoRef<#written_return_type>)
    } else {
        let db_lifetime = match sig.inputs.iter_mut().next() {
            Some(FnArg::Typed(PatType { ty, .. })) => match ty.as_mut() {
                syn::Type::Reference(type_reference) => {
                    ensure_db_lifetime(&mut sig.generics, type_reference)
                }
                other => {
                    return Error::new_spanned(
                        other,
                        "First argument to a memoized function must be a reference to the database.",
                    )
                    .wrap_err();
                }
            },
            Some(FnArg::Receiver(receiver)) => return reject_receiver(receiver).wrap_err(),
            None => {
                return Error::new_spanned(
                    &sig.ident,
                    "Memoized function must have at least one argument (&Database)",
                )
                .wrap_err();
            }
        };
        parse_quote!(&#db_lifetime #written_return_type)
    };

    sig.output = ReturnType::Type(parse_quote!(->), emitted_return_type.boxed());
    written_return_type.wrap_ok()
}
```

`memo_macro` already rejected empty inputs and any `Receiver`. `ensure_db_lifetime`, `hash`, `ArgType`, and `type_is` stay.

A trait method expands to the rewritten signature:

```rust
// generated by crates/pico_macros/src/memo_macro.rs
fn extract_iso_literals<'db>(
    db: &'db IsographState,
    path: PathBuf,
) -> &'db Option<Vec<IsoLiteralExtraction<Self>>>;
```

```rust
// generated by crates/pico_macros/src/memo_macro.rs
fn first_letter<'db>(
    db: &'db TestDatabase,
    input_id: SourceId<Input>,
) -> ::pico::MemoRef<char>;
```

The second is `#[memo(raw)]`. No `'db` is added for `raw`, same as today.

Same `raw` on both sides. `#[memo]` on the trait and `#[memo(raw)]` on the impl (or the reverse) is `T` vs `MemoRef<T>` after rewrite, which does not impl the trait.

`#[memo]` only on the trait and a plain impl returning `T` is `&T` vs `T`, which does not impl the trait. `#[memo]` only on the impl, trait written as `&T` by hand, still compiles. That is the isograph shape.

A `Receiver` is a compile error. Today `unreachable!()` panics the proc macro.

### Tests

`crates/pico/tests/trait_method.rs`, same harness as `crates/pico/tests/basic.rs` (`TestDatabase`, `Input`, an `AtomicUsize` counter).

```rust
// from crates/pico/tests/trait_method.rs
trait FirstLetter {
    #[memo]
    fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char;
}

struct Impl;

impl FirstLetter for Impl {
    #[memo]
    fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
        FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let input = db.get(input_id);
        input.value.chars().next().unwrap()
    }
}
```

- `Impl::first_letter(&db, id)` is `'a`. Counter is 1. Call again with the same source: counter is 1. `db.set` a different `value` for the same key: `'q'`, counter is 2.
- `<Impl as FirstLetter>::first_letter(&db, id)` is the same slot: a call through the trait after `Impl::first_letter`, no extra increment.
- Trait `FirstLetterDefault` with a default `#[memo]` body that increments the counter. `struct DefaultImpl;` `impl FirstLetterDefault for DefaultImpl {}`. Same reuse assertions as the first test, calling `DefaultImpl::first_letter`.
- Trait `FirstLetterRaw` and impl both `#[memo(raw)]`. `ImplRaw::first_letter` is a `MemoRef<char>`. `lookup` is `'a`. Counter is 1 on the first call and on a second call with the same source.
- Trait `Mark` with `fn mark(db: &TestDatabase);`. The impl body increments the counter. Two calls, counter is 1.

Do not add a production function only the tests call.

## Change 2: HostLanguage and pico.md write `#[memo]` on the trait method

extract-iso-literals-from-file.md requires this file. The trait writes `T` with `#[memo]`. The impl is already `T` with `#[memo]`.

```rust
// from crates/isograph_compiler/src/host_language.rs
use pico_macros::memo;

pub trait HostLanguage: Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + std::fmt::Debug + 'static;

    #[memo]
    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>>;
}
```

Before, in extract-iso-literals-from-file.md:

```rust
    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
```

The opening paragraph of extract-iso-literals-from-file.md: drop "The trait writes `&T`. The impl writes `T` with `#[memo]`." Both write `T` with `#[memo]`. First argument is `&Database`. There is no `&self`. Origin of that shape is still isograph `CompilationProfile`.

pico.md Memo section:

```rust
// from docs-website/docs/design-docs/pico.md
trait HostLanguage: Sized + 'static {
    #[memo]
    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>>;
}

impl HostLanguage for TypeScriptHostLanguage {
    #[memo]
    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
        let contents = db.get(source_id).contents.as_str();
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| { /* IsoLiteralExtraction; skip comments and empty backticks */ })
            .collect::<Vec<_>>()
            .wrap_some()
    }
}
```

Replace "The trait writes the lookup type `&Option<Vec<IsoLiteralExtraction<Self>>>`. `#[memo]` is on the impl. pico rewrites the impl return to `&T`." with: both write `T`. `#[memo]` rewrites each to `&T`. First argument is `&Database`. There is no `&self`. Origin of that shape: isograph `CompilationProfile` methods in `graphql_network_protocol.rs`.
