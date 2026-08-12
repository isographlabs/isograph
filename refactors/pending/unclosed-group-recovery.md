# Unclosed-group recovery

A future optimization over cut-at-unmatched.md, deferred until the grammar stage exists and real LSP behavior can be judged.

## The problem the cut leaves

Under the cut, an unclosed bracket drops everything after it in its level. The LSP's steady state is the user mid-typing, `{` typed and `}` not yet:

```
field Query.Foo {
  bar
```

The unclosed `{` cuts the root level, so the tree is `field Query.Foo`: the header then degrades for want of a selection set, and the selections being typed are not in the tree, so completion, go-to-definition, and find-references inside the braces have nothing to stand on, precisely while the user is typing there. The same shape drops `{ bar, baz }` from `foo( { bar, baz }`.

## The optimization: synthetic closing

An unclosed group closes synthetically at its level's end instead of cutting: `closing` becomes `Option<WithSpan<CloseBracket>>`, the group survives with its interior, and the matcher's `UnmatchedOpen` error remains the one report. The literal above parses as a field declaration with the selection `bar`, and `foo( { bar, baz }` parses as `foo` with an argument group whose interior degrades locally.

Costs, which are why this waits: `closing` becomes optional everywhere it is read (the bracket tree, the chunk tree, resolution, every consumer), the crossing-junk cases (`foo { (} )`) need re-deciding under synthesis, and the matcher, chunking, resolution, and their tests all churn. Where the synthetic close goes is also a sub-decision: the level's end is the structural rule; closing earlier (before a `{`, recovering `field Query.Foo( {` as a header typo) is a heuristic that would misfire on object-literal arguments.
