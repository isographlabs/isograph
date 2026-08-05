use std::convert::Infallible;

use isograph_parser::{
    BracketError, BracketItemParent, BracketKind, Bracketed, BracketsMatched, Closing,
    MatchedBrackets, NonBracketTokenKind, ResolvedBracketNode, TreeContents,
};
use span::{Span, WithSpan};
use tests::{span_of, Fixture};

use BracketKind::{Brace, Bracket, Paren};
use PathStep::{Balanced, Inner, StrayClose, Unbalanced};

/// One node on the path from a resolved position up to the root, leaf first, root omitted.
#[derive(Debug, PartialEq, Eq)]
enum PathStep {
    Inner,
    StrayClose(BracketKind),
    Balanced(BracketKind),
    Unbalanced(BracketKind),
}

/// The path from the node a position resolves to up to the root, as comparable steps.
fn path_of(node: ResolvedBracketNode<'_, BracketsMatched>) -> Vec<PathStep> {
    let mut steps = Vec::new();
    let mut parent = match node {
        ResolvedBracketNode::MatchedBrackets(_) => return steps,
        ResolvedBracketNode::Inner(path) => {
            steps.push(Inner);
            path.parent
        }
        ResolvedBracketNode::StrayClose(path) => {
            steps.push(StrayClose(*path.inner));
            path.parent
        }
        ResolvedBracketNode::Bracketed(path) => {
            steps.push(step_of(path.inner));
            path.parent
        }
    };
    loop {
        parent = match parent {
            BracketItemParent::MatchedBrackets(_) => return steps,
            BracketItemParent::Bracketed(path) => {
                steps.push(step_of(path.inner));
                path.parent
            }
        };
    }
}

fn step_of(group: &Bracketed<BracketsMatched>) -> PathStep {
    match group.closing {
        Closing::Real(_) => Balanced(group.opening.item),
        Closing::Synthetic(()) => Unbalanced(group.opening.item),
    }
}

#[test]
fn the_unclosed_paren_is_an_unbalanced_group_inside_the_balanced_brace() {
    let fixture = Fixture::load("unclosed_paren.iso");
    // The `(` that the `}` refuses to close; it is the fixture's only paren.
    assert_eq!(
        path_of(fixture.on("(")),
        vec![Unbalanced(Paren), Balanced(Brace)]
    );
}

#[test]
fn the_enclosing_brace_groups_stay_balanced() {
    let fixture = Fixture::load("unclosed_paren.iso");
    // The run inside `first { ... }`, before the unbalanced paren section begins.
    assert_eq!(path_of(fixture.on("broken")), vec![Inner, Balanced(Brace)]);
    // Inside `second { fine }`, after the broken section.
    assert_eq!(path_of(fixture.on("fine")), vec![Inner, Balanced(Brace)]);
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
    assert_eq!(path_of(fixture.on("Query")), vec![Inner]);
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn balanced_input_nests_as_typed() {
    let fixture = Fixture::load("balanced.iso");
    assert_eq!(
        path_of(fixture.on("1")),
        vec![Inner, Balanced(Bracket), Balanced(Paren), Balanced(Brace)]
    );
    assert_eq!(
        path_of(fixture.on("id")),
        vec![Inner, Balanced(Brace), Balanced(Brace)]
    );
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn a_wrong_kind_close_leaves_only_the_paren_unbalanced() {
    let fixture = Fixture::load("wrong_kind_close.iso");
    assert_eq!(
        path_of(fixture.on("(")),
        vec![Unbalanced(Paren), Balanced(Brace)]
    );
    assert_eq!(path_of(fixture.on("bar")), vec![Inner, Balanced(Brace)]);
    assert_eq!(path_of(fixture.on("Query")), vec![Inner]);
    // Whitespace after the `(` sits outside the childless paren group, so it resolves to
    // the enclosing brace group.
    assert_eq!(path_of(fixture.at(0, 22)), vec![Balanced(Brace)]);
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
    assert_eq!(path_of(fixture.on("{")), vec![Balanced(Brace)]);
    assert_eq!(
        path_of(fixture.on("(")),
        vec![Unbalanced(Paren), Balanced(Brace)]
    );
    assert_eq!(
        path_of(fixture.on("[")),
        vec![Unbalanced(Bracket), Unbalanced(Paren), Balanced(Brace)]
    );
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
    assert_eq!(path_of(fixture.on("foo")), vec![Inner, Balanced(Brace)]);
    assert_eq!(
        path_of(fixture.on(")")),
        vec![StrayClose(Paren), Balanced(Brace)]
    );
    assert_eq!(path_of(fixture.on("bar")), vec![Inner, Balanced(Brace)]);
    match fixture.tree.errors().as_slice() {
        [BracketError::UnexpectedClose(stray)] => {
            assert_eq!(stray.span, span_of(&fixture.text, ")"));
            assert_eq!(stray.item, BracketKind::Paren);
        }
        errors => panic!("expected exactly the stray close, got {errors:?}"),
    }
}

#[test]
fn a_stray_close_does_not_end_a_different_kind() {
    let fixture = Fixture::load("stray_close_inside_paren.iso");
    // The paren pair still matches around the stray `}`.
    assert_eq!(path_of(fixture.on("(")), vec![Balanced(Paren)]);
    assert_eq!(
        path_of(fixture.on("}")),
        vec![StrayClose(Brace), Balanced(Paren)]
    );
    match fixture.tree.errors().as_slice() {
        [BracketError::UnexpectedClose(stray)] => {
            assert_eq!(stray.span, span_of(&fixture.text, "}"));
            assert_eq!(stray.item, BracketKind::Brace);
        }
        errors => panic!("expected exactly the stray close, got {errors:?}"),
    }
}

#[test]
fn crossing_pairs_produce_two_errors_in_source_order() {
    let fixture = Fixture::load("crossing_pairs.iso");
    assert_eq!(path_of(fixture.on("(")), vec![Balanced(Paren)]);
    assert_eq!(
        path_of(fixture.on("{")),
        vec![Unbalanced(Brace), Balanced(Paren)]
    );
    // The trailing `}` is a stray brace close at the top level: its `{` was consumed inside
    // the paren group.
    assert_eq!(path_of(fixture.on("}")), vec![StrayClose(Brace)]);
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
    assert_eq!(path_of(fixture.on("a")), vec![Inner]);
    // `c` sits in a run inside b's balanced brace group, which sits inside a's unbalanced
    // brace group.
    assert_eq!(
        path_of(fixture.on("c")),
        vec![Inner, Balanced(Brace), Unbalanced(Brace)]
    );
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
    assert_eq!(
        path_of(fixture.on("\"a}\"")),
        vec![Inner, Balanced(Brace)]
    );
    assert_eq!(fixture.tree.errors(), vec![]);
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

#[test]
fn content_after_an_unclosed_open_sits_inside_the_unbalanced_group() {
    let fixture = Fixture::load("unclosed_at_end.iso");
    assert_eq!(path_of(fixture.on("b")), vec![Inner, Unbalanced(Brace)]);
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
    assert_eq!(path_of(fixture.on("a")), vec![Inner]);
    assert_eq!(path_of(fixture.on("}")), vec![StrayClose(Brace)]);
    match fixture.tree.errors().as_slice() {
        [BracketError::UnexpectedClose(stray)] => {
            assert_eq!(stray.span, span_of(&fixture.text, "}"));
            assert_eq!(stray.item, BracketKind::Brace);
        }
        errors => panic!("expected exactly the stray close, got {errors:?}"),
    }
}
