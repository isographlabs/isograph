use std::convert::Infallible;

use isograph_parser::{
    BracketError, BracketItemParent, BracketKind, BracketedPath, BracketsMatched, Closing,
    InnerPath, MatchedBrackets, MatchedClosePath, NonBracketTokenKind, OpenBracketPath,
    ResolvedBracketNode, TreeContents, UnmatchedClosePath,
};
use span::{Span, WithSpan};
use tests::{span_of, Fixture};

use BracketKind::{Brace, Bracket, Paren};

fn run(node: ResolvedBracketNode<'_, BracketsMatched>) -> InnerPath<'_, BracketsMatched> {
    match node {
        ResolvedBracketNode::Inner(run) => run,
        node => panic!("expected a run, got {node:?}"),
    }
}

fn open_bracket(
    node: ResolvedBracketNode<'_, BracketsMatched>,
) -> OpenBracketPath<'_, BracketsMatched> {
    match node {
        ResolvedBracketNode::OpenBracket(open) => open,
        node => panic!("expected an open bracket, got {node:?}"),
    }
}

fn matched_close(
    node: ResolvedBracketNode<'_, BracketsMatched>,
) -> MatchedClosePath<'_, BracketsMatched> {
    match node {
        ResolvedBracketNode::MatchedClose(close) => close,
        node => panic!("expected a matched close, got {node:?}"),
    }
}

fn unmatched_close(
    node: ResolvedBracketNode<'_, BracketsMatched>,
) -> UnmatchedClosePath<'_, BracketsMatched> {
    match node {
        ResolvedBracketNode::UnmatchedClose(close) => close,
        node => panic!("expected an unmatched close, got {node:?}"),
    }
}

fn whitespace_in_group(
    node: ResolvedBracketNode<'_, BracketsMatched>,
) -> BracketedPath<'_, BracketsMatched> {
    match node {
        ResolvedBracketNode::Bracketed(group) => group,
        node => panic!("expected a group, got {node:?}"),
    }
}

fn enclosing_group(
    parent: BracketItemParent<'_, BracketsMatched>,
) -> BracketedPath<'_, BracketsMatched> {
    match parent {
        BracketItemParent::Bracketed(group) => *group,
        parent => panic!("expected an enclosing group, got {parent:?}"),
    }
}

fn assert_root(parent: BracketItemParent<'_, BracketsMatched>) {
    assert!(matches!(parent, BracketItemParent::MatchedBrackets(_)));
}

fn assert_balanced(group: &BracketedPath<'_, BracketsMatched>, kind: BracketKind) {
    assert_eq!(group.inner.opening.item, kind);
    assert!(matches!(group.inner.closing, Closing::Real(_)));
}

fn assert_unbalanced(group: &BracketedPath<'_, BracketsMatched>, kind: BracketKind) {
    assert_eq!(group.inner.opening.item, kind);
    assert!(matches!(group.inner.closing, Closing::Synthetic(())));
}

#[test]
fn the_unclosed_paren_is_an_unbalanced_group_inside_the_balanced_brace() {
    let fixture = Fixture::load("unclosed_paren.iso");
    // The `(` that the `}` refuses to close; it is the fixture's only paren.
    let open = open_bracket(fixture.on("("));
    assert_eq!(open.inner.item, Paren);
    let paren_group = *open.parent;
    assert_unbalanced(&paren_group, Paren);
    let brace_group = enclosing_group(paren_group.parent);
    assert_balanced(&brace_group, Brace);
    assert_root(brace_group.parent);
}

#[test]
fn the_enclosing_brace_groups_stay_balanced() {
    let fixture = Fixture::load("unclosed_paren.iso");
    // The run inside `first { ... }`, before the unbalanced paren section begins.
    let first_group = enclosing_group(run(fixture.on("broken")).parent);
    assert_balanced(&first_group, Brace);
    assert_root(first_group.parent);
    // Inside `second { fine }`, after the broken section.
    let second_group = enclosing_group(run(fixture.on("fine")).parent);
    assert_balanced(&second_group, Brace);
    assert_root(second_group.parent);
}

#[test]
fn the_unclosed_paren_is_the_only_error() {
    let fixture = Fixture::load("unclosed_paren.iso");
    let errors = fixture.tree.errors();
    match errors.as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.0.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}

#[test]
fn an_unbracketed_run_sits_at_the_top_level() {
    let fixture = Fixture::load("text_outside.iso");
    assert_root(run(fixture.on("Query")).parent);
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn balanced_input_nests_as_typed() {
    let fixture = Fixture::load("balanced.iso");
    // `1` sits in the `[...]` inside the `(...)` inside the outer `{...}`.
    let square_group = enclosing_group(run(fixture.on("1")).parent);
    assert_balanced(&square_group, Bracket);
    let paren_group = enclosing_group(square_group.parent);
    assert_balanced(&paren_group, Paren);
    let brace_group = enclosing_group(paren_group.parent);
    assert_balanced(&brace_group, Brace);
    assert_root(brace_group.parent);
    // `id` sits in the inner `{...}` inside the outer `{...}`.
    let inner_brace_group = enclosing_group(run(fixture.on("id")).parent);
    assert_balanced(&inner_brace_group, Brace);
    let outer_brace_group = enclosing_group(inner_brace_group.parent);
    assert_balanced(&outer_brace_group, Brace);
    assert_root(outer_brace_group.parent);
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn a_wrong_kind_close_leaves_only_the_paren_unbalanced() {
    let fixture = Fixture::load("wrong_kind_close.iso");
    let paren_group = *open_bracket(fixture.on("(")).parent;
    assert_unbalanced(&paren_group, Paren);
    let brace_group = enclosing_group(paren_group.parent);
    assert_balanced(&brace_group, Brace);
    assert_root(brace_group.parent);
    // The `}` that refused to close the paren is the brace group's own close.
    let close = matched_close(fixture.on("}"));
    assert_eq!(*close.inner, span_of(&fixture.text, "}"));
    assert_balanced(&close.parent, Brace);
    assert_balanced(&enclosing_group(run(fixture.on("bar")).parent), Brace);
    assert_root(run(fixture.on("Query")).parent);
    // Whitespace after the `(` sits outside the childless paren group, so it resolves to
    // the enclosing brace group.
    assert_balanced(&whitespace_in_group(fixture.at(0, 22)), Brace);
    match fixture.tree.errors().as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.0.span, span_of(&fixture.text, "("));
            // The childless group's span is its opening alone.
            assert_eq!(unclosed.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}

#[test]
fn wrong_kind_opens_close_synthetically_and_nest() {
    let fixture = Fixture::load("several_wrong_kind_opens.iso");
    let brace_group = *open_bracket(fixture.on("{")).parent;
    assert_balanced(&brace_group, Brace);
    assert_root(brace_group.parent);
    let square_group = *open_bracket(fixture.on("[")).parent;
    assert_unbalanced(&square_group, Bracket);
    let paren_group = enclosing_group(square_group.parent);
    assert_unbalanced(&paren_group, Paren);
    let outer_group = enclosing_group(paren_group.parent);
    assert_balanced(&outer_group, Brace);
    assert_root(outer_group.parent);
    match fixture.tree.errors().as_slice() {
        [BracketError::Unclosed(paren), BracketError::Unclosed(square)] => {
            assert_eq!(paren.item.0.span, span_of(&fixture.text, "("));
            assert_eq!(square.item.0.span, span_of(&fixture.text, "["));
            // The paren group reaches its last child, the `[` group.
            assert_eq!(
                paren.span,
                Span::join(span_of(&fixture.text, "("), span_of(&fixture.text, "[")),
            );
        }
        errors => panic!("expected the paren then the bracket, got {errors:?}"),
    }
}

#[test]
fn a_stray_close_is_one_token_inside_the_balanced_brace() {
    let fixture = Fixture::load("stray_close.iso");
    assert_balanced(&enclosing_group(run(fixture.on("foo")).parent), Brace);
    let stray = unmatched_close(fixture.on(")"));
    assert_eq!(*stray.inner, Paren);
    let brace_group = enclosing_group(stray.parent);
    assert_balanced(&brace_group, Brace);
    assert_root(brace_group.parent);
    assert_balanced(&enclosing_group(run(fixture.on("bar")).parent), Brace);
    match fixture.tree.errors().as_slice() {
        [BracketError::UnexpectedClose(stray)] => {
            assert_eq!(stray.span, span_of(&fixture.text, ")"));
            assert_eq!(stray.item, Paren);
        }
        errors => panic!("expected exactly the stray close, got {errors:?}"),
    }
}

#[test]
fn a_stray_close_does_not_end_a_different_kind() {
    let fixture = Fixture::load("stray_close_inside_paren.iso");
    // The paren pair still matches around the stray `}`: its own `)` resolves as the
    // matched close of the balanced paren group.
    let close = matched_close(fixture.on(")"));
    assert_eq!(*close.inner, span_of(&fixture.text, ")"));
    assert_balanced(&close.parent, Paren);
    assert_root(close.parent.parent);
    let stray = unmatched_close(fixture.on("}"));
    assert_eq!(*stray.inner, Brace);
    assert_balanced(&enclosing_group(stray.parent), Paren);
    match fixture.tree.errors().as_slice() {
        [BracketError::UnexpectedClose(stray)] => {
            assert_eq!(stray.span, span_of(&fixture.text, "}"));
            assert_eq!(stray.item, Brace);
        }
        errors => panic!("expected exactly the stray close, got {errors:?}"),
    }
}

#[test]
fn crossing_pairs_produce_two_errors_in_source_order() {
    let fixture = Fixture::load("crossing_pairs.iso");
    let paren_group = *open_bracket(fixture.on("(")).parent;
    assert_balanced(&paren_group, Paren);
    assert_root(paren_group.parent);
    let brace_group = *open_bracket(fixture.on("{")).parent;
    assert_unbalanced(&brace_group, Brace);
    assert_balanced(&enclosing_group(brace_group.parent), Paren);
    // The trailing `}` is a stray brace close at the top level: its `{` was consumed inside
    // the paren group.
    let stray = unmatched_close(fixture.on("}"));
    assert_eq!(*stray.inner, Brace);
    assert_root(stray.parent);
    match fixture.tree.errors().as_slice() {
        [BracketError::Unclosed(brace), BracketError::UnexpectedClose(stray)] => {
            assert_eq!(brace.item.0.span, span_of(&fixture.text, "{"));
            assert_eq!(stray.span, span_of(&fixture.text, "}"));
        }
        errors => panic!("expected the unclosed brace then the stray close, got {errors:?}"),
    }
}

#[test]
fn the_close_pairs_with_the_nearest_open() {
    let fixture = Fixture::load("adjacent_same_kind.iso");
    assert_root(run(fixture.on("a")).parent);
    // `c` sits in a run inside b's balanced brace group, which sits inside a's unbalanced
    // brace group.
    let b_group = enclosing_group(run(fixture.on("c")).parent);
    assert_balanced(&b_group, Brace);
    let a_group = enclosing_group(b_group.parent);
    assert_unbalanced(&a_group, Brace);
    assert_root(a_group.parent);
    // The one `}` is b's close, not a's.
    let close = matched_close(fixture.on("}"));
    assert_balanced(&close.parent, Brace);
    assert_unbalanced(&enclosing_group(close.parent.parent), Brace);
    match fixture.tree.errors().as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            // The unclosed group is the outer one: it contains `b`, which b's own group
            // does not.
            assert!(unclosed.span.contains(span_of(&fixture.text, "b")));
        }
        errors => panic!("expected exactly the outer unclosed brace, got {errors:?}"),
    }
}

#[test]
fn brackets_inside_strings_are_not_structural() {
    let fixture = Fixture::load("string_brackets.iso");
    let brace_group = enclosing_group(run(fixture.on("\"a}\"")).parent);
    assert_balanced(&brace_group, Brace);
    assert_root(brace_group.parent);
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn content_after_an_unclosed_open_sits_inside_the_unbalanced_group() {
    let fixture = Fixture::load("unclosed_at_end.iso");
    let brace_group = enclosing_group(run(fixture.on("b")).parent);
    assert_unbalanced(&brace_group, Brace);
    assert_root(brace_group.parent);
    match fixture.tree.errors().as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.0.span, span_of(&fixture.text, "{"));
        }
        errors => panic!("expected exactly the unclosed brace, got {errors:?}"),
    }
}

#[test]
fn a_stray_close_at_the_top_level_is_a_leaf_of_the_root() {
    let fixture = Fixture::load("stray_close_alone.iso");
    assert_root(run(fixture.on("a")).parent);
    let stray = unmatched_close(fixture.on("}"));
    assert_eq!(*stray.inner, Brace);
    assert_root(stray.parent);
    match fixture.tree.errors().as_slice() {
        [BracketError::UnexpectedClose(stray)] => {
            assert_eq!(stray.span, span_of(&fixture.text, "}"));
            assert_eq!(stray.item, Brace);
        }
        errors => panic!("expected exactly the stray close, got {errors:?}"),
    }
}

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
    let fixture = Fixture::load("balanced.iso");
    assert!(refine(fixture.tree).is_ok());
}

#[test]
fn refining_reports_the_unclosed_paren() {
    let fixture = Fixture::load("unclosed_paren.iso");
    let errors = refine(fixture.tree).expect_err("the fixture's paren never closes");
    match errors.as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.0.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}
