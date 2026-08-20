# optional-field-selection-set: `field Type.name` with no `{ }`

A field declaration's selection set is optional. `field Foo.Bar` is a complete declaration. `selection_set` is `Option<WithSpan<SelectionSet>>`. `parse_selectable_declaration` uses `consume_selection_set`. `require_selection_set` is deleted.

Lands after selectable-declaration.md, before parse-directives.md. The selection set is still required.

Origin: selectable-declaration.md after. Delta: `selection_set` is `Option`; `consume_selection_set` at the field. `require_selection_set` is deleted.

## Changes to SelectableDeclaration

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub selection_set: Option<WithSpan<SelectionSet>>,
```

## Changes to parse_selectable_declaration

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let target_type = consume_to_target(cursor)?;
    let description = consume_description(cursor);
    let selection_set =
        require_selection_set(cursor, TO_OR_DESCRIPTION_OR_SELECTION_SET)?;
    SelectableDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        variable_definitions,
        target_type,
        description,
        selection_set,
    }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let target_type = consume_to_target(cursor)?;
    let description = consume_description(cursor);
    let selection_set = consume_selection_set(cursor);
    SelectableDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        variable_definitions,
        target_type,
        description,
        selection_set,
    }
```

`consume_selection_set` is `pub(crate)`. `require_selection_set` is deleted. `parse_selectable_declaration` stays `Result` through `consume_to_target`.

After `Type.name`, variables, and optional `to`, description and `{` are both optional. Junk in the same chunk is leftover: `Expected(EndOfDeclaration, ...)`. `field Query.Foo Owner { id }` parses `Foo` and reports `EndOfDeclaration` at `Owner`. `TO_OR_DESCRIPTION_OR_SELECTION_SET` is unused and deleted.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_field_declaration_without_a_selection_set_parses() {
        let text = "field Query.Foo";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(as_selectable(parse.reference()).selection_set, None);
    }

    #[test]
    fn a_field_declaration_with_only_a_description_parses() {
        let text = "field Query.Foo \"the home route\"";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.description.is_some());
        assert_eq!(declaration.selection_set, None);
    }
```

`a_field_declaration_without_a_selection_set_is_a_failed_item` is deleted. `a_description_without_a_selection_set_is_a_failed_item` is deleted.

`a_selection_set_split_onto_its_own_line_is_a_failed_item` becomes a complete field plus `MultipleDeclarations` on the brace chunk:

```rust
    #[test]
    fn a_selection_set_on_its_own_line_is_a_second_declaration() {
        let text = "field Query.Foo\n{ bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(as_selectable(parse.reference()).selection_set, None);
        assert_eq!(
            errors,
            ParseError::MultipleDeclarations
                .with_span(span_of(text, "{ bar }"))
                .wrap_vec(),
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }
```

Tests that read `declaration.selection_set.location` or `selections(declaration.selection_set.reference())` go through `as_ref().expect("the fixture writes a selection set")`.

## Landing checklist

1. `SelectableDeclaration.selection_set`, `parse_selectable_declaration`, delete `require_selection_set`, the test replacements. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
