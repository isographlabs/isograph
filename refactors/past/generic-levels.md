# Generic levels

Won't do. The design below makes the tree and resolution types generic over the stage, so the bracket tree and the chunk tree share one skeleton and one resolution enum with both stages' resolutions live at once. It was rejected: the parent chains are stage-specific anyway (every stage inserts ancestors, so no structure-preserving map between stages' resolved nodes can exist), which reduces the shared enum to definitional deduplication across exactly two stages, paid for with the `LevelItem` trait, three GATs, the `Item` slot, `resolved_node_variant` in the macro, and a hand-written `Level` impl to dodge a higher-ranked bound. The direction taken instead: resolution answers positions against the newest tree only, through the one concrete `IsographResolutionNode` enum (isograph-resolution-node.md), whose variants each stage modifies in place — chunking.md is the first such modification, and it takes the bracket tree's resolution out as it puts the chunk tree's in.

The original design follows, unedited.

---

The tree types and the resolution types become generic over what a level holds, so the bracket tree and chunking.md's chunk tree are two stages of one skeleton and share one resolution enum. This re-lands the idea of refactors/past/generic-resolution.md — one resolved-node enum, generic over the stage, with shared leaf types resolving at every stage — reshaped for the tree as raw-items.md and one-variant-parent-enums.md left it. raw-items.md deleted `TreeContents` and the generic machinery because only one stage remained; chunking is the second stage, so the generality comes back, now as a type parameter for the level's item type rather than a contents trait with slots.

The skeleton is levels and groups: `Level<TItem>` is a `Vec<WithSpan<TItem>>`, and `Group<TItem>` is an opening, a `Level<TItem>` interior, and a closing. The bracket stage sets `TItem = BracketItem` (raw tokens and groups); the chunk stage sets `TItem = ChunkedLevelItem` (chunks and separators). `ResolvedBracketNode<'a, TItem>` is the one resolution enum, closed: `Level`, `Group`, `NonBracketToken`, `OpenBracket`, `CloseBracket`, and `Item`, where `Item` holds the stage's own leaves — uninhabited at the bracket stage, the chunk and separator nodes at the chunk stage. `NonBracketTokenPath<'a, BracketItem>` and `NonBracketTokenPath<'a, ChunkedLevelItem>` are the same variant's payload at the two stages; the stage decides who hosts the token (the level, or the chunk).

The leaf types `NonBracketToken`, `OpenBracket`, and `CloseBracket` stop implementing `ResolvePosition`: a single type cannot carry one impl per stage, so containers construct leaf paths from outside via a new `#[resolve_field(leaf = Variant)]` emission, the mechanism generic-resolution.md landed the first time.

Two files change: `crates/resolve_position_macros/src/resolve_position_macro.rs` (plus deleting its `map_generics` module) and `crates/isograph_parser/src/matched_brackets.rs`. Everything before the landing checklist is the finished state.

## The stage trait

What varies by stage is where three things sit: the stage's groups, the stage's loose tokens, and the stage's own leaves. `LevelItem` names those three, plus the one method `Level<TItem>`'s resolve calls. The method exists so `Level`'s impl needs no higher-ranked bound equating `TItem`'s `ResolvePosition` associated types with the level's — each stage's item type writes the two-line delegation instead, and every bound in the crate stays concrete or plainly generic.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub trait LevelItem: Sized {
    /// Where a group sits at this stage: directly in a level at the bracket stage, in
    /// the chunk that owns it as trailing group at the chunk stage.
    type GroupHost<'a>: fmt::Debug
    where
        Self: 'a;
    /// Where a leaf token sits when it is not a group's own bracket: the level at the
    /// bracket stage, the chunk that holds it at the chunk stage.
    type TokenHost<'a>: fmt::Debug
    where
        Self: 'a;
    /// The stage's own leaves, the payload of `ResolvedBracketNode::Item`: uninhabited
    /// at the bracket stage, the chunk and separator nodes at the chunk stage.
    type ResolvedItemNode<'a>: fmt::Debug
    where
        Self: 'a;

    /// The item's resolution, with the owning level's path as parent. Implementations
    /// delegate to the item type's own `ResolvePosition` impl.
    fn resolve_item<'a>(
        &'a self,
        parent: LevelPath<'a, Self>,
        position: Span,
    ) -> ResolvedBracketNode<'a, Self>;
}
```

## The tree types

`MatchedBrackets` and `Bracketed` become aliases, so the matcher, `errors()`, and every construction site keep their spelling. `Level` carries no bound (its impls bound `TItem` themselves); `Group` declares `TItem: LevelItem` because its derived impl needs the bound and the derive copies the type's own generics.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// One level: the whole literal at the root, a group's interior below. Generic over
/// what the level holds.
#[derive(Debug, PartialEq, Eq)]
pub struct Level<TItem>(pub Vec<WithSpan<TItem>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <TItem as LevelItem>::GroupHost<'a>,
    resolved_node = ResolvedBracketNode<'a, TItem>
)]
pub struct Group<TItem: LevelItem> {
    #[resolve_field(leaf = OpenBracket)]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<Level<TItem>>,
    #[resolve_field(leaf = CloseBracket)]
    pub closing: WithSpan<CloseBracket>,
}

/// The bracket stage: levels hold raw tokens and groups.
pub type MatchedBrackets = Level<BracketItem>;
pub type Bracketed = Group<BracketItem>;

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = LevelPath<'a, BracketItem>,
    resolved_node = ResolvedBracketNode<'a, BracketItem>
)]
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

impl LevelItem for BracketItem {
    type GroupHost<'a> = LevelPath<'a, BracketItem>;
    type TokenHost<'a> = LevelPath<'a, BracketItem>;
    type ResolvedItemNode<'a> = std::convert::Infallible;

    fn resolve_item<'a>(
        &'a self,
        parent: LevelPath<'a, Self>,
        position: Span,
    ) -> ResolvedBracketNode<'a, Self> {
        self.resolve(parent, position)
    }
}

/// A token that is not part of any structure; matched brackets are structure, never
/// raw, so a bracket token here is unmatched.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = LevelPath<'a, BracketItem>,
    resolved_node = ResolvedBracketNode<'a, BracketItem>
)]
pub enum RawToken {
    NonBracket(#[resolve_field(leaf = NonBracketToken)] NonBracketToken),
    Open(#[resolve_field(leaf = OpenBracket)] OpenBracket),
    Close(#[resolve_field(leaf = CloseBracket)] CloseBracket),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NonBracketToken(pub NonBracketTokenKind);

/// An opening bracket; its parent says whether it is a group's own opening or
/// unmatched.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenBracket(pub BracketKind);

/// A closing bracket; its parent says whether it is a group's own closing or
/// unmatched.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CloseBracket(pub BracketKind);
```

`NonBracketToken`, `OpenBracket`, and `CloseBracket` lose their `ResolvePosition` derives; their paths are built by the leaf emissions in `Group`, `RawToken`, and (at the chunk stage) chunking.md's `ChunkToken`.

## The resolution types

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug)]
pub enum ResolvedBracketNode<'a, TItem: LevelItem> {
    Level(LevelPath<'a, TItem>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    Group(GroupPath<'a, TItem>),
    NonBracketToken(NonBracketTokenPath<'a, TItem>),
    OpenBracket(OpenBracketPath<'a, TItem>),
    CloseBracket(CloseBracketPath<'a, TItem>),
    /// The stage's own leaves; never constructed at the bracket stage, whose payload is
    /// uninhabited.
    Item(TItem::ResolvedItemNode<'a>),
}

#[derive(Debug)]
pub enum LevelParent<'a, TItem: LevelItem> {
    Root,
    Interior(Box<GroupPath<'a, TItem>>),
}

pub type LevelPath<'a, TItem> =
    PositionResolutionPath<&'a Level<TItem>, LevelParent<'a, TItem>>;

pub type GroupPath<'a, TItem> =
    PositionResolutionPath<&'a Group<TItem>, <TItem as LevelItem>::GroupHost<'a>>;

pub type NonBracketTokenPath<'a, TItem> =
    PositionResolutionPath<&'a NonBracketToken, <TItem as LevelItem>::TokenHost<'a>>;

/// Shared by `OpenBracket` and `CloseBracket`: a bracket token is a group's own
/// opening or closing, or unmatched wherever the stage hosts loose tokens.
#[derive(Debug)]
pub enum BracketTokenParent<'a, TItem: LevelItem> {
    Matched(GroupPath<'a, TItem>),
    Unmatched(TItem::TokenHost<'a>),
}

pub type OpenBracketPath<'a, TItem> =
    PositionResolutionPath<&'a OpenBracket, BracketTokenParent<'a, TItem>>;
pub type CloseBracketPath<'a, TItem> =
    PositionResolutionPath<&'a CloseBracket, BracketTokenParent<'a, TItem>>;
```

The leaf emissions convert parents through `From`, so each stage supplies the conversions its hosts admit. The `Matched` arm is stage-independent; the `Unmatched` arm is per-stage because `TokenHost` is associated:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<'a, TItem: LevelItem> From<GroupPath<'a, TItem>> for BracketTokenParent<'a, TItem> {
    fn from(path: GroupPath<'a, TItem>) -> Self {
        BracketTokenParent::Matched(path)
    }
}

impl<'a> From<LevelPath<'a, BracketItem>> for BracketTokenParent<'a, BracketItem> {
    fn from(path: LevelPath<'a, BracketItem>) -> Self {
        BracketTokenParent::Unmatched(path)
    }
}
```

## Level's hand-written impl

Every other node derives; `Level` is written out because its items loop goes through `LevelItem::resolve_item`, which no emission produces.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<TItem: LevelItem> ResolvePosition for Level<TItem> {
    type Parent<'a>
        = LevelParent<'a, TItem>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TItem>
    where
        Self: 'a;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a> {
        for item in self.0.iter() {
            if item.location.contains(position) {
                let own_path = PositionResolutionPath {
                    inner: self,
                    parent,
                };
                return item.item.resolve_item(own_path, position);
            }
        }
        ResolvedBracketNode::Level(PositionResolutionPath {
            inner: self,
            parent,
        })
    }
}
```

`errors()` and the matcher are untouched apart from spelling: the inherent impl sits on `MatchedBrackets` (the alias), and `match_brackets`, `ParsedGroup`, and `parse_bracketed` construct `Bracketed` and `MatchedBrackets` through the aliases exactly as today.

## The macro

Three changes in `resolve_position_macros`.

1. Generic impls. The derive emits over the type's own generics via `split_for_impl`, the way freddie's bind_macro does: `impl #impl_generics ResolvePosition for #name #ty_generics #where_clause`. The `self_type_generics` attribute argument and the `map_generics` module are deleted; nothing uses them once the one generic derive site (`Group<TItem>`) states its bound on the type itself.

2. Leaf mode. `#[resolve_field(leaf = Variant)]` marks a field or enum-variant payload whose type is a leaf: the emission constructs the payload's path and wraps it in the named `ResolvedNode` variant, delegating nothing, so the payload type needs no `ResolvePosition` impl. Combining `leaf` with `parent_variant` is a compile error; a leaf's parent converts through `From`, with the target inferred from the named variant's payload type. On a struct field the existing wrapper machinery (`WithSpan`, `Vec`, `Option`) applies and the innermost emission becomes:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
if self.opening.location.contains(position) {
    let own_path = ::resolve_position::PositionResolutionPath {
        inner: self,
        parent,
    };
    return Self::ResolvedNode::OpenBracket(::resolve_position::PositionResolutionPath {
        inner: &self.opening.item,
        parent: own_path.into(),
    });
}
```

On an enum-variant payload the parent passes through `From` directly, since the enum narrows without becoming a path segment of its own:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
RawToken::Open(inner) => Self::ResolvedNode::OpenBracket(
    ::resolve_position::PositionResolutionPath {
        inner,
        parent: parent.into(),
    },
)
```

3. Fallback variant override. A struct's fallback emission names the variant after the type: `Self::ResolvedNode::#struct_name(self.path(parent).into())`. The container attribute gains an optional `resolved_node_variant` that overrides the name, for stage-specific nodes that answer through the `Item` slot: chunking.md's `Chunk` and `Separator` declare `resolved_node_variant = Item`, and the `.into()` converts their own path into the stage's `ResolvedItemNode`. The bracket stage's types keep the default.

`path()` on the trait stays: the descend emissions still use it, and the leaf emissions build their path literals directly because the conversion applies to the completed own-path, not to the parent going in.

## Tests

The bracket suite keeps every assertion; the renames are mechanical: `ResolvedBracketNode::MatchedBrackets` → `::Level`, `::Bracketed` → `::Group`, `MatchedBracketsParent` → `LevelParent`, `BracketedPath<'a>` → `GroupPath<'a, BracketItem>`, and the resolve entry point becomes `tree.resolve(LevelParent::Root, span)`.

One addition proves the generics in fact, ahead of the chunk stage: a `#[cfg(test)]` second stage whose item is a run of tokens hosting its own tokens, exercising a `TokenHost` that is not the level and an inhabited `Item` slot.

```rust
// from crates/isograph_parser/src/matched_brackets.rs, in the test module
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = LevelPath<'a, Run>,
    resolved_node = ResolvedBracketNode<'a, Run>,
    resolved_node_variant = Item
)]
struct Run(#[resolve_field(leaf = NonBracketToken)] Vec<WithSpan<NonBracketToken>>);

type RunPath<'a> = PositionResolutionPath<&'a Run, LevelPath<'a, Run>>;

impl LevelItem for Run {
    type GroupHost<'a> = LevelPath<'a, Run>;
    type TokenHost<'a> = RunPath<'a>;
    type ResolvedItemNode<'a> = RunPath<'a>;

    fn resolve_item<'a>(
        &'a self,
        parent: LevelPath<'a, Self>,
        position: Span,
    ) -> ResolvedBracketNode<'a, Self> {
        self.resolve(parent, position)
    }
}
```

Both leaf conversions at this stage are the reflexive `From`, so the stage needs no impls of its own. The tests: a position on a token answers `NonBracketToken` with the run as its host; a position on the run but between its tokens answers `Item` holding the run's path; a position outside every run answers `Level`.

## Landing checklist

1. The macro changes: `split_for_impl`, `self_type_generics` and `map_generics` deleted, leaf mode, `resolved_node_variant`.
2. matched_brackets.rs rewritten as above; `cargo test -p isograph_parser` green with only the mechanical renames in assertions.
3. The `Run` stage tests.
4. Move this doc to refactors/past; chunking.md sits on top of it.
