# Concat exists to feed encoded LSP tokens

The product is editor syntax highlighting: `lsp_semantic_tokens_for_file` returning `Vec<lsp_types::SemanticToken>` for `semanticTokens/full`. File-absolute `WithSpan` tokens are not a product intern.

file-semantic-tokens.md ships concat (`iso_literal_semantic_tokens_in_file`) because the encode wrapper in that slice reads it. That is how highlighting gets out before semantic-tokens-line-offset.md. Not a blocker. Ship concat.

semantic-tokens-line-offset.md encodes each literal against its own text, then a path-keyed memo offsets those LSP deltas into document coordinates. That path memo does not read concat. When it lands, production encode stops calling concat.

Compiler tests in file-semantic-tokens.md that assert `Keyword` at a file byte offset may keep calling concat, or they may assert relative tokens plus locations. Decide then. Do not keep concat in production for the sake of file-absolute spans.

pico.md may name concat as the highlighting intern while file-semantic-tokens.md is the encode path. When line-offset lands, highlighting is the path-keyed encoded memo. Amend pico.md then.
