# Semantic tokens

How the LSP's highlighting derives from the parse, for the stage that lands with LSP work; not part of the parsing series. The parsing docs constrain it only by keeping every span the derivation needs.

## The rule: highlight from the deepest layer that has meaning

Every token in the literal is classified by exactly one of two layers, and errors are a third channel beside them:

1. Semantic: a token covered by a parsed grammar node gets that node's classification. `field` and `entrypoint` are keywords, an `EntityName` is a type, a `SelectionName` is a member, a `VariableName` is a variable, an `IntegerValue` is a number, and so on, one legend entry per grammar leaf; the mapping is a match over `IsographResolutionNode`'s grammar variants.
2. Lexical: a token no grammar node covers, because it sits in the matcher's cut, in an unparsed chunk, or in a trailing-junk suffix, gets its token kind's classification: identifier as identifier, number as number, string as string. Tokens qua tokens, with no claim about roles the parse never established.
3. Errors are diagnostics, not token classes: a bracket error's span (the matcher's vec) and each grammar error's span get squiggles, and no semantic-token legend entry exists for "error".

So `foo ( asfd`: `foo` parsed as a selection and highlights as one; the `(` carries the unclosed-bracket diagnostic; `asfd` sits in the cut and highlights as a bare identifier. Highlighting degrades from meaningful to lexical exactly where meaning ended, and never to nothing, so a mid-typing literal stays readable.

## Derivation

Both layers derive after parsing, from data the pipeline already has: the token list (`tokenize`), the grammar tree, and the matcher's error vec. One walk over the grammar tree emits the semantic spans; one pass over the token list classifies every token those spans did not cover. Nothing is accumulated during parsing, which is the deliberate reversal of upstream, where `semantic_token_legend` constants thread through every parse call and couple the parser to the legend.

The exact legend, the leaf-to-legend mapping in full, and the emission code are specified when this doc becomes active, against whatever the LSP protocol work needs; the layering rule above is the part the parsing series is built to support and must not be violated by it.
