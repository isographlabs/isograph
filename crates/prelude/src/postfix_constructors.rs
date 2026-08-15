//! Clippy cannot ban `Ok`/`Err`/`Some` constructors (`disallowed-methods` is methods,
//! not enum variants). This walk is the enforcement.

use std::fs;
use std::path::{Path, PathBuf};

use crate::Postfix;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ImplItemFn, ItemFn, Macro, Pat, TraitItemFn};

enum CtorCheck {
    Lint,
    InWrapImpl,
}

const RUST_ROOTS: &[&str] = &[
    "crates/common_lang_types",
    "crates/isograph_cli",
    "crates/isograph_config",
    "crates/isograph_parser",
    "crates/prelude",
    "crates/resolve_position",
    "crates/resolve_position_macros",
    "crates/safe_peekable",
    "crates/scoped_stack",
    "crates/span",
    "crates/string_key_newtype",
    "crates/swc_isograph_plugin",
    "crates/tests",
    "crates/u64_newtypes",
];

#[test]
fn rust_sources_use_postfix_constructors() {
    let root = workspace_root();
    let mut hits = Vec::new();
    for rel in RUST_ROOTS {
        walk_rust(root.join(rel).reference(), &mut hits);
    }
    assert!(
        hits.is_empty(),
        "use postfix wrappers, not Ok/Err/Some, Box::new, vec![x], or .into():\n{}",
        hits.join("\n")
    );
}

#[test]
fn pending_docs_use_postfix_constructors() {
    let root = workspace_root();
    let mut hits = Vec::new();
    walk_markdown(root.join("refactors/pending").reference(), &mut hits);
    assert!(
        hits.is_empty(),
        "use postfix wrappers, not Ok/Err/Some, Box::new, vec![x], or .into():\n{}",
        hits.join("\n")
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/prelude sits under crates/")
        .parent()
        .expect("crates sits under the workspace root")
        .to_path_buf()
}

fn walk_rust(dir: &Path, hits: &mut Vec<String>) {
    let mut stack = dir.to_path_buf().wrap_vec();
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(dir.reference()) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(name, "target") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                lint_rust_file(path.reference(), hits);
            }
        }
    }
}

fn walk_markdown(dir: &Path, hits: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            lint_markdown_file(path.reference(), hits);
        }
    }
}

fn lint_rust_file(path: &Path, hits: &mut Vec<String>) {
    let src = match fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            hits.push(format!("{}: failed to read: {e}", path.display()));
            return;
        }
    };
    let file = match syn::parse_file(src.reference()) {
        Ok(file) => file,
        Err(e) => {
            hits.push(format!("{}: failed to parse: {e}", path.display()));
            return;
        }
    };
    let mut collector = Collector {
        path: path.display().to_string(),
        hits,
        ctor_check: CtorCheck::Lint,
    };
    collector.visit_file(file.reference());
}

fn lint_markdown_file(path: &Path, hits: &mut Vec<String>) {
    let src = match fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            hits.push(format!("{}: failed to read: {e}", path.display()));
            return;
        }
    };
    let display = path.display().to_string();
    let mut rest = src.as_str();
    let mut line = 1usize;
    while let Some(start) = rest.find("```") {
        line += rest[..start].bytes().filter(|&b| b == b'\n').count();
        rest = &rest[start + 3..];
        let nl = match rest.find('\n') {
            Some(i) => i,
            None => break,
        };
        let info = rest[..nl].trim();
        let lang = info
            .split(|c: char| c == ',' || c.is_whitespace())
            .next()
            .unwrap_or("");
        rest = &rest[nl + 1..];
        line += 1;
        let end = match rest.find("```") {
            Some(i) => i,
            None => break,
        };
        let body = &rest[..end];
        if matches!(lang, "rust" | "rs") {
            lint_rust_snippet(display.reference(), line, body, hits);
        }
        line += body.bytes().filter(|&b| b == b'\n').count();
        rest = &rest[end + 3..];
    }
}

fn lint_rust_snippet(path: &str, start_line: usize, body: &str, hits: &mut Vec<String>) {
    let before = hits.len();
    if let Ok(file) = syn::parse_file(body) {
        let mut collector = Collector {
            path: path.to_string(),
            hits,
            ctor_check: CtorCheck::Lint,
        };
        collector.visit_file(file.reference());
        shift_new_hits(hits, before, path, start_line.saturating_sub(1));
        return;
    }
    if let Ok(item) = syn::parse_str::<syn::Item>(body) {
        let mut collector = Collector {
            path: path.to_string(),
            hits,
            ctor_check: CtorCheck::Lint,
        };
        collector.visit_item(item.reference());
        shift_new_hits(hits, before, path, start_line.saturating_sub(1));
        return;
    }
    if let Ok(expr) = syn::parse_str::<Expr>(body) {
        let mut collector = Collector {
            path: path.to_string(),
            hits,
            ctor_check: CtorCheck::Lint,
        };
        collector.visit_expr(expr.reference());
        shift_new_hits(hits, before, path, start_line.saturating_sub(1));
        return;
    }
    let wrapped = format!("fn __postfix_wrap() {{\n{body}\n}}");
    if let Ok(file) = syn::parse_file(wrapped.reference()) {
        let mut collector = Collector {
            path: path.to_string(),
            hits,
            ctor_check: CtorCheck::Lint,
        };
        collector.visit_file(file.reference());
        shift_new_hits(hits, before, path, start_line.saturating_sub(1));
        return;
    }
    scan_snippet_lines(path, start_line, body, hits);
}

fn shift_new_hits(hits: &mut [String], before: usize, path: &str, delta: usize) {
    for hit in hits.iter_mut().skip(before) {
        if let Some(adjusted) = shift_hit_line(hit, path, delta) {
            *hit = adjusted;
        }
    }
}

fn shift_hit_line(hit: &str, path: &str, delta: usize) -> Option<String> {
    let rest = hit.strip_prefix(format!("{path}:").reference())?;
    let (line, msg) = rest.split_once(": ")?;
    let n: usize = line.parse().ok()?;
    format!("{path}:{}: {msg}", n + delta).wrap_some()
}

fn scan_snippet_lines(path: &str, start_line: usize, body: &str, hits: &mut Vec<String>) {
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if is_pattern_line(trimmed) {
            continue;
        }
        for ctor in ["Ok(", "Err(", "Some("] {
            if trimmed.contains(ctor) {
                hits.push(format!("{path}:{}: {ctor} constructor", start_line + i));
            }
        }
    }
}

fn is_pattern_line(trimmed: &str) -> bool {
    trimmed.contains("let Ok(")
        || trimmed.contains("let Err(")
        || trimmed.contains("let Some(")
        || (trimmed.contains("Ok(") && trimmed.contains("=>") && !trimmed.contains("=> Ok("))
        || (trimmed.contains("Err(") && trimmed.contains("=>") && !trimmed.contains("=> Err("))
        || (trimmed.contains("Some(") && trimmed.contains("=>") && !trimmed.contains("=> Some("))
}

struct Collector<'a> {
    path: String,
    hits: &'a mut Vec<String>,
    ctor_check: CtorCheck,
}

impl Collector<'_> {
    fn enter_fn(&mut self, name: &str) -> CtorCheck {
        let next = if matches!(
            name,
            "wrap_ok" | "wrap_err" | "wrap_some" | "boxed" | "wrap_vec" | "to"
        ) {
            CtorCheck::InWrapImpl
        } else {
            CtorCheck::Lint
        };
        std::mem::replace(&mut self.ctor_check, next)
    }

    fn record(&mut self, span: proc_macro2::Span, what: &str) {
        let line = span.start().line;
        let where_ = if line == 0 {
            self.path.clone()
        } else {
            format!("{}:{line}", self.path)
        };
        self.hits.push(format!("{where_}: {what}"));
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let was = self.enter_fn(node.sig.ident.to_string().reference());
        syn::visit::visit_item_fn(self, node);
        self.ctor_check = was;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let was = self.enter_fn(node.sig.ident.to_string().reference());
        syn::visit::visit_impl_item_fn(self, node);
        self.ctor_check = was;
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        let was = self.enter_fn(node.sig.ident.to_string().reference());
        syn::visit::visit_trait_item_fn(self, node);
        self.ctor_check = was;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let CtorCheck::Lint = self.ctor_check {
            match expr {
                Expr::Path(path) => {
                    if let Some(name) = ctor_name(path.path.reference()) {
                        self.record(path.path.span(), &format!("{name}( constructor"));
                    }
                }
                Expr::Call(call) if is_box_new(call.func.reference()) => {
                    self.record(call.func.span(), "Box::new");
                }
                Expr::MethodCall(m) if m.method == "into" && m.args.is_empty() => {
                    self.record(m.method.span(), ".into()");
                }
                _ => {}
            }
        }
        syn::visit::visit_expr(self, expr);
    }

    fn visit_pat(&mut self, _pat: &'ast Pat) {}

    fn visit_macro(&mut self, mac: &'ast Macro) {
        if let CtorCheck::InWrapImpl = self.ctor_check {
            return;
        }
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        match name.as_str() {
            "quote" | "quote_spanned" => return,
            "matches" => {
                if let Ok(expr) = syn::parse2::<Expr>(tokens_before_comma(mac.tokens.clone())) {
                    self.visit_expr(expr.reference());
                }
                return;
            }
            "vec" => {
                if let Ok(exprs) = syn::parse2::<ExprList>(mac.tokens.clone())
                    && exprs.0.len() == 1
                    && !mac.tokens.to_string().contains(';')
                {
                    self.record(mac.path.span(), "vec![x]");
                }
                return;
            }
            _ => {}
        }
        if let Ok(exprs) = syn::parse2::<ExprList>(mac.tokens.clone()) {
            for expr in exprs.0 {
                self.visit_expr(expr.reference());
            }
        }
    }
}

struct ExprList(Vec<Expr>);

impl syn::parse::Parse for ExprList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut exprs = Vec::new();
        while !input.is_empty() {
            if let Ok(expr) = input.parse::<Expr>() {
                exprs.push(expr);
                if input.peek(syn::Token![,]) {
                    let _: syn::Token![,] = input.parse()?;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Self(exprs).wrap_ok()
    }
}

fn tokens_before_comma(tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let mut out = proc_macro2::TokenStream::new();
    for tt in tokens {
        if let proc_macro2::TokenTree::Punct(p) = tt.reference()
            && p.as_char() == ','
        {
            break;
        }
        out.extend(std::iter::once(tt));
    }
    out
}

fn is_box_new(func: &Expr) -> bool {
    let Expr::Path(path) = func else {
        return false;
    };
    let segs: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    matches!(
        segs.iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["Box", "new"] | ["std", "boxed", "Box", "new"]
    )
}

fn ctor_name(path: &syn::Path) -> Option<&'static str> {
    let ident = path.segments.last()?.ident.reference();
    if ident == "Ok" {
        "Ok".wrap_some()
    } else if ident == "Err" {
        "Err".wrap_some()
    } else if ident == "Some" {
        "Some".wrap_some()
    } else {
        None
    }
}
