use std::fmt;
use std::iter::Peekable;

use span::{Span, WithSpan};

use crate::{BracketKind, BracketToken, IsographLangTokenKind, NonBracketTokenKind, SplitToken};

/// The types one matched-brackets tree holds: what a run between brackets is, and what the
/// two bracket errors carry. A pipeline stage is an implementor, and a pass that changes any
/// of these changes all of them at once, through `try_map` (error-refinement.md). The slots
/// carry these bounds so the tree types' derives compile.
pub trait TreeContents {
    /// What a run between brackets is. The matcher produces
    /// `Vec<WithSpan<NonBracketTokenKind>>`; later passes replace it with parsed nodes.
    type Inner: fmt::Debug + PartialEq + Eq;
    /// What a stray close carries. Dirty stages use `BracketKind`; refined stages use
    /// `Infallible`, which makes the variant unconstructible.
    type Stray: fmt::Debug + PartialEq + Eq;
    /// What a synthetic closing carries. Dirty stages use `()`; refined stages use
    /// `Infallible`, which makes the variant unconstructible.
    type Unclosed: fmt::Debug + PartialEq + Eq;
}

/// The stage `match_brackets` produces: its runs hold lexed tokens, and the tree can carry
/// both bracket errors.
#[derive(Debug, PartialEq, Eq)]
pub struct BracketsMatched;

impl TreeContents for BracketsMatched {
    type Inner = Vec<WithSpan<NonBracketTokenKind>>;
    type Stray = BracketKind;
    type Unclosed = ();
}

/// One isograph literal with its brackets matched. Spans live on the `WithSpan` wrapping
/// each item.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedBrackets<TContents: TreeContents>(pub Vec<WithSpan<BracketItem<TContents>>>);

#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem<TContents: TreeContents> {
    /// A maximal run containing no brackets. Its span runs from its first token's start to
    /// its last token's end, whitespace between them included.
    Inner(TContents::Inner),
    Bracketed(Bracketed<TContents>),
    /// A close bracket no open of its kind was waiting for; it is an invalid section one
    /// token wide.
    StrayClose(TContents::Stray),
}

/// An open bracket, everything up to its close, and the close, which is always present, so
/// every pass after this one works with guaranteed matching brackets. The wrapping
/// `WithSpan`'s span runs from the start of the opening to the end of a real closing, or to
/// the end of the last child when the closing is synthetic.
#[derive(Debug, PartialEq, Eq)]
pub struct Bracketed<TContents: TreeContents> {
    pub opening: WithSpan<BracketKind>,
    pub closing: Closing<TContents>,
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Closing<TContents: TreeContents> {
    /// The close bracket the author typed.
    Real(Span),
    /// The group never got its close and was forced to end: at the close bracket an
    /// enclosing group owns, or at the end of the tokens. Where it ended is the end of the
    /// wrapping `WithSpan`'s span; the missing close has no span of its own. A group closed
    /// this way is an invalid section.
    Synthetic(TContents::Unclosed),
}

#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<BracketKind>),
    /// A group whose close was synthesized.
    Unclosed(WithSpan<UnclosedGroup>),
}

/// The wrapping `WithSpan`'s span is the whole group; its end is where the close should have
/// been.
#[derive(Debug, PartialEq, Eq)]
pub struct UnclosedGroup {
    pub opening: WithSpan<BracketKind>,
}

impl<TContents> MatchedBrackets<TContents>
where
    TContents: TreeContents<Stray = BracketKind, Unclosed = ()>,
{
    /// Every error the pass produced, in source order of the position each error starts at.
    /// The list is empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(&self.0, &mut errors);
        errors
    }
}

fn collect_errors<TContents>(
    items: &[WithSpan<BracketItem<TContents>>],
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<Stray = BracketKind, Unclosed = ()>,
{
    for item in items {
        match &item.item {
            BracketItem::Inner(_) => {}
            BracketItem::StrayClose(kind) => {
                errors.push(BracketError::UnexpectedClose(WithSpan::new(*kind, item.span)));
            }
            BracketItem::Bracketed(bracketed) => {
                if matches!(bracketed.closing, Closing::Synthetic(())) {
                    errors.push(BracketError::Unclosed(WithSpan::new(
                        UnclosedGroup {
                            opening: bracketed.opening,
                        },
                        item.span,
                    )));
                }
                collect_errors(&bracketed.children, errors);
            }
        }
    }
}

type TokenStream = Peekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> MatchedBrackets<BracketsMatched> {
    let mut tokens = tokens.into_iter().peekable();
    let mut enclosing = Vec::new();
    let items = parse_items(&mut tokens, &mut enclosing);
    MatchedBrackets(items)
}

/// Parse items until a close bracket some enclosing group owns, or the end of the tokens.
///
/// `enclosing` is the kind of every group this level sits inside, innermost last, the group
/// being parsed included; it is how a close bracket with no open of its kind anywhere is
/// recognized as stray rather than left to end this level.
fn parse_items(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
) -> Vec<WithSpan<BracketItem<BracketsMatched>>> {
    let mut items = Vec::new();
    let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
    while let Some(&token) = tokens.peek() {
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                tokens.next();
                run.push(WithSpan::new(kind, token.span));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(parse_bracketed(
                    tokens,
                    enclosing,
                    WithSpan::new(kind, token.span),
                ));
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Leaving it unconsumed is what
                    // synthetically closes every group between here and its owner.
                    break;
                }
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(WithSpan::new(BracketItem::StrayClose(kind), token.span));
            }
        }
    }
    flush_run(&mut items, &mut run);
    items
}

/// End the run in progress, if any, into one `Inner` item spanning its first token's start
/// to its last token's end.
fn flush_run(
    items: &mut Vec<WithSpan<BracketItem<BracketsMatched>>>,
    run: &mut Vec<WithSpan<NonBracketTokenKind>>,
) {
    let span = match (run.first(), run.last()) {
        (Some(first), Some(last)) => Span::join(first.span, last.span),
        _ => return,
    };
    items.push(WithSpan::new(BracketItem::Inner(std::mem::take(run)), span));
}

/// One group, whose opening the caller already consumed. This parses the children, then
/// looks at the one token that stopped them: the group's own close, which it consumes into
/// `Closing::Real`, or a close an enclosing group owns, which it leaves alone while closing
/// this group synthetically where it stands.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
    opening: WithSpan<BracketKind>,
) -> WithSpan<BracketItem<BracketsMatched>> {
    enclosing.push(opening.item);
    let children = parse_items(tokens, enclosing);
    enclosing.pop();

    let (closing, end) = match tokens.peek() {
        Some(&token)
            if SplitToken::from(token.item)
                == SplitToken::Bracket(BracketToken::Close(opening.item)) =>
        {
            tokens.next();
            (Closing::Real(token.span), token.span.end)
        }
        // The group was forced to end: at a close an enclosing group owns, or at the end of
        // the tokens. It ends where its last child does.
        _ => (
            Closing::Synthetic(()),
            children.last().map_or(opening.span.end, |last| last.span.end),
        ),
    };

    let span = Span::new(opening.span.start, end);
    WithSpan::new(
        BracketItem::Bracketed(Bracketed {
            opening,
            closing,
            children,
        }),
        span,
    )
}

impl<TFrom: TreeContents> MatchedBrackets<TFrom> {
    /// Cross the tree to another stage, mapping every slot fallibly. The result is either
    /// the whole crossed tree or every refusal, in source order.
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
