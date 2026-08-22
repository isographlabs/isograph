# Memoize encoded tokens for the file

Requires file-semantic-tokens.md and lsp-semantic-tokens-offset.md. file-semantic-tokens.md encodes every iso literal in the file in one `lsp_semantic_tokens` call. That encode is not a memo. This slice makes `lsp_semantic_tokens_for_file` a `#[memo]` on `RelativePathToSourceFile`. There is no encode of one extraction. LSP `semanticTokens/full` is one stream; the intern is that stream.

Origin: isograph issue 548 (encoded positions after typing before the literal). Origin of the walk: lsp-semantic-tokens-offset.md. Delta: the file function is a memo; prepend makes locations and file text `!=` so the encoded vec `!=`; append makes locations `==` and if extract backdates, the encoded vec `==`.

One shippable change: `#[memo]` on `lsp_semantic_tokens_for_file`.

## What the user does

No editor highlighting until the adapter. Tests intern

```
export const Home = iso(`entrypoint Query.HomeRoute`)
```

`lsp_semantic_tokens_for_file` of that path has first token `delta_line` 0, `delta_start` the UTF-16 column of `entrypoint`, `length` 11, keyword. Prefix `"const x = 1;\n"`: first token `delta_line` is 1, `delta_start` is unchanged.

## Types

```text
lsp_semantic_tokens_for_file(path)
  -> parsed_iso_literals_in_file(path)
  + locations_of_iso_literals_in_file(path)
  + DiskFile contents
  -> lsp_semantic_tokens(page_content, every (offset, tokens) in extract order)
```

The body is the same as file-semantic-tokens.md. `#[memo]` is on that function. `path` is `RelativePathToSourceFile`.

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso tokens.

## Tests

Same interned-file facts as file-semantic-tokens.md LSP tests. Add Eq after append (encoded vec equals the pre-append vec). Prefix: first token `delta_line` 1, `delta_start` equals the pre-prefix `delta_start`. Context-only `Home` -> `Page` (same length): encoded vec Eq-equals.

Do not encode a single interior against its own text.

## Call sites

- e2e-semantic-tokens.md, later adapter `semanticTokens/full`.

Amend `docs-website/docs/design-docs/pico.md` when this lands: syntax highlighting is this memo.
