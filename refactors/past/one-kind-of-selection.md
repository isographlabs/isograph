# one-kind-of-selection: one `Selection` with an optional nested set

Lands after parse-fields.md. parse-variables.md and later grammar docs assume this shape.

Origin: landed `crates/isograph_parser/src/selections.rs`. Delta: `ScalarSelection` and `ObjectSelection` go away. `Selection` is the struct. `selection_set` is `Option`. Isograph keeps `SelectionType<ScalarSelection, ObjectSelection>`; a nested set is an optional field here, the same as `arguments`.

`foo { bar }` is a selection whose nested set is `None`. That is representable the same way any other optional field is.

## Change 1: `Selection`

Before:

```rust
// from crates/isograph_parser/src/selections.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ScalarSelection {
    #[resolve_field]
    #[parent_variant(ScalarSelection)]
    pub reader_alias: Option<WithSpan<SelectionNameWrapper>>,
    #[resolve_field]
    #[parent_variant(ScalarSelection)]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    #[parent_variant(ScalarSelection)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectSelection {
    #[resolve_field]
    #[parent_variant(ObjectSelection)]
    pub reader_alias: Option<WithSpan<SelectionNameWrapper>>,
    #[resolve_field]
    #[parent_variant(ObjectSelection)]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectSelection)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(ObjectSelection)]
    pub selection_set: WithSpan<SelectionSet>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectableName);

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    ClientFieldDeclaration(crate::ClientFieldDeclarationPath<'a>),
    ObjectSelection(Box<ObjectSelectionPath<'a>>),
}

#[derive(Debug)]
pub enum SelectionNameWrapperParent<'a> {
    ScalarSelection(ScalarSelectionPath<'a>),
    ObjectSelection(ObjectSelectionPath<'a>),
}

pub type ScalarSelectionPath<'a> =
    PositionResolutionPath<&'a ScalarSelection, SelectionSlotPath<'a>>;

pub type ObjectSelectionPath<'a> =
    PositionResolutionPath<&'a ObjectSelection, SelectionSlotPath<'a>>;

pub type SelectionNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectionNameWrapper, SelectionNameWrapperParent<'a>>;
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum ArgumentListParent<'a> {
    ScalarSelection(crate::ScalarSelectionPath<'a>),
    ObjectSelection(crate::ObjectSelectionPath<'a>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;
```

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
);
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Selection {
    #[resolve_field]
    pub reader_alias: Option<WithSpan<SelectionNameWrapper>>,
    #[resolve_field]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub selection_set: Option<WithSpan<SelectionSet>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectableName);

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    ClientFieldDeclaration(crate::ClientFieldDeclarationPath<'a>),
    Selection(Box<SelectionPath<'a>>),
}

pub type SelectionPath<'a> = PositionResolutionPath<&'a Selection, SelectionSlotPath<'a>>;

pub type SelectionNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectionNameWrapper, SelectionPath<'a>>;
```

`SelectionNameWrapperParent` is deleted. `name` and `reader_alias` are vanilla: their parent is `SelectionPath`.

`SelectionSetParent::Selection` is boxed to break `SelectionSetPath -> SelectionPath -> SelectionSlotPath -> SelectionSetPath`. The derive's `parent.into()` converts through `From<T> for Box<T>`. parse-pointers.md adds `ClientPointerDeclaration`.

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
);

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, SelectionPath<'a>>;
```

`ArgumentListParent` is deleted. `arguments` is vanilla. parse-directives.md introduces `ArgumentListParent` with `Selection` and `IsographFieldDirective` when directive argument lists arrive.

## Change 2: `parse_selection`

Before:

```rust
// from crates/isograph_parser/src/selections.rs
    let arguments = consume_argument_list(cursor);
    let selection_set = consume_selection_set(cursor);
    match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            arguments,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection {
            reader_alias,
            name,
            arguments,
        }),
    }
    .wrap_ok()
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
    let arguments = consume_argument_list(cursor);
    let selection_set = consume_selection_set(cursor);
    Selection {
        reader_alias,
        name,
        arguments,
        selection_set,
    }
    .wrap_ok()
```

## Change 3: resolution node

Before:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ScalarSelection(ScalarSelectionPath<'a>),
    ObjectSelection(ObjectSelectionPath<'a>),
```

After:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Selection(SelectionPath<'a>),
```

`ScalarSelectionPath` and `ObjectSelectionPath` are deleted.

## Tests

`as_scalar` / `as_object` become `as_selection`. Nested-set tests unwrap `selection_set`.

```rust
// from crates/isograph_parser/src/selections.rs
    fn as_selection(slot: &Slot<Selection, UnparsedChunkItems>) -> &Selection {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a selection")
    }
```

Every `as_scalar(...)` / `as_object(...)` call site becomes `as_selection(...)`. Where a test read `object.selection_set.item.0`, it reads `selection.selection_set.as_ref().expect("the fixture selects a nested set").item.0`.

```rust
// from crates/isograph_parser/src/selections.rs
    #[test]
    fn object_selections_nest() {
        let text = "pet { name, age }";
        let (items, errors, _) = parsed_selections(text);
        let object = as_selection(items[0].item.reference());
        assert_eq!(object.name.location, span_of(text, "pet"));
        let inner = object
            .selection_set
            .as_ref()
            .expect("the fixture selects a nested set")
            .item
            .0
            .reference();
        assert_eq!(inner.len(), 2);
        assert_eq!(
            as_selection(inner[0].item.reference()).name.location,
            span_of(text, "name")
        );
        assert_eq!(
            as_selection(inner[1].item.reference()).name.location,
            span_of(text, "age")
        );
        assert!(as_selection(inner[0].item.reference()).selection_set.is_none());
        assert_eq!(errors, vec![]);
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn as_selection(slot: &Slot<Selection, UnparsedChunkItems>) -> &Selection {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a selection")
    }

    #[test]
    fn selection_names_resolve_with_their_ancestry() {
        let text = "field Query.Foo { pet { name } }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "name")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                assert_eq!(name.parent.inner.name.location, span_of(text, "name"));
                let object = match name.parent.parent.parent.parent {
                    SelectionSetParent::Selection(object) => object,
                    parent => panic!("expected a nested-selection parent, got {parent:?}"),
                };
                assert_eq!(object.inner.name.location, span_of(text, "pet"));
                match object.parent.parent.parent {
                    SelectionSetParent::ClientFieldDeclaration(declaration) => {
                        assert_eq!(
                            declaration.inner.client_field_name.location,
                            span_of(text, "Foo")
                        );
                    }
                    parent => panic!("expected the declaration at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn argument_names_resolve_through_the_selection() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::FieldArgumentNameWrapper(name) => {
                assert_eq!(name.parent.parent.parent.parent.inner.name.location, span_of(text, "bar"));
            }
            node => panic!("expected the argument name, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableUse(_) => {}
            node => panic!("expected the variable use, got {node:?}"),
        }
    }
```

`name.parent` is `SelectionPath`. `name.parent.parent` is the selection slot. `name.parent.parent.parent` is the inner `SelectionSet`. `name.parent.parent.parent.parent` is `SelectionSetParent::Selection`.

`FieldArgumentNameWrapper.parent.parent.parent.parent` is `SelectionPath` (`ArgumentList`'s parent).

## Landing checklist

1. `Selection` as the struct, `selection_set: Option`, parent enums and path aliases, `parse_selection`, the resolution-node variant, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
