# No concat intern

Superseded. file-semantic-tokens.md does not intern file-absolute `WithSpan<IsographSemanticToken>`. `lsp_semantic_tokens` (lsp-semantic-tokens-offset.md) takes each literal’s start offset and the file text. `lsp_semantic_tokens_for_file` maps `parsed_iso_literals_in_file` plus `locations_of_iso_literals_in_file` into that call.

semantic-tokens-line-offset.md is still the intern that reuses encoded tokens after a prepend.
