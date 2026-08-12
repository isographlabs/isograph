# Unclosed-group recovery

How the pipeline behaves while a group is unclosed, which is the LSP's steady state: the user has typed `{` and not yet `}`. The decision here is open; this doc records the problem and the candidate directions, to be decided after the grammar stage exists and real LSP behavior can be judged.

## The problem

The bracket matcher demotes an unclosed group: the opening becomes a raw (unmatched) token and the would-be children return to the parent level. The grammar stage then stops at unmatched tokens (parsing-standards.md's unmatched rule), so everything from the unclosed `{` to its chunk's end is unreachable. For a stray close bracket this is exactly right: `{ foo, bar) }` parses every selection, and the bracket stage's one error tells the truth. For an unclosed open bracket it is not:

```
field Query.Foo {
  bar
```

The `{` demotes and is skipped, so the root level holds two chunks, `field Query.Foo` and `bar`. The header chunk reports `Expected(a selection set, found nothing more)`; the `bar` chunk reports `MultipleDeclarations`; the bracket stage reports the unclosed `{`. One mistake, three reports, and only the third names it. The selections the user is actively typing are not in the grammar tree at all, so completion, go-to-definition, and find-references inside the braces have nothing to stand on, precisely while the user is mid-keystroke.

The asymmetry: demotion plus skip is tuned for stray closes, and the unclosed open, the frequent editing state, gets the worst of both.

## Candidate directions

1. Synthetic closing in the bracket matcher. An unclosed group closes at its level's end instead of demoting: the group survives into the tree with a real opening and a synthetic closing, its interior chunks and parses normally, and the bracket stage's unclosed error remains the one report. The literal above parses as a field declaration with the selection `bar`. This is the `Closing::Synthetic` design the bracket docs weighed before choosing demotion (refactors/past/bracket-matching-cases.md, resilient-parser.md); revisiting it means the closing slot becomes an enum (a real span or nothing), which ripples through the chunk tree, resolution, and every consumer that reads `closing`. Crossing-junk cases (`foo { (} )`) still need demotion or a rule of their own.
2. Rendering-side suppression. Keep demotion and skip, and have the rendering stage suppress grammar errors that follow a demoted opening in the same literal. This removes the misleading reports but does not put the selections in the tree, so the LSP gains nothing; it treats the symptom.
3. Accept the noise. Keep everything as designed; the bracket error is present and correct, and the extra reports are tolerated as a mid-keystroke transient.

Direction 1 is the only one that helps the LSP, and its cost is a real bracket-stage redesign. Whether that cost is paid, and whether the grammar stage's skip rule then narrows to stray closes only, is the future decision this doc holds.
