# Isograph resolution node

`ResolvedBracketNode` is renamed to `IsographResolutionNode` and moves into its own module. The enum is the crate's one lasting resolution surface: positions always resolve against the newest tree, and each new stage modifies this enum's variants in place rather than adding an enum beside it — chunking.md is the first such modification, swapping the bracket-stage variants for chunk-stage ones. The name and location stop referencing the bracket stage so the enum never renames again. This refactor changes no variants and no behavior; every spelling site updates.

## The move

New module, registered in lib.rs:

```rust
// from crates/isograph_parser/src/lib.rs
mod isograph_resolution_node;
mod matched_brackets;
mod non_bracket_token;
mod token_kind;
mod tokenize;

pub use isograph_resolution_node::*;
pub use matched_brackets::*;
pub use non_bracket_token::*;
pub use token_kind::*;
pub use tokenize::*;
```

The module holds the enum alone; the path types it references stay where they are defined:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
use crate::{
    BracketedPath, CloseBracketPath, MatchedBracketsPath, NonBracketTokenPath, OpenBracketPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the bracket tree's.
#[derive(Debug)]
pub enum IsographResolutionNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    /// This will be resolved for spans that contains one of the opening/closing brace
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    Bracketed(BracketedPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
```

## The spelling sites

- Every `resolved_node = ResolvedBracketNode<'a>` derive attribute in matched_brackets.rs becomes `resolved_node = IsographResolutionNode<'a>`.
- The resolution tests' matches rename the same way.

Nothing else changes: the parent enums, the path aliases, `errors()`, and the matcher are untouched.

## Landing checklist

1. The new module, the rename, and the attribute and test updates; `cargo test -p isograph_parser` and the clippy pre-commit hook pass with no assertion changes beyond the spelling.
2. Move this doc to refactors/past.
