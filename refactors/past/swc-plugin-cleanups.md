# swc plugin cleanups

Four self-contained changes to `crates/swc_isograph_plugin/src/lib.rs`. Each removes a branch or a failure path that exists below the point where its outcome is actually decided: error plumbing that never succeeds or never fails, a panic on a keyword the regex already vetted, a `SyntaxContext` recomputed at every use site, and two duplicated `fold_expr` arms.

The existing fixture tests (`tests/transform.rs` over `tests/fixtures/base/*` and `tests/fixtures/errors/*`) gate every change. No fixture changes; the plugin's output is byte-identical throughout.

## Change 1: error plumbing that matches the flow

`show_error` returns a `Result` that is always `Err`, and its one caller discards it. `path_for_artifact` returns a `Result` that is always `Ok`, and its one caller unwraps it with an `expect`. Both signatures narrow to what actually happens, and `anyhow`, used only for the always-`Err`, leaves the crate.

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
fn show_error(span: Span, err: &IsographTransformError) -> Result<(), anyhow::Error> {
    let msg = IsographTransformError::to_string(err);

    HANDLER.with(|handler| {
        handler.struct_span_err(span, &msg).emit();
    });
    bail!(msg)
}
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
fn show_error(span: Span, err: &IsographTransformError) {
    HANDLER.with(|handler| {
        handler.struct_span_err(span, &err.to_string()).emit();
    });
}
```

The two call sites drop their `let _ =`:

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
Err(err) => {
    show_error(*span, &err);
    // On error, we keep the same expression and fail showing the error
    return expr;
}
```

`path_for_artifact` loses its `Result` and `handle_valid_isograph_entrypoint_literal` loses its `expect`:

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
    fn path_for_artifact(
        &self,
        real_filepath: &Path,
        config: &IsographProjectConfig,
        root_dir: &Path,
    ) -> Result<PathBuf, IsographTransformError> {
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
    fn path_for_artifact(
        &self,
        real_filepath: &Path,
        config: &IsographProjectConfig,
        root_dir: &Path,
    ) -> PathBuf {
```

The body's final `Ok(file_to_artifact)` becomes `file_to_artifact`; nothing else in the body changes.

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
        let file_to_artifact = iso_template_literal
            .path_for_artifact(self.filepath, self.config, self.root_dir)
            .expect("Failed to get path for artifact.");
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
        let file_to_artifact =
            iso_template_literal.path_for_artifact(self.filepath, self.config, self.root_dir);
```

`use anyhow::{Result, bail};` is deleted from `lib.rs`, and `anyhow = { workspace = true }` is deleted from `crates/swc_isograph_plugin/Cargo.toml`; nothing else in the crate uses it.

## Change 2: keyword parsing without a panic

`ArtifactType::from(&str)` panics on any keyword outside `entrypoint`/`field`/`pointer`, relying on the `OPERATION_REGEX` alternation to have already excluded that case. The conversion becomes a total function, and the impossible case feeds the diagnostic that already exists for a bad keyword, so a future disagreement between the regex and the match surfaces to the user as `InvalidIsoKeyword` instead of crashing the plugin.

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
impl From<&str> for ArtifactType {
    fn from(s: &str) -> Self {
        match s {
            "entrypoint" => Self::Entrypoint,
            "field" | "pointer" => Self::Field,
            _ => {
                panic!("Regex will not produce this case. This is indicative of a bug in Isograph.")
            }
        }
    }
}
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
impl ArtifactType {
    fn from_keyword(keyword: &str) -> Option<ArtifactType> {
        match keyword {
            "entrypoint" => Some(ArtifactType::Entrypoint),
            "field" | "pointer" => Some(ArtifactType::Field),
            _ => None,
        }
    }
}
```

The one construction site threads the `Option` into its existing `ok_or`:

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
            return OPERATION_REGEX
                .captures_iter(first.raw.trim())
                .next()
                .map(|capture_group| {
                    debug!("capture_group {:?}", capture_group);
                    ValidIsographTemplateLiteral {
                        artifact_type: ArtifactType::from(&capture_group[1]),
                        field_type: capture_group[2].to_string(),
                        field_name: capture_group[3].to_string(),
                    }
                })
                .ok_or(IsographTransformError::InvalidIsoKeyword);
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
            return OPERATION_REGEX
                .captures_iter(first.raw.trim())
                .next()
                .and_then(|capture_group| {
                    debug!("capture_group {:?}", capture_group);
                    let artifact_type = ArtifactType::from_keyword(&capture_group[1])?;
                    Some(ValidIsographTemplateLiteral {
                        artifact_type,
                        field_type: capture_group[2].to_string(),
                        field_name: capture_group[3].to_string(),
                    })
                })
                .ok_or(IsographTransformError::InvalidIsoKeyword);
```

## Change 3: one `SyntaxContext`, computed at entry

The visitor carries `unresolved_mark: Option<Mark>`, and three separate sites each rebuild the same context with `.map(|m| SyntaxContext::empty().apply_mark(m)).unwrap_or_default()`. The decision moves to the two entry points: the wasm entry applies the mark once, the test entry passes `SyntaxContext::empty()` (what `None` produced), and everything below carries a plain `SyntaxContext`.

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
pub fn compile_iso_literal_visitor<'a>(
    config: &'a IsographProjectConfig,
    filepath: &'a Path,
    root_dir: &'a Path,
    unresolved_mark: Option<Mark>,
) -> impl Pass + 'a {
    fold_pass(IsoLiteralCompilerVisitor {
        config,
        filepath,
        unresolved_mark,
        imports: vec![],
        root_dir,
    })
}
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
pub fn compile_iso_literal_visitor<'a>(
    config: &'a IsographProjectConfig,
    filepath: &'a Path,
    root_dir: &'a Path,
    unresolved_ctxt: SyntaxContext,
) -> impl Pass + 'a {
    fold_pass(IsoLiteralCompilerVisitor {
        config,
        filepath,
        unresolved_ctxt,
        imports: vec![],
        root_dir,
    })
}
```

The wasm entry applies the mark at the one place a mark exists:

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
    let isograph = compile_iso_literal_visitor(
        &config,
        path,
        root_dir.as_path(),
        Some(metadata.unresolved_mark),
    );
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
    let isograph = compile_iso_literal_visitor(
        &config,
        path,
        root_dir.as_path(),
        SyntaxContext::empty().apply_mark(metadata.unresolved_mark),
    );
```

Every struct and helper that carried the `Option<Mark>` carries the context instead, and the three `.map(...).unwrap_or_default()` sites become plain field reads:

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
#[derive(Debug, Clone)]
struct IsographImport {
    path: Atom,
    item: Atom,
    unresolved_ctxt: SyntaxContext,
}

impl IsographImport {
    fn as_module_item(&self) -> ModuleItem {
        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: Default::default(),
            specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                span: Default::default(),
                local: Ident {
                    ctxt: self.unresolved_ctxt,
                    span: DUMMY_SP,
                    sym: self.item.clone(),
                    optional: false,
                },
            })],
            src: Box::new(self.path.clone().into()),
            type_only: false,
            with: None,
            phase: Default::default(),
        }))
    }
}

fn build_ident_expr_for_hoisted_import(
    ident_name: &str,
    unresolved_ctxt: SyntaxContext,
) -> Expr {
    Expr::Ident(Ident {
        span: DUMMY_SP,
        sym: ident_name.into(),
        optional: false,
        ctxt: unresolved_ctxt,
    })
}
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
    fn build_require_expr_from_path(path: &str, unresolved_ctxt: SyntaxContext) -> Expr {
        Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: quote_ident!(unresolved_ctxt, "require").as_callee(),
                args: vec![
                    Lit::Str(Str {
                        span: Default::default(),
                        value: Atom::from(path),
                        raw: None,
                    })
                    .as_arg(),
                ],
                type_args: None,
                ctxt: SyntaxContext::empty(),
            })),
            prop: MemberProp::Ident(IdentName {
                sym: "default".into(),
                span: DUMMY_SP,
            }),
        })
    }
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
#[derive(Debug, Clone)]
struct IsoLiteralCompilerVisitor<'a> {
    root_dir: &'a Path,
    config: &'a IsographProjectConfig,
    filepath: &'a Path,
    imports: Vec<IsographImport>,
    unresolved_ctxt: SyntaxContext,
}
```

In `handle_valid_isograph_entrypoint_literal`, the three uses become `self.unresolved_ctxt`: the `build_require_expr_from_path` argument, the `IsographImport { unresolved_ctxt: self.unresolved_ctxt, .. }` field, and the `build_ident_expr_for_hoisted_import` argument.

`Mark` leaves the `use swc_core::common::...` list in `lib.rs`; nothing names the type after this change.

The two test entry points pass the empty context:

```rust
// from crates/swc_isograph_plugin/tests/transform.rs (after)
use swc_core::common::SyntaxContext;
```

```rust
// from crates/swc_isograph_plugin/tests/transform.rs (after)
        &|_| {
            compile_iso_literal_visitor(
                &config,
                Path::new(&filename),
                Path::new(root_dir),
                SyntaxContext::empty(),
            )
        },
```

## Change 4: one arm for both call shapes

`fold_expr` matches `iso(...)` and `iso(...)(...)` in two arms whose bodies are copies of each other. Recognizing the call becomes its own function, and the compile-or-report body exists once.

```rust
// from crates/swc_isograph_plugin/src/lib.rs (before)
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        if let Expr::Call(CallExpr {
            callee: Callee::Expr(callee),
            args,
            span,
            ..
        }) = &expr
        {
            match &**callee {
                Expr::Ident(ident) => {
                    if ident.sym == "iso" {
                        match self.compile_iso_call_statement(args, None) {
                            Ok(build_expr) => {
                                // might have `iso` functions inside the build expr
                                let build_expr = build_expr.fold_children_with(self);
                                return build_expr;
                            }
                            Err(err) => {
                                show_error(*span, &err);
                                // On error, we keep the same expression and fail showing the error
                                return expr;
                            }
                        }
                    }
                }
                Expr::Call(CallExpr {
                    callee: Callee::Expr(child_callee),
                    args: child_args,
                    span: child_span,
                    ..
                }) => {
                    if let Expr::Ident(ident) = &**child_callee
                        && ident.sym == "iso"
                    {
                        match self.compile_iso_call_statement(child_args, Some(args)) {
                            Ok(build_expr) => {
                                // might have `iso` functions inside the build expr
                                let build_expr = build_expr.fold_children_with(self);
                                return build_expr;
                            }
                            Err(err) => {
                                show_error(*child_span, &err);
                                // On error, we keep the same expression and fail showing the error
                                return expr;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        expr.fold_children_with(self)
    }
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
/// An `iso` invocation as it appears in the tree: `iso(iso_args)` bare, or
/// `iso(iso_args)(fn_args)` immediately called.
struct IsoCall<'a> {
    iso_args: &'a [ExprOrSpread],
    fn_args: Option<&'a [ExprOrSpread]>,
    /// The span of the `iso(...)` call itself, where errors are reported.
    span: Span,
}

fn iso_call(expr: &Expr) -> Option<IsoCall<'_>> {
    let Expr::Call(CallExpr {
        callee: Callee::Expr(callee),
        args,
        span,
        ..
    }) = expr
    else {
        return None;
    };
    match &**callee {
        Expr::Ident(ident) if ident.sym == "iso" => Some(IsoCall {
            iso_args: args,
            fn_args: None,
            span: *span,
        }),
        Expr::Call(CallExpr {
            callee: Callee::Expr(inner_callee),
            args: iso_args,
            span: iso_span,
            ..
        }) => match &**inner_callee {
            Expr::Ident(ident) if ident.sym == "iso" => Some(IsoCall {
                iso_args,
                fn_args: Some(args),
                span: *iso_span,
            }),
            _ => None,
        },
        _ => None,
    }
}
```

```rust
// from crates/swc_isograph_plugin/src/lib.rs (after)
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        if let Some(IsoCall {
            iso_args,
            fn_args,
            span,
        }) = iso_call(&expr)
        {
            return match self.compile_iso_call_statement(iso_args, fn_args) {
                Ok(build_expr) => {
                    // might have `iso` functions inside the build expr
                    build_expr.fold_children_with(self)
                }
                Err(err) => {
                    show_error(span, &err);
                    // On error, we keep the same expression and fail showing the error
                    expr
                }
            };
        }

        expr.fold_children_with(self)
    }
```

The error span is unchanged: for the bare shape it is the `iso(...)` call's own span, and for the called shape it is the inner `iso(...)` call's span, exactly the `span` and `child_span` the two arms report today. An erroring call still returns the expression unfolded, so nested `iso` literals inside it stay untransformed, as today.

## Shipping order

The changes are independent; they land in the order written, one commit each.

## Landing checklist

- `cargo test -p swc_isograph_plugin` passes after each change.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
