# PathParent: project `PositionResolutionPath`'s `Parent` type argument

Lands after resolve-position-on-unmatched-span.md (refactors/past). generic-slot.md uses this.

`T::Parent::Parent` is not legal. `T::Parent` is a `PositionResolutionPath<Inner, Parent>`. That `Parent` is a struct type argument. This trait is that projection.

```rust
// from crates/resolve_position/src/lib.rs
pub trait PathParent {
    type Parent;
}

impl<Inner, Parent> PathParent for PositionResolutionPath<Inner, Parent> {
    type Parent = Parent;
}
```

A type alias of `PositionResolutionPath` implements it. `SlotPath` is `PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>`, so `<SlotPath<'a> as PathParent>::Parent` is `IsoLiteralParsePath<'a>`.

`()` and parent enums do not implement it.

```rust
// from crates/resolve_position/src/lib.rs
    #[test]
    fn position_resolution_path_projects_its_parent_type_argument() {
        fn assert_parent<T: PathParent<Parent = U>, U>() {}
        assert_parent::<PositionResolutionPath<&u8, ()>, ()>();
    }
```

`cargo test -p resolve_position` passes.

## Landing checklist

1. `PathParent` and the impl on `PositionResolutionPath`, the test above. `cargo test -p resolve_position` passes.
2. Move this doc to refactors/past.
