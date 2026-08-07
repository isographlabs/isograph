# Generic resolution

`ResolvedBracketNode` becomes generic over the stage, and `MatchedBrackets<TContents>` resolves at every stage through one generic walk. The enum stays closed at five variants. `Inner(InnerPath<'a, TContents>)` is the tree-level answer for a position in a run; a stage whose run type has interior structure answers the finer question with a second resolve on the run itself, whose parent is the `InnerPath` from the first answer, so ancestry chains across the two queries (chunking.md's `ChunkedRun` is the first such run type). Leaf types (`Inner`, `OpenBracket`, `CloseBracket`) stop implementing `ResolvePosition` at the tree level; the walk constructs their paths from outside, converting each leaf payload through `std::borrow::Borrow`, which is reflexive for free and lets the generic stray arm hand the concrete `&CloseBracket` to the one `CloseBracket` variant.

Two changes: the macro emits generic impls with leaf modes, and the test extractors become derived.

## The stray slot borrows to the close token

The `StrayClose` leaf arm holds `&TContents::StrayClose` and the `CloseBracket` variant's payload holds `&CloseBracket`; the bound that connects them at every stage is `Borrow<CloseBracket>` on the slot. std's blanket `Borrow<T> for T` covers every stage whose slot is `CloseBracket`, and a stage whose slot is `Infallible` writes the vacuous impl — legal under the orphan rule because `CloseBracket` is local, and its `match *self {}` asserts an invariant the type system itself expresses: no value of `Infallible` exists, and no tree at such a stage can hold a stray to resolve.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined.
    type StrayClose: fmt::Debug + PartialEq + Eq;
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined. Every carrier borrows to the token, so the stray leaf
    /// answers the concrete `CloseBracket` variant at every stage.
    type StrayClose: fmt::Debug + PartialEq + Eq + std::borrow::Borrow<CloseBracket>;
```

## Change 1: the macro emits generic impls with leaf modes

In `resolve_position_macros`, `resolve_position`, and the derive sites. One unit.

### Attribute surface

`self_type_generics` is deleted, and with it the `map_generics` module: the derive emits one impl over the type's own generics via `split_for_impl`, the way freddie's bind_macro does. `#[resolve_field]` keeps marking what resolution descends into. `#[resolve_field(leaf = VariantName)]` marks a field or enum variant whose payload is a leaf: the emission constructs the payload's path and wraps it in the named `ResolvedNode` variant, delegating nothing, and the payload type needs no `ResolvePosition` impl. Every leaf payload passes through `::std::borrow::Borrow::borrow`, with the target inferred from the variant's payload type: reflexive and free at concrete sites, and the conversion that lets a slot-typed payload answer a concrete variant.

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

### matched_brackets.rs conversions

The generic `MatchedBrackets` impl delegates through `own_path.into()` like every other container, so `BracketItemParent` carries the root conversion beside the group one:

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
                inner: ::std::borrow::Borrow::borrow(&self.opening.item),
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
                        inner: ::std::borrow::Borrow::borrow(&item.item),
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
                    inner: ::std::borrow::Borrow::borrow(inner),
                    parent: parent.into(),
                })
            }
            BracketItem::Bracketed(inner) => inner.resolve(parent, position),
            BracketItem::StrayClose(inner) => {
                Self::ResolvedNode::CloseBracket(::resolve_position::PositionResolutionPath {
                    inner: ::std::borrow::Borrow::borrow(inner),
                    parent: parent.into(),
                })
            }
        }
    }
}
```

`MatchedBrackets<TContents>` keeps today's shape: the items loop delegating through `own_path.into()`, then the fallback `Self::ResolvedNode::MatchedBrackets(...)`.

### Tests

The bracket suite keeps every assertion: at `BracketsMatched` the answers are unchanged. One addition: a `#[cfg(test)]` second `TreeContents` implementor whose `Inner` is a newtype run, with one test resolving into `ResolvedBracketNode::Inner` at that stage — proving the walk is generic in fact.

## Change 2: derived test extractors

`ResolvedBracketNode` gains `#[cfg_attr(test, derive(derive_more::Unwrap))]`, with `derive_more = { version = "2", features = ["unwrap"] }` as a dev-dependency (mercury in freddie is the precedent). The tests' bespoke extractors (`run`, `open_bracket`, `close_bracket`, `group_leaf`) are deleted in favor of the derived `unwrap_inner()`, `unwrap_open_bracket()`, `unwrap_close_bracket()`, `unwrap_bracketed()`; under `cfg_attr(test, ...)` the panicking methods exist only in test builds, so no production surface grows.

## Landing checklist

1. Change 1; `cargo test` green with no assertion edits.
2. Change 2; the bespoke extractors deleted.
3. chunking.md's resolution section is implementable on top; move this doc to refactors/past.
