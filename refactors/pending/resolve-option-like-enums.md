# resolve_position: option-like enums

`#[derive(ResolvePosition)]` on an enum currently requires every variant to delegate. That shuts out two enum shapes: option-like enums, where only some variants mean something is there (`Bracketed.closing` is not a `#[resolve_field]` today, so a position on a group's real close resolves to the group instead of to the close), and mixed enums, where only some variants continue into the same resolved-node family (chunking.md's `ChunkItem`, whose `SelectionSet` variant descends in the chunk query while its other variants answer the enclosing chunk). This doc adds variant-level marking to the macro's enum derive and uses it to give `Closing` a derived resolve, adding the matched-close leaf.

`closing` is already `WithSpan<Closing<TContents>>` with a real span on a real close and a zero-width span where a synthetic close should have been, so the field itself is walkable by the existing `WithSpan` field case: the wrapper's span gates entry, and a zero-width span admits no one-token-wide position. What is missing is only `Closing: ResolvePosition`. Its two variants want different answers: `Real` is a leaf (the matched close), and `Synthetic` occupies nothing, so a resolve that reaches it — possible only for an empty position sitting exactly on the zero-width span — answers with the enclosing group, the same answer the struct fallback convention gives.

## Change 1: the macro's mixed-enum derive

`ResolvePositionArgs` gains two optional idents:

```rust
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(resolve_position))]
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
    self_type_generics: Option<syn::AngleBracketedGenericArguments>,
    /// For a mixed enum: the `ResolvedNode` variant a marked unit variant answers with,
    /// holding the enum's own path.
    leaf: Option<syn::Ident>,
    /// For a mixed enum: the `ResolvedNode` variant an unmarked variant answers with,
    /// holding what the parent converts into.
    fallback: Option<syn::Ident>,
}
```

`handle_data_enum` scans variants for `#[resolve_field]`. An enum with no marks keeps today's emission (every variant delegates), so existing derives are untouched. An enum with at least one mark uses the mixed emission, with three arm kinds:

- A marked variant with a single unnamed field delegates: the position continues into the payload, which resolves in the same resolved-node family.
- A marked unit variant is a leaf: it answers the `leaf` variant with the enum's own path.
- An unmarked variant answers the `fallback` variant with `parent.into()`.

```rust
impl ::resolve_position::ResolvePosition for #enum_name #self_type_generics {
    type Parent<'a> = #parent_type;
    type ResolvedNode<'a> = #resolved_node;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span
    ) -> Self::ResolvedNode<'a> {
        match self {
            #(#enum_name::#marked_payload_variant(inner) => inner.resolve(parent, position),)*
            #(#enum_name::#marked_unit_variant => {
                Self::ResolvedNode::#leaf(self.path(parent).into())
            })*
            _ => Self::ResolvedNode::#fallback(parent.into()),
        }
    }
}
```

The fallback arm's `parent.into()` asks the caller's world for one conversion: `From<Parent>` into the fallback variant's payload. For a single-variant parent enum that is a five-line unwrap, written where the parent enum lives.

`leaf` is required iff some marked variant is a unit variant, and `fallback` is required iff some variant is unmarked; either given without its trigger is an error. A marked variant with named fields or multiple unnamed fields is an error, as today. An enum whose every variant is marked with a payload is the all-delegate emission spelled redundantly, and is rejected with an error saying to drop the marks; an enum whose every variant is a marked unit is rejected with an error naming the fix (derive the struct form instead: every variant being its own leaf means the type wants to be separate structs).

## Change 2: `Closing` resolves

In `matched_brackets.rs`. The derive, the new parent enum, and its unwrap conversion:

```rust
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ClosingParent<'a>,
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>,
    leaf = MatchedClose,
    fallback = Bracketed
)]
pub enum Closing<TContents: TreeContents> {
    /// The close bracket the author typed.
    #[resolve_field]
    Real,
    /// The group never got its close and was forced to end: at the close bracket an
    /// enclosing group owns, or at the end of the tokens. A group closed this way is an
    /// invalid section.
    Synthetic(TContents::SyntheticClose),
}

/// The one place a closing can sit: its group.
#[derive(Debug)]
pub enum ClosingParent<'a> {
    Bracketed(BracketedPath<'a>),
}

impl<'a> From<ClosingParent<'a>> for BracketedPath<'a> {
    fn from(parent: ClosingParent<'a>) -> Self {
        match parent {
            ClosingParent::Bracketed(group) => group,
        }
    }
}
```

`Bracketed.closing` gains `#[resolve_field]`, and the resolved-node enum gains the leaf:

```rust
pub enum ResolvedBracketNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(BracketedPath<'a>),
    Inner(InnerPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    MatchedClose(MatchedClosePath<'a>),
    UnmatchedClose(UnmatchedClosePath<'a>),
}

pub type MatchedClosePath<'a> =
    PositionResolutionPath<&'a Closing<BracketsMatched>, ClosingParent<'a>>;
```

The generated code, written out. For `Closing<BracketsMatched>`:

```rust
impl ::resolve_position::ResolvePosition for Closing<BracketsMatched> {
    type Parent<'a> = ClosingParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            Closing::Real => {
                Self::ResolvedNode::MatchedClose(self.path(parent).into())
            }
            _ => Self::ResolvedNode::Bracketed(parent.into()),
        }
    }
}
```

and inside `Bracketed`'s derived resolve, between the `opening` and `children` checks (field order), the existing `WithSpan` field case emits:

```rust
if self.closing.location.contains(position) {
    let new_parent = <Closing<BracketsMatched> as ::resolve_position::ResolvePosition>::Parent::Bracketed(self.path(parent).into());
    return self.closing.item.resolve(new_parent, position);
}
```

which needs no macro change: `ClosingParent` has the `Bracketed` variant the emission names, and its payload is `BracketedPath` unboxed, so the `.into()` is the identity `From` on the path.

The matcher, `errors()`, `try_map`, and the refinement slots are untouched: `Closing`'s shape does not change, only its resolvability.

## Change 3: the cases doc

`bracket-matching-cases.md`'s resolution paragraph replaces "a position on a group's real close ... resolves to the group" with: a real close resolves to `MatchedClose` with the group as its parent; interior whitespace still resolves to the group.
