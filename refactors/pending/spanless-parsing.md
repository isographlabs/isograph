# Spanless parsing: trees generic over the span slot

Whether the parse trees should be generic over their span annotation, so the same parse can produce a tree with real spans or a tree with none: parse without spans by default, and reparse with spans only when something needs positions. This doc lays out the machinery, what the span-free tree is actually worth, and the alternatives; the decision is open at the end.

The idea: `TSpan = Span` for the LSP and diagnostics, `TSpan = NoSpan` for the compile pipeline. The compile path parses spanless, asks the structure whether anything went wrong, and only when it did reparses with spans to say where.

## What a spanless tree buys, and where the benefit is real

Equality. A spanless tree's derived `PartialEq` is structural equality, and structural equality is what memoization-with-early-cutoff wants: if a recompute produces a value equal to the old one, dependents do not rerun. Spans defeat this — inserting one character shifts the span of every node after it, so a spanned tree is unequal after nearly any edit even when nothing structural changed, and everything downstream of the parse recomputes.

Two honest limits on that story at the bracket-matching level:

- `BracketItem<T>` with `T = String` carries the raw text of every run, whitespace included. An edit inside a literal changes some run's `String`, so the whole-tree spanless equality breaks on the same edits the spanned one does. Whole-tree cutoff at the bracket level is therefore nearly worthless.
- The per-subtree story is where the value is: an edit inside one bracket group changes that group's subtree, but sibling groups' spanless subtrees compare equal, where their spanned subtrees all shifted. Anything memoized per group — stage 3 parsing a group's contents, and everything derived from that — survives edits to earlier siblings only if the group's identity is span-free.

The benefit grows at each later pass: once stage 3 has parsed runs into real nodes, whitespace is gone from `T`, and a spanless AST is stable under formatting-only edits. So this decision is really about the whole pass pipeline, not about bracket matching alone — the parameter only pays if every pass's tree carries it.

One more thing the split gives regardless of memoization: the compile path can defer all position work. Errors are structural facts (`Closing::Synthetic`, `StrayClose`), so a spanless tree still knows *that* something is wrong; only *where* needs spans, and only failing compiles pay for it.

## The machinery

The `span` crate's `WithSpan` gains a defaulted parameter, so existing code is untouched (this amends what `refactors/past/parser-lang-types.md` landed, if adopted):

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WithSpan<T, TSpan = Span> {
    pub item: T,
    pub span: TSpan,
}

/// The absence of a span, for trees that exist to be compared rather than pointed into.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct NoSpan;

/// The two span annotations a parse can produce. Two implementors by design; this seam is the
/// whole of what the trait exists for.
pub trait SpanAnnotation: Copy {
    fn from_span(span: Span) -> Self;
}

impl SpanAnnotation for Span {
    fn from_span(span: Span) -> Span {
        span
    }
}

impl SpanAnnotation for NoSpan {
    fn from_span(_: Span) -> NoSpan {
        NoSpan
    }
}
```

Deferred alongside the rest of this doc, and noted here because this is the doc that generalizes the wrapper: `WithSpan<T>` may instead become an alias of a general annotated pair, `With<T, Span>` — the shape `common_lang_types`' `WithGenericLocation` already has — in which case `TSpan` above is `With`'s second parameter rather than a parameter added to `WithSpan`. Either form is the same mechanical change, decided later.

The bracket tree threads the parameter, defaulted so spanned code reads as it does today:

```rust
pub struct BracketTree<T, TSpan = Span> {
    pub items: Vec<WithSpan<BracketItem<T, TSpan>, TSpan>>,
}

pub enum BracketItem<T, TSpan = Span> {
    Text(T),
    Bracketed(Bracketed<T, TSpan>),
    StrayClose(BracketKind),
}

pub struct Bracketed<T, TSpan = Span> {
    pub opening: WithSpan<BracketKind, TSpan>,
    pub closing: Closing<TSpan>,
    pub children: Vec<WithSpan<BracketItem<T, TSpan>, TSpan>>,
}

pub enum Closing<TSpan = Span> {
    Real(TSpan),
    Synthetic,
}
```

The parser is generic at the surface and span-based inside: the lexer must track real offsets to slice text and compute ends either way, and nodes convert at construction:

```rust
pub fn match_brackets<TSpan: SpanAnnotation>(literal: &str) -> BracketTree<String, TSpan>
```

with every `item.with_span(span)` in `resilient-parser.md`'s implementation becoming `item.with_span(TSpan::from_span(span))`, and `Closing::Real(token.span)` becoming `Closing::Real(TSpan::from_span(token.span))`. Nothing else about the algorithm changes; one function serves both instantiations, so the two trees cannot disagree about structure.

What stays `Span`-only, and what works for any `TSpan`:

```rust
impl<T, TSpan> BracketTree<T, TSpan> {
    /// Whether the pass produced any error: a stray close or a synthetically closed group.
    /// Structural, so the spanless compile path can ask it.
    pub fn has_errors(&self) -> bool;
}

impl<T> BracketTree<T, Span> {
    /// The errors with their positions. Spans are what an error report is for, so this lives
    /// only on the spanned tree.
    pub fn errors(&self) -> Vec<BracketError>;
}

// Position resolution is meaningless without positions.
impl<T> ResolvePosition for BracketTree<T, Span> { ... }
```

The compile path's flow, which is the sentence this doc exists to evaluate:

```rust
let tree = match_brackets::<NoSpan>(literal);
if tree.has_errors() {
    // Only failing literals pay for positions.
    let spanned = match_brackets::<Span>(literal);
    report(spanned.errors());
}
```

The grammar stage's token collector is the same split. The parse is constructed with `NoSemanticTokens::new()` (cheap pass) or `CollectedSemanticTokens::new()` (the reparse the LSP asks for), specified in semantic-tokens.md change 2. The entry point chooses `TSpan` and `TTokens` by which values it constructs. `require_token` names neither.

## Alternative A: one spanned parse, strip when needed

Keep `match_brackets` producing `BracketTree<String, Span>` only, and derive the spanless tree by a trivial recursive map:

```rust
pub fn strip_spans<T>(tree: BracketTree<T, Span>) -> BracketTree<T, NoSpan>
```

The type genericity stays (both instantiations exist); the parse does not. One parser, no `SpanAnnotation` bound on it, and no possibility of the two modes drifting. The cost is that the compile path always pays for spans and then throws them away — which the small-literal assumption says is nothing — and that the memoization boundary must call `strip_spans` explicitly. Under this alternative "reparse with spans iff needed" inverts into "strip spans where cutoff needs it", which is the same capability with one parse instead of two.

## Alternative B: spanned trees with span-blind equality

No new types: keep spans everywhere and hand-write `PartialEq`/`Hash` to ignore them, so cutoff sees structural equality. This is the least code and the most dangerous: two trees that compare equal carry different spans, so a memoization layer may hand back a cached tree whose spans describe an older revision of the literal. Any consumer that reads positions out of a memoized tree then points at the wrong bytes, and nothing in the types says so. Spans that lie are worse than no spans; this alternative is listed to record why it loses to the other two.

## Costs common to adopting the parameter

- `TSpan` threads through every tree type of every pass, forever, the way `T` already does. Each new pass doc writes its types twice-instantiable or the chain breaks and the benefit stops at that pass.
- `resolve_position`, `errors()`, the harness, and everything else that points into text is constrained to the `Span` instantiation; the docs and impls carry that split.
- The LSP path, which is the daemon's steady state, always wants spans — so the spanless instantiation exists for the compile pipeline and for memoization keys, and the "default" mode depends on which consumer you stand in.

## The decision, and what it hangs on

Open. What has to be true for the parameter to pay:

- pico's memoization must actually cut off on equal values (dependents skip when a recomputed value equals the cached one). If it does not, spanless equality buys nothing and Alternative A's `strip_spans` at specific boundaries — or nothing at all — is the whole story. Verify against pico before adopting.
- Stage 3 must adopt the same parameter, because the bracket-level benefit alone (per-sibling-group stability) is thin while `T = String` carries whitespace. The real payoff is a spanless AST that is stable under formatting edits.
- The per-group memoization design (which pass keys on what) has to exist, since it decides whether cutoff happens at whole-literal or per-group granularity.

This doc should be decided together with the stage 3 doc and the first doc that puts pico above the parser. Until then, `resilient-parser.md` stays as written — spans always — and adopting this doc later means the mechanical change described above plus its harness split.
