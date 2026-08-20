# Parser review

The parser is a four-stage pipeline (tokenize, match brackets, chunk, parse grammar) with real recovery and a large test suite. The grammar layer is in decent shape. The lexer and the type AST are not. `cargo test -p isograph_parser --lib` is green.

## Bugs

### Lexer: `1.5` is an integer, then rejected as overflow (fixed)

The float regex is live. `1.5`, `12.34`, `0.0`, `-1.5`, `1e2`, and `1.5e2` are `FloatLiteral`. `parse_non_constant_value` does not match that kind, so `a: 1.5` is `Expected a value, found floating point value`. `IntegerDoesNotFitI64` is overflow of an integer token only. `1.` is still `ErrorNumberLiteralTrailingInvalid`. `.5` is still `ErrorFloatLiteralMissingZero`.

### Lexer: string failures do not produce the error kinds that exist, and they do not consume the body (fixed)

`lex_string` / `lex_block_string` bump through the body and set `lexer.extras.error_token`. `tokenize` takes that extras onto the `Error` logos emits.

- `"unterminated"` → one `ErrorUnterminatedString`
- `"\"\\x\""` → one `ErrorUnsupportedStringCharacter`
- unterminated `"""` → one `ErrorUnterminatedBlockString`

`number_and_string_errors_are_their_kinds` passes. `an_unterminated_string_is_not_a_description` consumes the whole token.

### Lexer: a control character inside a block string panics (fixed)

`BlockStringToken::Error` is consumed like `Other`. A terminated block string containing U+0000 is `BlockStringLiteral`. An unterminated one is `ErrorUnterminatedBlockString`.

### Block strings are values in tests and in descriptions, not in `parse_non_constant_value` (fixed)

`parse_non_constant_value` matches `StringLiteral` or `BlockStringLiteral`. `a: """hi"""` is a string value. `a_block_string_is_a_value` passes.

### `!` is consumed and dropped. Nullability is not in the tree (moved)

Moved to type-annotation-null.md.

### `parse_iso_literal` has no single entry point and three error channels (moved)

Moved to parse-iso-literal-entry.md.

### Test suite is red on purpose in one case (fixed)

`tokenize::tests::observe_kinds` is not in the tree.

## Invariants not encoded in types

`Slot` and `parse_singleton` (deferred): parser-minor-improvements.md.

### `VariableDeclarationOrUsage` is only a declaration (moved)

Moved to variable-declaration-or-usage.md.

### `TypeAnnotation::List(Box<ListTypeAnnotation>)` vs named-struct enums (not needed)

`List(Box<ListTypeAnnotation>)` is a single named payload. `Box` is recursion so `TypeAnnotation` is sized. Not `{ inner: ... }` and not `List(A, B)`.

`Expectation::Keyword(&'static str)` and `Found::Token(NonBracketTokenKind)` are also one payload. `OneOf(&[])` displaying as `"one of"` is a Display bug, not an enum-shape bug.

### `BooleanValue(Boolean)` is two layers (not needed)

`ResolvePosition` on enums requires exactly one unnamed payload per variant. `enum BooleanValue { True, False }` cannot derive it. `BooleanValue(Boolean)` is the resolve node; `Boolean` is True/False without being leaves. `NullValue` is a unit struct because it has no payload.

### Dead token kinds sit on every match

Three different holes.

`EndOfFile` is actually dead. `tokenize` stops at the last real token. End of input is `Found::EndOfChunk`. Every `From` / `Display` / `SplitToken` match still has an `EndOfFile` arm. Delete the variant from `IsographLangTokenKind` and `NonBracketTokenKind`. Small. parser-minor-improvements.md.

`SemanticToken::Content` is leftover fill-in in leftover-semantic-tokens.md (`leftover_token` maps unparsed identifiers, `@`, `!`, `$`, and so on). Do not delete it here.

`Expectation::Description` and `Expectation::SelectionSet` are never passed to `cursor.expected`. Descriptions and selection sets are optional in the language, so those variants will not become real diagnostics without a language change. Display tests can use `Keyword` and `Selection` (`Selection` is used). Delete the two variants. Also small. parser-minor-improvements.md.

### Wrapper interned keys have inconsistent visibility (fixed)

Every interned-key wrapper field is `pub`, including `Description`. Resolve returns the wrapper; callers read `.0`.

### String / description values are lexemes, not values (moved)

Moved to string-literal-value.md. The interned payload is the GraphQL string value. The token span still includes the quotes.

## Structure

### Four trees, then the first chunk tree is thrown away and cloned back (moved)

Moved to four-trees.md. Chunking introduces `Chunk` / trailing separators; it is not a typed map of the bracket tree. Leftover contents are leftover-in-extra.md. Highlighting is leftover-semantic-tokens.md.

### Trailing separators of a parsed chunk leave the tree (moved)

Moved to leftover-in-extra.md. The comma in `entrypoint Query.foo,` goes in `Slot.extra`.

### Semantic tokens stop at the first failure in a chunk (moved)

Moved to leftover-semantic-tokens.md. `entrypoint $ $` extra is `$ $`, both `Content`. `entrypoint\nasdf` records `asdf` as `Content` from `extra_chunks`.

-----

### Keyword-as-identifier is copy-pasted

`entrypoint` / `field`, `to`, and `true` / `false` / `null` are all "require Identifier, then match the source slice." `consume_to_target` peeks, compares to `"to"`, then `require_token` with `Keyword`. `parse_boolean_or_null` records `BooleanOrNull` before checking the word, so `a: yes` highlights `yes` as boolean/null and then errors. A `consume_keyword` that records only on match would remove the duplication and the bad highlight.

### `parse_each_chunk` is the one good shared seam; tests do not use a shared harness

`consume_selection_set`, `consume_argument_list`, `consume_variable_declaration_list`, object interiors, and list interiors all go through `parse_each_chunk`. That is the right extraction.

`span_of`, `parsed_items`, and the dummy parent-cursor setup are duplicated in `arguments.rs` and `selections.rs` tests. `crates/tests` is an empty crate.

### `lib.rs` glob-exports every module (moved)

Moved to parse-iso-literal-entry.md Change 3.

### `impl std::error::Error for Expectation`

`Expectation` is a fragment of a diagnostic. `ParseError` is the error. The impl does not buy `thiserror` anything (`Expected(ExpectedFound)` already displays). Three number-error Displays are the identical string `"unsupported number (int or float) literal"`, so the three variants are indistinguishable in user text.

### Commented-out grammar in `token_kind.rs`

Spread, comments, `Pipe`, `PeriodPeriod` sit as comments, plus `TODO don't skip comments and spaces`. The crate rule is that a comment must not describe what was not done. `observe_kinds` is the same residue in test form.

## Grammar sharp edges (tested, still wrong for a GraphQL-shaped language)

Line break and comma are the same chunk separator. These are tests, not accidents:

- `field Query.Foo\n{ bar }` is a field with no selection set plus `MultipleDeclarations` on the brace.
- `bar\n{ baz }` inside a set is a scalar plus a failed selection.
- `bar\n@loadable` is a selection plus a failed selection on `@`.
- `[Pet\n!]` does not attach the bang to `Pet`.

Spaces do not split. Newlines do. Anyone who formats a selection set or a `to` clause onto the next line gets a second declaration. If that is the language, the diagnostic should say so (`expected the selection set on the same line`). It currently says `Expected nothing after the declaration`.

`#` comments are `Error` plus identifiers. There is no comment token. The skip regex skips only `[ \t\f\ufeff]+`.

## What is in good shape

Bracket matching with cut-and-diagnose is consistent and well tested. Crossing `foo { (} )` and unclosed interiors behave as documented. Chunking's `CommaWithoutItem` vs trailing comma is the right split. Per-chunk recovery (`each_malformed_variable_declaration_degrades_alone`, leftover keeps the item) is the right parser architecture. `SafePeekable` / `ItemCursor` make "peek without consume" a lifetime, not a boolean. `parse_name_colon` is the right helper for `name: value`. Resolve-position coverage on the grammar tree is thorough.

Specified work is in type-annotation-null.md, parse-iso-literal-entry.md, variable-declaration-or-usage.md, string-literal-value.md, leftover-in-extra.md, leftover-semantic-tokens.md, and four-trees.md. `Slot`, `parse_singleton`, `EndOfFile`, and unused `Expectation` variants wait in parser-minor-improvements.md. Items below the `-----` have not been processed.
