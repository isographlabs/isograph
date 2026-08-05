# Error refinement: contents that change all at once

The matched-brackets tree is generic over one parameter, `TContents: TreeContents`, bundling every per-stage type: what a run between brackets is (`Inner`), what a stray close carries (`Stray`), and what a synthetic closing carries (`Unclosed`). The trait and the generic types live in `resilient-parser.md`'s Change 1; this doc is the crossing between stages and the rules for who consumes which.

One parameter rather than one per slot, because the slots change together: a pass that parses runs also decides what the errors of its output are, and separate parameters would make half-crossed trees representable that no pass produces. Crossing is one function, `try_map`: every slot is mapped fallibly, and either the whole tree crosses or the full refusal list comes back. Refinement is a `try_map` whose target stage has `Infallible` slots — after it there are no bracket-matching errors and no inner errors (the tokenizer's `Error*` kinds inside runs, and stage 4's error tokens once runs are parsed), by construction rather than by promise.

`Infallible` is std's stable never type, and this is its sanctioned direction: the refined stage's error variants are unconstructible, consumers match them with empty total matches, and nothing anywhere panics, unwraps, or claims unreachability.

## Who consumes which stage

- LSP features consume the error-containing stages, always: resilience means hover, completion, and validity work around invalid sections, so positions are only ever resolved against trees that can carry errors.
- The compiler's early passes consume the error-containing stages too, on the batch path as much as under the daemon: extraction, tokenizing, bracket matching, stage 4 parsing, and whatever validation wants to report several errors at once all operate on dirty trees.
- Refinement happens once, late, at the boundary where dirt stops being processable: artifact generation and everything after it take the refined stage and match on it totally.

## `try_map`

In `crates/isograph_parser/src/matched_brackets.rs`, beside the types. The mappers receive spans (and, for an unclosed group, the same `WithSpan<UnclosedGroup>` the error vocabulary already uses), so a refusing mapper can build its diagnostic without help. All refusals are collected — the walk continues past a failure so one crossing reports every problem — and rebuilt nodes are kept only when nothing refused, so a dropped node never reaches a consumer. For a refusal-per-bracket-error crossing, the refusal list matches `errors()` on the source tree, ordering included: a group's `Unclosed` comes before the errors inside it.

```rust
impl<TFrom: TreeContents> MatchedBrackets<TFrom> {
    /// Cross the tree to another stage: every slot mapped, fallibly. Either every node
    /// crossed, or every refusal, in source order.
    pub fn try_map<TTo: TreeContents, TError>(
        self,
        map_inner: &mut impl FnMut(WithSpan<TFrom::Inner>) -> Result<TTo::Inner, TError>,
        map_stray: &mut impl FnMut(WithSpan<TFrom::Stray>) -> Result<TTo::Stray, TError>,
        map_unclosed: &mut impl FnMut(
            TFrom::Unclosed,
            WithSpan<UnclosedGroup>,
        ) -> Result<TTo::Unclosed, TError>,
    ) -> Result<MatchedBrackets<TTo>, Vec<TError>> {
        let mut errors = Vec::new();
        let items = try_map_items(self.0, &mut errors, map_inner, map_stray, map_unclosed);
        if errors.is_empty() {
            Ok(MatchedBrackets(items))
        } else {
            Err(errors)
        }
    }
}

fn try_map_items<TFrom, TTo, TError>(
    items: Vec<WithSpan<BracketItem<TFrom>>>,
    errors: &mut Vec<TError>,
    map_inner: &mut impl FnMut(WithSpan<TFrom::Inner>) -> Result<TTo::Inner, TError>,
    map_stray: &mut impl FnMut(WithSpan<TFrom::Stray>) -> Result<TTo::Stray, TError>,
    map_unclosed: &mut impl FnMut(
        TFrom::Unclosed,
        WithSpan<UnclosedGroup>,
    ) -> Result<TTo::Unclosed, TError>,
) -> Vec<WithSpan<BracketItem<TTo>>>
where
    TFrom: TreeContents,
    TTo: TreeContents,
{
    let mut mapped = Vec::new();
    for with_span in items {
        let WithSpan { item, span } = with_span;
        match item {
            BracketItem::Inner(inner) => match map_inner(WithSpan::new(inner, span)) {
                Ok(inner) => mapped.push(WithSpan::new(BracketItem::Inner(inner), span)),
                Err(e) => errors.push(e),
            },
            BracketItem::StrayClose(stray) => match map_stray(WithSpan::new(stray, span)) {
                Ok(stray) => mapped.push(WithSpan::new(BracketItem::StrayClose(stray), span)),
                Err(e) => errors.push(e),
            },
            BracketItem::Bracketed(Bracketed {
                opening,
                closing,
                children,
            }) => {
                let closing = match closing {
                    Closing::Real(close) => Some(Closing::Real(close)),
                    Closing::Synthetic(payload) => {
                        let group = WithSpan::new(UnclosedGroup { opening }, span);
                        match map_unclosed(payload, group) {
                            Ok(payload) => Some(Closing::Synthetic(payload)),
                            Err(e) => {
                                errors.push(e);
                                None
                            }
                        }
                    }
                };
                let children = try_map_items(children, errors, map_inner, map_stray, map_unclosed);
                if let Some(closing) = closing {
                    mapped.push(WithSpan::new(
                        BracketItem::Bracketed(Bracketed {
                            opening,
                            closing,
                            children,
                        }),
                        span,
                    ));
                }
            }
        }
    }
    mapped
}
```

## Refining

A refining crossing maps both bracket-error slots to refusals; what `map_inner` does is the target stage's business. Before stage 4 exists, the two error mappers are already fully determined:

```rust
&mut |stray| Err(BracketError::UnexpectedClose(stray)),
&mut |(), group| Err(BracketError::Unclosed(group)),
```

Stage 4's doc defines its stages as implementors of the same trait — a dirty stage whose `Inner` is its node type with error tokens and whose bracket slots are `BracketKind` and `()`, and a refined stage whose `Inner` is the node type without error variants and whose bracket slots are both `Infallible`. Its refine is one `try_map` whose `map_inner` refuses on inner error tokens and whose other two mappers are the pair above, so the crossing removes matching errors and inner errors together. (If `spanless-parsing.md` is adopted, its span slot becomes a fourth associated type on the same trait, changing at the same crossings.)

## Matching on a refined stage

The error arms are required by exhaustiveness but are empty matches on an uninhabited place: total code, not a panic.

```rust
use std::convert::Infallible;

fn walk<TContents>(items: &[WithSpan<BracketItem<TContents>>])
where
    TContents: TreeContents<Stray = Infallible, Unclosed = Infallible>,
{
    for item in items {
        match &item.item {
            BracketItem::Inner(inner) => { /* ... */ }
            BracketItem::Bracketed(bracketed) => {
                match &bracketed.closing {
                    Closing::Real(_close) => { /* ... */ }
                    Closing::Synthetic(never) => match **never {},
                }
                walk(&bracketed.children);
            }
            BracketItem::StrayClose(never) => match **never {},
        }
    }
}
```

## Tests

In `crates/tests/tests/bracket_matching.rs`. The refined stage marker is test-local until stage 4 defines the real ones; `balanced` is the balanced-mixed-kinds fixture from `bracket-matching-cases.md`.

```rust
use std::convert::Infallible;

use isograph_parser::{
    BracketError, BracketsMatched, MatchedBrackets, NonBracketTokenKind, TreeContents,
};
use span::WithSpan;

#[derive(Debug, PartialEq, Eq)]
struct BracketsMatchedNoErrors;

impl TreeContents for BracketsMatchedNoErrors {
    type Inner = Vec<WithSpan<NonBracketTokenKind>>;
    type Stray = Infallible;
    type Unclosed = Infallible;
}

fn refine(
    tree: MatchedBrackets<BracketsMatched>,
) -> Result<MatchedBrackets<BracketsMatchedNoErrors>, Vec<BracketError>> {
    tree.try_map(
        &mut |tokens| Ok(tokens.item),
        &mut |stray| Err(BracketError::UnexpectedClose(stray)),
        &mut |(), group| Err(BracketError::Unclosed(group)),
    )
}

#[test]
fn a_clean_tree_refines() {
    let fixture = Fixture::load("balanced");
    assert!(refine(fixture.tree).is_ok());
}

#[test]
fn refining_reports_the_unclosed_paren() {
    let fixture = Fixture::load("unclosed_paren");
    let errors = refine(fixture.tree).expect_err("the fixture's paren never closes");
    match errors.as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.opening.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}
```
