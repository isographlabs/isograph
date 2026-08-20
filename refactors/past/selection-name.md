# selection-name: `SelectionNameWrapper` wraps `SelectionName`

A selectable is a definition: `FieldDeclaration.name` and `EntrypointDeclaration.name` are `SelectableNameWrapper(SelectableName)`. A selection is an item in a selection set: `Selection.name` and `reader_alias` are `SelectionNameWrapper(SelectionName)`. The interned keys are different types. A later pass checks that a selection name refers to a selectable that exists.

`SelectionNameWrapper` wraps `common_lang_types::SelectionName`, not `SelectableName`.

Lands after selectable-name-wrapper.md, before optional-to.md. No grammar change.

Origin: `SelectionNameWrapper` in `crates/isograph_parser/src/selections.rs`. Delta: inner type `SelectionName`. Origin for the interned key: none in isograph; `string_key_newtype!(SelectionName)` in `crates/common_lang_types/src/string_key_types.rs`.

## Changes to common_lang_types

```rust
// from crates/common_lang_types/src/string_key_types.rs
string_key_newtype!(SelectionName);
```

Sits with `FieldArgumentName` and `VariableName`. `string_key_newtype!` supplies `From<StringKey>`. No conversion to or from `SelectableName`.

## Changes to SelectionNameWrapper

Before:

```rust
// from crates/isograph_parser/src/selections.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectableName);
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectionName);
```

Construction is unchanged: `first.interned().map(SelectionNameWrapper)`. Tests that write `SelectionNameWrapper("bar".intern().to())` stay; `.to()` infers `SelectionName`.

`SelectableNameWrapper` still wraps `SelectableName`.



## Tests

Existing selection name and alias tests. Behavior is unchanged. `cargo test -p isograph_parser` and `cargo test -p common_lang_types` pass.

## Landing checklist

1. `SelectionName` in string_key_types.rs, the wrapper inner type, the AGENTS.md sentences. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
