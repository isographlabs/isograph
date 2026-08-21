# leftover-semantic-tokens blockers

Fold these into leftover-semantic-tokens.md before implementing. Each item is independently shippable leftover-semantic-tokens.md text.

WIP parser implementation is git stash `wip leftover-semantic-tokens`. leftover_token, extra walk, extra_chunks walk, leftover roles on leftover fixtures. 273 pass, 1 fail: `a_second_bang_is_leftover` (item 1). leftover-in-extra.md is in past.

## 1. Second bang leftover span

`field Query.Foo to Pet!! { id }` parses. The first `!` is consumed as nullability and is not recorded. The second `!` is leftover Content. `{ id }` is leftover Bracket / Content / Bracket.

`a_second_bang_is_leftover` already names the leftover error span:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        let text = "field Query.Foo to Pet!! { id }";
        let second_bang = Span::new(
            span_of(text, "Pet!!").start + 4,
            span_of(text, "Pet!!").start + 5,
        );
```

`parsed()` / `assert_semantic_tokens` finds expected lexemes left to right from the previous match end. After `"Pet"`, `"!"` is the first bang.

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
        let offset = text[search_from..]
            .find(pattern)
            .expect("the expected lexeme occurs in the fixture after the previous token");
```

How to find it: leftover-semantic-tokens records Content at `second_bang`. `(SemanticToken::Content, "!")` in the `parsed()` list expects Content at the first bang. `displayed` prints `(Content, "!")` for both. Spans differ. The test fails.

leftover-semantic-tokens.md leftover token list for this fixture does not use `(SemanticToken::Content, "!")`. Leftover Content is at `second_bang`. Assert that token separately, or omit `"!"` from the sequential list.

HEAD `a_second_bang_is_leftover` expected tokens stop at `"Pet"` and pass.

## 2. leftover_at is a directive

leftover-semantic-tokens.md leftover_at_after_an_entrypoint_is_content:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        let text = "entrypoint Query.foo @lazy";
```

That fixture parses as a directive. `@` and `lazy` are DirectiveName, not leftover Content.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        let text = "entrypoint Query.foo @lazyLoad";
        // Keyword, Type, Period, FieldName, DirectiveName "@", DirectiveName "lazyLoad"
```

Delete leftover_at_after_an_entrypoint_is_content. Leftover `@` is a fixture the grammar does not consume as a directive, not this one.

## 3. Tests use `parsed()`

parse-test-semantic-tokens landed. leftover-semantic-tokens.md leftover tests still write `parse_iso_literal` and `assert_eq` on `parsed.tokens`. Tests call `parsed(text, expected_tokens)`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn parsed(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<AstError>>) {
```

Rewrite leftover-semantic-tokens.md leftover tests to that helper.

`leftover_after_an_entrypoint_is_not_recorded` is already `tokens_after_a_complete_entrypoint_are_leftover`. Update that test's token list. Do not add leftover_after_an_entrypoint_is_content as a second copy.

Every leftover fixture that already has a token list gains leftover roles. The stash already did that except item 1.
