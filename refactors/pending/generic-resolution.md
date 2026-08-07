# Generic resolution

`ResolvedBracketNode` becomes generic over the stage, and `MatchedBrackets<TContents>` resolves at every stage through one generic walk. The enum stays closed at five variants. `Inner(InnerPath<'a, TContents>)` is the tree-level answer for a position in a run; a stage whose run type has interior structure answers the finer question with a second resolve on the run itself, whose parent is the `InnerPath` from the first answer, so ancestry chains across the two queries (chunking.md's `ChunkedRun` is the first such run type). `TreeContents` is unchanged. Leaf types (`Inner`, `OpenBracket`, `CloseBracket`) stop implementing `ResolvePosition` at the tree level; the walk constructs their paths from outside.

Three changes: the path family gains the stage parameter (ships alone, behavior unchanged), the macro emits generic impls with leaf modes, and the test extractors become derived.

## Change 1 (prefactor): the path family gains the stage parameter

In `matched_brackets.rs`. Every parent enum, path alias, and the resolved enum gain `TContents`, defaulting to `BracketsMatched` where existing code names them with one lifetime; the derives stay pinned in this change and keep compiling because the defaults keep the names they emit meaningful.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug)]
pub enum ResolvedBracketNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(BracketedPath<'a>),
    Inner(InnerPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
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
/// Every node a position can resolve to, at any stage. `Inner` is the tree-level answer
/// for a position in a run; a stage whose run type has interior structure answers the
/// finer question with a second resolve on the run, parented by the `InnerPath`.
#[derive(Debug)]
pub enum ResolvedBracketNode<'a, TContents: TreeContents = BracketsMatched> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Bracketed(BracketedPath<'a, TContents>),
    Inner(InnerPath<'a, TContents>),
    OpenBracket(OpenBracketPath<'a, TContents>),
    CloseBracket(CloseBracketPath<'a, TContents>),
}

pub type MatchedBracketsPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a MatchedBrackets<TContents>, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a, TContents: TreeContents = BracketsMatched> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Bracketed(Box<BracketedPath<'a, TContents>>),
}

/// The conversion the leaf emissions build parents through.
impl<'a, TContents: TreeContents> From<BracketedPath<'a, TContents>>
    for BracketItemParent<'a, TContents>
{
    fn from(path: BracketedPath<'a, TContents>) -> Self {
        BracketItemParent::Bracketed(Box::new(path))
    }
}

pub type BracketedPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a Bracketed<TContents>, BracketItemParent<'a, TContents>>;
pub type InnerPath<'a, TContents = BracketsMatched> = PositionResolutionPath<
    &'a <TContents as TreeContents>::Inner,
    BracketItemParent<'a, TContents>,
>;

/// The one place an opening bracket can sit: its group.
#[derive(Debug)]
pub enum OpenBracketParent<'a, TContents: TreeContents = BracketsMatched> {
    Bracketed(Box<BracketedPath<'a, TContents>>),
}

impl<'a, TContents: TreeContents> From<BracketedPath<'a, TContents>>
    for OpenBracketParent<'a, TContents>
{
    fn from(path: BracketedPath<'a, TContents>) -> Self {
        OpenBracketParent::Bracketed(Box::new(path))
    }
}

pub type OpenBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a OpenBracket, OpenBracketParent<'a, TContents>>;
pub type CloseBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a CloseBracket, BracketItemParent<'a, TContents>>;
```

`InnerPath<'a>` now means `InnerPath<'a, BracketsMatched>`, whose inner is `&Inner` as before, and the enums carry the same default, so the tests' helper signatures, the pinned derives' attribute sites, and every assertion are untouched. `TContents::StrayClose` at every stage is `CloseBracket` or a future refined type; the stray's path is `CloseBracketPath` today, and a stage that changes the slot writes its own alias then.

## Change 2: the macro emits generic impls with leaf modes

In `resolve_position_macros`, `resolve_position`, and the derive sites. One unit.

### Attribute surface

`self_type_generics` is deleted, and with it the `map_generics` module: the derive emits one impl over the type's own generics via `split_for_impl`, the way freddie's bind_macro does. `#[resolve_field]` keeps marking what resolution descends into. `#[resolve_field(leaf = VariantName)]` marks a field or enum variant whose payload is a leaf: the emission constructs the payload's path and wraps it in the named `ResolvedNode` variant, delegating nothing, and the payload type needs no `ResolvePosition` impl.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(resolve_position))]
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
}
```

### `path()` is deleted

The trait's one helper builds a node's own path, converting the parent on the way in. The new emissions build the struct literal and convert the completed path instead, so nothing calls it, and its only remaining users would be `resolve_position`'s own test module.

Before:

```rust
// from crates/resolve_position/src/lib.rs
pub trait ResolvePosition: Sized {
    type Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
    where
        Self: 'a;

    /// Called when we are sure that the node contains the cursor. i.e. the parent must check
    /// self.field.location.contains(position) before calling .resolve().
    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a>;

    fn path<'a, TParent: From<Self::Parent<'a>>>(
        &'a self,
        parent: Self::Parent<'a>,
    ) -> PositionResolutionPath<&'a Self, TParent> {
        PositionResolutionPath {
            inner: self,
            parent: parent.into(),
        }
    }
}
```

After:

```rust
// from crates/resolve_position/src/lib.rs
pub trait ResolvePosition: Sized {
    type Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
    where
        Self: 'a;

    /// Called when we are sure that the node contains the cursor. i.e. the parent must check
    /// self.field.location.contains(position) before calling .resolve().
    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a>;
}
```

The test module's two hand impls document the convention, so they rewrite to the exact shape the derive now emits.

Before:

```rust
// from crates/resolve_position/src/lib.rs
    impl ResolvePosition for Parent {
        type Parent<'a> = ();

        type ResolvedNode<'a> = TestResolvedNode<'a>;

        fn resolve<'a>(
            &'a self,
            parent: Self::Parent<'a>,
            position: Span,
        ) -> Self::ResolvedNode<'a> {
            for child in self.children.iter() {
                if child.location.contains(position) {
                    let parent = <Child as ResolvePosition>::Parent::Parent(self.path(parent));
                    return child.item.resolve(parent, position);
                }
            }

            Self::ResolvedNode::Parent(self.path(parent))
        }
    }

    impl ResolvePosition for Child {
        type Parent<'a> = ChildParent<'a>;

        type ResolvedNode<'a> = TestResolvedNode<'a>;

        fn resolve<'a>(
            &'a self,
            mut parent: Self::Parent<'a>,
            position: Span,
        ) -> Self::ResolvedNode<'a> {
            for child in self.children.iter() {
                if child.location.contains(position) {
                    let parent = <Child as ResolvePosition>::Parent::Child(self.path(parent));
                    return child.item.resolve(parent, position);
                }
            }

            Self::ResolvedNode::Child(self.path(parent))
        }
    }
```

After:

```rust
// from crates/resolve_position/src/lib.rs
    impl ResolvePosition for Parent {
        type Parent<'a> = ();

        type ResolvedNode<'a> = TestResolvedNode<'a>;

        fn resolve<'a>(
            &'a self,
            parent: Self::Parent<'a>,
            position: Span,
        ) -> Self::ResolvedNode<'a> {
            for child in self.children.iter() {
                if child.location.contains(position) {
                    let parent = ChildParent::Parent(PositionResolutionPath {
                        inner: self,
                        parent,
                    });
                    return child.item.resolve(parent, position);
                }
            }

            Self::ResolvedNode::Parent(PositionResolutionPath {
                inner: self,
                parent,
            })
        }
    }

    impl ResolvePosition for Child {
        type Parent<'a> = ChildParent<'a>;

        type ResolvedNode<'a> = TestResolvedNode<'a>;

        fn resolve<'a>(
            &'a self,
            parent: Self::Parent<'a>,
            position: Span,
        ) -> Self::ResolvedNode<'a> {
            for child in self.children.iter() {
                if child.location.contains(position) {
                    let parent = ChildParent::Child(PositionResolutionPath {
                        inner: self,
                        parent: Box::new(parent),
                    });
                    return child.item.resolve(parent, position);
                }
            }

            Self::ResolvedNode::Child(PositionResolutionPath {
                inner: self,
                parent: Box::new(parent),
            })
        }
    }
```

### Two defects this change's first implementation attempt surfaced

Both are recorded here so the doc, not the implementer, decides.

The first is mechanical and now fixed in this doc: the generic `MatchedBrackets` impl delegates through `own_path.into()` like every other container, so `BracketItemParent` needs a `From` for the root path too, alongside the group conversion Change 1 landed:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<'a, TContents: TreeContents> From<MatchedBracketsPath<'a, TContents>>
    for BracketItemParent<'a, TContents>
{
    fn from(path: MatchedBracketsPath<'a, TContents>) -> Self {
        BracketItemParent::MatchedBrackets(path)
    }
}
```

The second is open and blocks Change 2: the `StrayClose` leaf arm builds a path whose inner is `&TContents::StrayClose`, but the `CloseBracket` variant's payload is `CloseBracketPath`, whose inner is the concrete `&CloseBracket`. Generic code cannot equate them — rustc demands `TContents: TreeContents<StrayClose = CloseBracket>` somewhere. The options:

- Bound the tree types: `BracketItem<TContents: TreeContents<StrayClose = CloseBracket>>`. Compiles today, and forecloses the refined stage whose stray slot is `Infallible` — the slot would be pinned forever.
- Split the leaf: the enum gains a sixth variant `StrayClose(StrayClosePath<'a, TContents>)`, the stray arm targets it, and the closing field keeps `CloseBracket`. Restores the stray-versus-closing distinction in the leaf enum that the close-bracket work deliberately collapsed.
- Unify the slot: `TreeContents::StrayClose` becomes the type of every close token, and `Bracketed.closing` becomes `Option<WithSpan<TContents::StrayClose>>`. Both leaf sites then build the same slot-typed path, the `CloseBracket` variant's payload becomes that path, and the collapse survives generically; a refined stage's slot change flows through closings and strays together, which matches their shared meaning.

### Derive sites

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// A maximal run of non-bracket tokens between brackets.
#[derive(Debug, PartialEq, Eq)]
pub struct Inner(pub Vec<WithSpan<NonBracketTokenKind>>);

/// A group's opening bracket.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenBracket(pub BracketKind);

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CloseBracket(pub BracketKind);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = ResolvedBracketNode<'a, TContents>)]
pub struct MatchedBrackets<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<BracketItem<TContents>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a, TContents>,
    resolved_node = ResolvedBracketNode<'a, TContents>
)]
pub enum BracketItem<TContents: TreeContents> {
    #[resolve_field(leaf = Inner)]
    Inner(TContents::Inner),
    Bracketed(Bracketed<TContents>),
    #[resolve_field(leaf = CloseBracket)]
    StrayClose(TContents::StrayClose),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a, TContents>,
    resolved_node = ResolvedBracketNode<'a, TContents>
)]
pub struct Bracketed<TContents: TreeContents> {
    #[resolve_field(leaf = OpenBracket)]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field]
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
    /// The close the author typed, or `None` for a group that never got its close and was
    /// forced to end: at the close bracket an enclosing group owns, or at the end of the
    /// tokens. A `None` group is an invalid section.
    #[resolve_field(leaf = CloseBracket)]
    pub closing: Option<WithSpan<CloseBracket>>,
}
```

`Inner` loses its derive along with `OpenBracket` and `CloseBracket`: at the tree level all three are leaves, and `Inner`'s old impl answered exactly what the leaf emission answers.

### Generated code

For `Bracketed<TContents>`:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<TContents: TreeContents> ::resolve_position::ResolvePosition for Bracketed<TContents> {
    type Parent<'a>
        = BracketItemParent<'a, TContents>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TContents>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        // opening: #[resolve_field(leaf = OpenBracket)] on a WithSpan field. The leaf's
        // parent converts from the container's own path; the `.into()` target is inferred
        // from the named variant's payload type.
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
        // children: #[resolve_field] on a Vec<WithSpan> field delegates.
        for item in self.children.iter() {
            if item.location.contains(position) {
                let own_path = ::resolve_position::PositionResolutionPath {
                    inner: self,
                    parent,
                };
                return item.item.resolve(own_path.into(), position);
            }
        }
        // closing: #[resolve_field(leaf = CloseBracket)] on an Option<WithSpan> field.
        for item in self.closing.iter() {
            if item.location.contains(position) {
                let own_path = ::resolve_position::PositionResolutionPath {
                    inner: self,
                    parent,
                };
                return Self::ResolvedNode::CloseBracket(
                    ::resolve_position::PositionResolutionPath {
                        inner: &item.item,
                        parent: own_path.into(),
                    },
                );
            }
        }
        Self::ResolvedNode::Bracketed(::resolve_position::PositionResolutionPath {
            inner: self,
            parent,
        })
    }
}
```

For `BracketItem<TContents>`:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<TContents: TreeContents> ::resolve_position::ResolvePosition for BracketItem<TContents> {
    type Parent<'a>
        = BracketItemParent<'a, TContents>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TContents>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            // #[resolve_field(leaf = ...)]: the payload's path is the answer; the parent
            // passes through the reflexive From.
            BracketItem::Inner(inner) => {
                Self::ResolvedNode::Inner(::resolve_position::PositionResolutionPath {
                    inner,
                    parent: parent.into(),
                })
            }
            BracketItem::Bracketed(inner) => inner.resolve(parent, position),
            BracketItem::StrayClose(inner) => {
                Self::ResolvedNode::CloseBracket(::resolve_position::PositionResolutionPath {
                    inner,
                    parent: parent.into(),
                })
            }
        }
    }
}
```

`MatchedBrackets<TContents>` keeps today's shape: the items loop delegating through `own_path.into()`, then the fallback `Self::ResolvedNode::MatchedBrackets(...)`.

The stray arm's `inner` is `&TContents::StrayClose` and the variant's payload wants `&CloseBracket`: this compiles at any stage whose slot is `CloseBracket`, which is every planned stage, and a stage that changes the slot gets a compile error at its `resolve` call sites — the error is the feature, since such a stage must decide its stray leaf then.

### Tests

The bracket suite keeps every assertion: at `BracketsMatched` the answers are unchanged. One addition: a `#[cfg(test)]` second `TreeContents` implementor whose `Inner` is a newtype run, with one test resolving into `ResolvedBracketNode::Inner` at that stage — proving the walk is generic in fact.

## Change 3: derived test extractors

`ResolvedBracketNode` gains `#[cfg_attr(test, derive(derive_more::Unwrap))]`, with `derive_more = { version = "2", features = ["unwrap"] }` as a dev-dependency (mercury in freddie is the precedent). The tests' bespoke extractors (`run`, `open_bracket`, `close_bracket`, `group_leaf`) are deleted in favor of the derived `unwrap_inner()`, `unwrap_open_bracket()`, `unwrap_close_bracket()`, `unwrap_bracketed()`; under `cfg_attr(test, ...)` the panicking methods exist only in test builds, so no production surface grows.

## Landing checklist

1. Change 1; `cargo test` green with no test edits.
2. Change 2; `cargo test` green with no assertion edits.
3. Change 3; the bespoke extractors deleted.
4. chunking.md's resolution section is implementable on top; move this doc to refactors/past.
