# expectation-one-of: `Expectation::OneOf`

A slot that accepts several forms is `Expectation::OneOf`, not a unit variant whose message lists them. `DeclarationKeyword` becomes `OneOf` of `Keyword("entrypoint")` and `Keyword("field")`. Landed `ToOrDescriptionOrSelectionSet` becomes `OneOf` of `Keyword("to")`, `Description`, and `SelectionSet`.

Lands before optional-field-selection-set.md. No grammar change. selection-name.md and optional-to.md are past.

## Sweep

Slots that skip optionals and then `require` the last item report only that last item. Landed:

- `parse_selectable_declaration`: `consume_to_target`, `consume_description`, then `require_selection_set` with `ToOrDescriptionOrSelectionSet`. That unit variant is this doc's `OneOf`. optional-field-selection-set.md later makes the selection set optional.
- Variable defaults: `consume_token_if(Equals)`, then `parse_non_constant_value`. Junk that is not `=` is leftover in the variable chunk. No `require` of a union.
- `!` after a type: `consume_token_if(Exclamation)`. Junk after the type is leftover.
- Selection arguments and nested sets: both `consume_*`. Leftover is `Separator`.

Forms that already name every alternative in one required slot stay unit variants: `Value`, `TypeAnnotation`, `Separator(BracketKind)`. Their messages are the alternatives. They are not skip-then-require.

`DeclarationKeyword` is a required slot with two keyword texts. It becomes `OneOf`.

## Changes to parse_error.rs

`Expectation` drops `thiserror` on the enum. `Display` is manual so `OneOf` can join.

```rust
// from crates/isograph_parser/src/parse_error.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Token(NonBracketTokenKind),
    Keyword(&'static str),
    Description,
    OneOf(&'static [Expectation]),
    EndOfDeclaration,
    SelectionSet,
    Selection,
    Separator(BracketKind),
    Argument,
    Value,
    ObjectEntry,
    VariableDeclarationOrUsage,
    TypeAnnotation,
    EndOfType,
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Expectation::Token(kind) => write!(f, "{kind}"),
            Expectation::Keyword(word) => write!(f, "the keyword `{word}`"),
            Expectation::Description => write!(f, "a description"),
            Expectation::OneOf(items) => write_one_of(f, items),
            Expectation::EndOfDeclaration => write!(f, "the end of the declaration"),
            Expectation::SelectionSet => write!(f, "a selection set, like '{{ id, name }}'"),
            Expectation::Selection => write!(f, "a field selection"),
            Expectation::Separator(kind) => {
                write!(f, "a comma, a line break, or {}", kind.closing())
            }
            Expectation::Argument => write!(f, "an argument, like 'id: $id'"),
            Expectation::Value => write!(
                f,
                "a value, like $foo, 42, \"bar\", true, false, null, or an object literal"
            ),
            Expectation::ObjectEntry => write!(f, "an object entry, like 'id: 4'"),
            Expectation::VariableDeclarationOrUsage => {
                write!(f, "a variable declaration, like '$id: ID!'")
            }
            Expectation::TypeAnnotation => {
                write!(f, "a type, like 'String', 'String!', or '[String]'")
            }
            Expectation::EndOfType => write!(f, "the end of the type"),
        }
    }
}

impl std::error::Error for Expectation {}

fn write_one_of(f: &mut fmt::Formatter<'_>, items: &[Expectation]) -> fmt::Result {
    match items {
        [] => write!(f, "one of"),
        [item] => write!(f, "{item}"),
        [first, second] => write!(f, "{first} or {second}"),
        [start @ .., last] => {
            for (i, item) in start.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{item}")?;
            }
            write!(f, ", or {last}")
        }
    }
}
```

`OneOf` is a slice of two or more atoms. Nested `OneOf` is not constructed. The empty and singleton arms exist so `Display` is total.

```rust
// from crates/isograph_parser/src/parse_error.rs
pub const DECLARATION_KEYWORD: Expectation = Expectation::OneOf(&[
    Expectation::Keyword("entrypoint"),
    Expectation::Keyword("field"),
]);
```

Landed `parse_iso_literal_item` still special-cases `"pointer"` as `UnsupportedDeclarationType`. The `_` arm and the identifier `require` use `DECLARATION_KEYWORD` in place of `Expectation::DeclarationKeyword`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(DECLARATION_KEYWORD))?;
```

```rust
        _ => ParseError::expected(
            DECLARATION_KEYWORD,
            Found::Token(NonBracketTokenKind::Identifier),
        )
```

`expectation_unit_variants_use_their_messages` asserts `DECLARATION_KEYWORD` and `Keyword`:

```rust
// from crates/isograph_parser/src/parse_error.rs
        assert_eq!(
            DECLARATION_KEYWORD.to_string(),
            "the keyword `entrypoint` or the keyword `field`",
        );
        assert_eq!(
            Expectation::Keyword("to").to_string(),
            "the keyword `to`",
        );
```

The landed assertion `"one of \`entrypoint\` or \`field\`"` is deleted. `"pointer"` already uses `DeclarationKeyword`; that call becomes `DECLARATION_KEYWORD`.

```rust
// from crates/isograph_parser/src/parse_error.rs
pub const TO_OR_DESCRIPTION_OR_SELECTION_SET: Expectation = Expectation::OneOf(&[
    Expectation::Keyword("to"),
    Expectation::Description,
    Expectation::SelectionSet,
]);
```

Landed `require_selection_set(cursor, Expectation::ToOrDescriptionOrSelectionSet)` becomes `require_selection_set(cursor, TO_OR_DESCRIPTION_OR_SELECTION_SET)`. The unit variant `ToOrDescriptionOrSelectionSet` is deleted.

```rust
        assert_eq!(
            TO_OR_DESCRIPTION_OR_SELECTION_SET.to_string(),
            "the keyword `to`, a description, or a selection set, like '{ id, name }'",
        );
```

## Tests

Unknown keyword `fieldd` still reports at that identifier. The expected side is `DECLARATION_KEYWORD`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Token(Identifier)),
            span_of(text, "fieldd"),
        );
```

A group at the start of a literal: `expected(DECLARATION_KEYWORD, Found::Group(BracketKind::Brace))`.

## Landing checklist

1. `Expectation` `Display`, `OneOf`, `Keyword`, `Description`, `DECLARATION_KEYWORD`, `TO_OR_DESCRIPTION_OR_SELECTION_SET`, delete `DeclarationKeyword` and `ToOrDescriptionOrSelectionSet`, call sites, tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
