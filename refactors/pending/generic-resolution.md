# Generic resolution: every stage resolves

`MatchedBrackets<TContents>` resolves at every stage, through one generic walk. Each stage supplies its own resolved-node enum as an associated type on `TreeContents`, so resolving a `MatchedBrackets<BracketsMatched>` answers `ResolvedBracketNode` and resolving a `MatchedBrackets<Chunked>` answers that stage's enum, with the same generated code walking both. Generated code never names an enum variant; every path becomes a node through `From`, and each stage's enum implements `From` for the paths that can occur at that stage.

The division of labor:

- The tree types (`MatchedBrackets`, `BracketItem`, `Bracketed`) get impls generic over `TContents`. They walk spans and delegate.
- A stage's run slot (`TContents::Inner`) is a stage-specific type with its own concrete impl, and a trait bound forces its resolution to land in the stage's enum. This is where a stage's extra leaves live: the chunked stage's run resolves further, into chunks and separators.
- Stage-independent leaf types (`OpenBracket`, `CloseBracket`) implement nothing. A leaf mode in the macro constructs their path and converts it, so one concrete type serves every stage.

Two changes: the path family reshape (ships alone, no behavior change), then the macro rewrite and the derive updates (one unit).

## Change 1 (prefactor): the path family gains the stage parameter

In `matched_brackets.rs`. The parent enums and path aliases become generic over `TContents`, defaulting to `BracketsMatched` so every existing use keeps compiling unchanged; `TreeContents` gains the stage's resolved enum; `ResolvedBracketNode` gains its `From` impls. The derives stay pinned in this change and still name variants; both construction styles coexist until Change 2.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub trait TreeContents {
    /// What a run between brackets is: the `Inner` run of lexed tokens out of the matcher,
    /// parsed nodes later.
    type Inner: fmt::Debug + PartialEq + Eq;
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined.
    type StrayClose: fmt::Debug + PartialEq + Eq;
}

pub type MatchedBracketsPath<'a> = PositionResolutionPath<&'a MatchedBrackets<BracketsMatched>, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(Box<BracketedPath<'a>>),
}

pub type BracketedPath<'a> =
    PositionResolutionPath<&'a Bracketed<BracketsMatched>, BracketItemParent<'a>>;
pub type InnerPath<'a> = PositionResolutionPath<&'a Inner, BracketItemParent<'a>>;

/// The one place an opening bracket can sit: its group.
#[derive(Debug)]
pub enum OpenBracketParent<'a> {
    Bracketed(Box<BracketedPath<'a>>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, OpenBracketParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, BracketItemParent<'a>>;
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub trait TreeContents: Sized + 'static {
    /// What a run between brackets is: the `Inner` run of lexed tokens out of the matcher,
    /// chunked or parsed forms later. It resolves into this stage's enum; this bound is
    /// what lets the generic walk delegate into it.
    type Inner: fmt::Debug
        + PartialEq
        + Eq
        + for<'a> ResolvePosition<
            Parent<'a> = BracketItemParent<'a, Self>,
            ResolvedNode<'a> = Self::Resolved<'a>,
        >;
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined. A leaf; its path converts into the stage's enum.
    type StrayClose: fmt::Debug + PartialEq + Eq + 'static;
    /// The stage's resolved-node enum: every node a position can resolve to at this
    /// stage. The `From` bounds are the constructions the generic walk performs.
    type Resolved<'a>: From<MatchedBracketsPath<'a, Self>>
        + From<BracketedPath<'a, Self>>
        + From<OpenBracketPath<'a, Self>>
        + From<CloseBracketPath<'a, Self>>
        + From<StrayClosePath<'a, Self>>;
}

pub type MatchedBracketsPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a MatchedBrackets<TContents>, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a, TContents: TreeContents> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Bracketed(Box<BracketedPath<'a, TContents>>),
}

pub type BracketedPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a Bracketed<TContents>, BracketItemParent<'a, TContents>>;
pub type InnerPath<'a> = PositionResolutionPath<&'a Inner, BracketItemParent<'a, BracketsMatched>>;

/// The one place an opening bracket can sit: its group.
#[derive(Debug)]
pub enum OpenBracketParent<'a, TContents: TreeContents> {
    Bracketed(Box<BracketedPath<'a, TContents>>),
}

pub type OpenBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a OpenBracket, OpenBracketParent<'a, TContents>>;
pub type CloseBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a CloseBracket, BracketItemParent<'a, TContents>>;
pub type StrayClosePath<'a, TContents> = PositionResolutionPath<
    &'a <TContents as TreeContents>::StrayClose,
    BracketItemParent<'a, TContents>,
>;
```

At `BracketsMatched`, `StrayClosePath` normalizes to `CloseBracketPath`, so its `From` bound is satisfied by the `CloseBracketPath` impl and no second impl exists (a second one would conflict). A refined stage with `StrayClose = Infallible` satisfies the bound with `match *path.inner {}`.

The stage impl and the `From` impls:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl TreeContents for BracketsMatched {
    type Inner = Inner;
    type StrayClose = CloseBracket;
    type Resolved<'a> = ResolvedBracketNode<'a>;
}

impl<'a> From<MatchedBracketsPath<'a>> for ResolvedBracketNode<'a> {
    fn from(path: MatchedBracketsPath<'a>) -> Self {
        ResolvedBracketNode::MatchedBrackets(path)
    }
}

impl<'a> From<BracketedPath<'a>> for ResolvedBracketNode<'a> {
    fn from(path: BracketedPath<'a>) -> Self {
        ResolvedBracketNode::Bracketed(path)
    }
}

impl<'a> From<InnerPath<'a>> for ResolvedBracketNode<'a> {
    fn from(path: InnerPath<'a>) -> Self {
        ResolvedBracketNode::Inner(path)
    }
}

impl<'a> From<OpenBracketPath<'a>> for ResolvedBracketNode<'a> {
    fn from(path: OpenBracketPath<'a>) -> Self {
        ResolvedBracketNode::OpenBracket(path)
    }
}

impl<'a> From<CloseBracketPath<'a>> for ResolvedBracketNode<'a> {
    fn from(path: CloseBracketPath<'a>) -> Self {
        ResolvedBracketNode::CloseBracket(path)
    }
}
```

The parent enums' conversions, used by Change 2's emissions (each parent enum implements `From` of the container paths that can hold the child; the boxing lives here):

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<'a, TContents: TreeContents> From<MatchedBracketsPath<'a, TContents>>
    for BracketItemParent<'a, TContents>
{
    fn from(path: MatchedBracketsPath<'a, TContents>) -> Self {
        BracketItemParent::MatchedBrackets(path)
    }
}

impl<'a, TContents: TreeContents> From<BracketedPath<'a, TContents>>
    for BracketItemParent<'a, TContents>
{
    fn from(path: BracketedPath<'a, TContents>) -> Self {
        BracketItemParent::Bracketed(Box::new(path))
    }
}

impl<'a, TContents: TreeContents> From<BracketedPath<'a, TContents>>
    for OpenBracketParent<'a, TContents>
{
    fn from(path: BracketedPath<'a, TContents>) -> Self {
        OpenBracketParent::Bracketed(Box::new(path))
    }
}
```

`ResolvedBracketNode` itself is unchanged in variants; the existing tests compile and pass untouched, since every alias they name defaults to `BracketsMatched`.

## Change 2: the macro emits generic impls and `From`-based construction

In `resolve_position_macros` and the derive sites. One unit: the emission style and the attributes change together.

### The attribute surface

`self_type_generics` is deleted, and with it the whole `map_generics` module; the derive emits one impl using the type's own generics via `split_for_impl`, the way freddie's bind_macro does. `#[resolve_field]` keeps marking what resolution descends into; `#[resolve_field(leaf)]` marks a field or enum variant whose payload is a leaf — the emission constructs the payload's path and converts it, delegating nothing.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(resolve_position))]
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
}
```

The field cases keep the `WithSpan`/`Option`/`Vec` shape detection, but the emissions no longer name the child's type or any enum variant, so `extract_single_generic_type` and the generics map go. `ResolvePosition::path` in the `resolve_position` crate is deleted too — nothing calls it once construction is literal — and that crate's own test module rewrites its hand impls in the literal-plus-`From` style.

### The derive sites

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a, BracketsMatched>, resolved_node = ResolvedBracketNode<'a>)]
pub struct Inner(pub Vec<WithSpan<NonBracketTokenKind>>);

/// A group's opening bracket.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenBracket(pub BracketKind);

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CloseBracket(pub BracketKind);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TContents::Resolved<'a>)]
pub struct MatchedBrackets<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<BracketItem<TContents>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a, TContents>,
    resolved_node = TContents::Resolved<'a>
)]
pub enum BracketItem<TContents: TreeContents> {
    Inner(TContents::Inner),
    Bracketed(Bracketed<TContents>),
    #[resolve_field(leaf)]
    StrayClose(TContents::StrayClose),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a, TContents>,
    resolved_node = TContents::Resolved<'a>
)]
pub struct Bracketed<TContents: TreeContents> {
    #[resolve_field(leaf)]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field]
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
    /// The close the author typed, or `None` for a group that never got its close and was
    /// forced to end: at the close bracket an enclosing group owns, or at the end of the
    /// tokens. A `None` group is an invalid section.
    #[resolve_field(leaf)]
    pub closing: Option<WithSpan<CloseBracket>>,
}
```

`OpenBracket` and `CloseBracket` lose their `ResolvePosition` derives entirely: they are data, and the leaf emissions build their paths from outside.

### The generated code

For `Bracketed<TContents>`:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<TContents: TreeContents> ::resolve_position::ResolvePosition for Bracketed<TContents> {
    type Parent<'a>
        = BracketItemParent<'a, TContents>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = TContents::Resolved<'a>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        // opening: #[resolve_field(leaf)] on a WithSpan field.
        if self.opening.location.contains(position) {
            let own_path = ::resolve_position::PositionResolutionPath {
                inner: self,
                parent,
            };
            return ::resolve_position::PositionResolutionPath {
                inner: &self.opening.item,
                parent: own_path.into(),
            }
            .into();
        }
        // children: #[resolve_field] on a Vec<WithSpan> field.
        for item in self.children.iter() {
            if item.location.contains(position) {
                let own_path = ::resolve_position::PositionResolutionPath {
                    inner: self,
                    parent,
                };
                return item.item.resolve(own_path.into(), position);
            }
        }
        // closing: #[resolve_field(leaf)] on an Option<WithSpan> field.
        for item in self.closing.iter() {
            if item.location.contains(position) {
                let own_path = ::resolve_position::PositionResolutionPath {
                    inner: self,
                    parent,
                };
                return ::resolve_position::PositionResolutionPath {
                    inner: &item.item,
                    parent: own_path.into(),
                }
                .into();
            }
        }
        ::resolve_position::PositionResolutionPath {
            inner: self,
            parent,
        }
        .into()
    }
}
```

The delegating arm's `own_path.into()` resolves to the child's `Parent` type through the parent enum's `From` impls; the leaf arms' outer `.into()` resolves to `Self::ResolvedNode` through the stage enum's `From` bounds. One field of the walk moved on purpose: a delegating field builds `own_path` inside the span check, so the borrow of `self` stays shared until a hit.

For `BracketItem<TContents>`:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<TContents: TreeContents> ::resolve_position::ResolvePosition for BracketItem<TContents> {
    type Parent<'a>
        = BracketItemParent<'a, TContents>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = TContents::Resolved<'a>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            BracketItem::Inner(inner) => inner.resolve(parent, position),
            BracketItem::Bracketed(inner) => inner.resolve(parent, position),
            // #[resolve_field(leaf)]: the payload's path converts, no delegation.
            BracketItem::StrayClose(inner) => ::resolve_position::PositionResolutionPath {
                inner,
                parent,
            }
            .into(),
        }
    }
}
```

`MatchedBrackets<TContents>` generates the same shape as today's root — the loop over items with `own_path.into()` delegation, then the fallback `PositionResolutionPath { inner: self, parent }.into()`.

### Tests

The bracket suite is the regression harness: every existing resolution test keeps its assertions, since at `BracketsMatched` the answers are unchanged. One addition proves the generic walk is generic in more than name:

- A second `TreeContents` implementor in `#[cfg(test)]` whose `Inner` is a run newtype with its own resolved enum, resolved through `MatchedBrackets<ThatStage>` — the compile is most of the assertion, and one test resolves a position into the stage-specific leaf.

## Landing checklist

1. Change 1 in matched_brackets.rs; `cargo test` green with no test edits.
2. Change 2 across resolve_position, resolve_position_macros, and matched_brackets.rs; `cargo test` green with no assertion edits.
3. chunking.md's resolution section becomes implementable; its doc gains the `Chunked` resolved enum on top of this.
4. Move this doc to refactors/past.
