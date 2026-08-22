# What concat is after encoded tokens use a line offset

Not a blocker for file-semantic-tokens.md. Ship concat. Revisit when semantic-tokens-line-offset.md is in discussion or about to land.

file-semantic-tokens.md adds `iso_literal_semantic_tokens_in_file` (concat): parse tokens with spans relative to the literal, plus `locations_of_iso_literals_in_file`, `with_offset` on each span, one `Vec<WithSpan<IsographSemanticToken>>` in file coordinates. `lsp_semantic_tokens_for_file` encodes that vec against the file text. That encode is not a memo. Concat is the only production input to encode.

semantic-tokens-line-offset.md encodes each literal against its own text (relative LSP deltas, keyed on the iso text). A path-keyed memo rewrites those deltas using the line and column of each `iso_literal_start_index`. That path memo does not read concat. After it lands, concat’s production reader is gone. Compiler tests that `entrypoint` is `Keyword` at a file byte offset still read concat.

The question to revisit: is concat a lasting intern, or only a stepping stone so file-semantic-tokens.md can encode before the line-offset slice exists?

Lasting: file-absolute `WithSpan` tokens stay a value (tests in file bytes, any later pass that wants file spans without LSP deltas). Line-offset is a second intern, for encoded tokens. pico.md should not call concat syntax highlighting. Highlighting is the encoded vec.

Stepping stone: concat exists so this slice can call `lsp_semantic_tokens` now. When line-offset lands, production stops calling concat. pico.md should not describe concat as the highlighting intern. Tests that need file-absolute spans either keep concat as a tests-only memo or assert relative tokens plus locations instead.

pico.md today names concat as syntax highlighting. That matches file-semantic-tokens.md and does not match line-offset. Amend pico.md for locations when file-semantic-tokens.md lands. Amend it again when line-offset lands, once this question is decided.
