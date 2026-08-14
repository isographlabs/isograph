# no-final-comma: one-item contexts reject a trailing comma

The rule: a comma is meaningful only inside a list, so a one-item context admits no comma. `parse_singleton` (parsing-standards.md) is that check: after the production succeeds and the chunk is exhausted, `boundary_comma` is an `Expected(end_expectation, Token(Comma))` error at the comma.

The two one-item contexts and the tests that cover them:

```
entrypoint Query.foo,        <- parse-entrypoint.md, a_final_comma_after_the_declaration_is_an_error
field Query.Foo { bar },     <- parse-fields.md, a_final_comma_after_the_field_declaration_is_an_error
pointer Pet.B to Pet { x },  <- parse-pointers.md, a_final_comma_after_the_pointer_declaration_is_an_error
field Query.Foo($x: [Pet,])  <- parse-variables.md, a_final_comma_inside_a_list_type_degrades_that_declaration
```

Trailing commas inside lists stay legal. parse-fields.md and parse-arguments.md already parse `{ bar, }`, `(a: 1,)`, `{ id: 4, }`.

This doc adds no code. It exists so the test matrix is in one place. When the four feature docs have landed, move this doc to refactors/past.

## Landing checklist

1. The four tests above exist in their feature docs; `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
