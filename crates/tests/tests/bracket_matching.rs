use std::convert::Infallible;

use isograph_parser::{
    BracketError, BracketItemParent, BracketKind, BracketsMatched, Closing, MatchedBrackets,
    NonBracketTokenKind, ResolvedBracketNode, SectionValidity, TreeContents,
};
use span::{Span, WithSpan};
use tests::{span_of, Fixture};

#[test]
fn unclosed_paren_is_an_invalid_section() {
    let fixture = Fixture::load("unclosed_paren");
    // The `(` that the `}` refuses to close; it is the fixture's only paren.
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Invalid));
}

#[test]
fn the_enclosing_brace_group_stays_valid() {
    let fixture = Fixture::load("unclosed_paren");
    // The run inside `first { ... }`, before the invalid paren section begins.
    assert!(matches!(fixture.on("broken").validity(), SectionValidity::Valid));
    // Inside `second { fine }`, after the broken section.
    assert!(matches!(fixture.on("fine").validity(), SectionValidity::Valid));
}

#[test]
fn the_unclosed_group_is_the_leaf_it_resolves_to() {
    let fixture = Fixture::load("unclosed_paren");
    match fixture.on("(") {
        ResolvedBracketNode::Bracketed(path) => {
            assert!(matches!(path.inner.closing, Closing::Synthetic(())));
        }
        node => panic!("expected the unclosed paren group, got {node:?}"),
    }
}

#[test]
fn the_unclosed_paren_is_the_only_error() {
    let fixture = Fixture::load("unclosed_paren");
    let errors = fixture.tree.errors();
    match errors.as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.0.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}

#[test]
fn an_unbracketed_run_is_valid_and_error_free() {
    let fixture = Fixture::load("text_outside");
    assert!(matches!(fixture.on("Query"), ResolvedBracketNode::Inner(_)));
    assert!(matches!(fixture.on("Query").validity(), SectionValidity::Valid));
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn balanced_input_is_valid_everywhere_and_error_free() {
    let fixture = Fixture::load("balanced");
    assert!(matches!(fixture.on("1").validity(), SectionValidity::Valid));
    assert!(matches!(fixture.on("id").validity(), SectionValidity::Valid));
    assert_eq!(fixture.tree.errors(), vec![]);
}

#[test]
fn a_wrong_kind_close_leaves_only_the_paren_invalid() {
    let fixture = Fixture::load("wrong_kind_close");
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Invalid));
    assert!(matches!(fixture.on("bar").validity(), SectionValidity::Valid));
    assert!(matches!(fixture.on("Query").validity(), SectionValidity::Valid));
    // Whitespace after the `(` sits outside the childless paren group, so it resolves to
    // the enclosing brace group.
    assert!(matches!(fixture.at(0, 22).validity(), SectionValidity::Valid));
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
    let fixture = Fixture::load("several_wrong_kind_opens");
    assert!(matches!(fixture.on("{").validity(), SectionValidity::Valid));
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Invalid));
    assert!(matches!(fixture.on("[").validity(), SectionValidity::Invalid));
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
fn a_stray_close_is_one_token_of_invalid() {
    let fixture = Fixture::load("stray_close");
    assert!(matches!(fixture.on("foo").validity(), SectionValidity::Valid));
    assert!(matches!(fixture.on(")").validity(), SectionValidity::Invalid));
    assert!(matches!(fixture.on("bar").validity(), SectionValidity::Valid));
    // The exact path at the `)`: a stray paren close, sitting inside the matched brace
    // group, at the top level.
    match fixture.on(")") {
        ResolvedBracketNode::StrayClose(stray) => {
            assert_eq!(*stray.inner, BracketKind::Paren);
            match stray.parent {
                BracketItemParent::Bracketed(brace_group) => {
                    assert_eq!(brace_group.inner.opening.item, BracketKind::Brace);
                    assert!(matches!(brace_group.inner.closing, Closing::Real(_)));
                    assert!(matches!(
                        brace_group.parent,
                        BracketItemParent::MatchedBrackets(_)
                    ));
                }
                parent => panic!("expected the brace group above the stray, got {parent:?}"),
            }
        }
        node => panic!("expected the stray close, got {node:?}"),
    }
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
    let fixture = Fixture::load("stray_close_inside_paren");
    // The paren pair still matches around the stray `}`.
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Valid));
    assert!(matches!(fixture.on("}").validity(), SectionValidity::Invalid));
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
    let fixture = Fixture::load("crossing_pairs");
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Valid));
    assert!(matches!(fixture.on("{").validity(), SectionValidity::Invalid));
    // The trailing `}` is a stray brace close at the top level: its `{` was consumed inside
    // the paren group.
    match fixture.on("}") {
        ResolvedBracketNode::StrayClose(stray) => {
            assert_eq!(*stray.inner, BracketKind::Brace);
            assert!(matches!(
                stray.parent,
                BracketItemParent::MatchedBrackets(_)
            ));
        }
        node => panic!("expected the stray close, got {node:?}"),
    }
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
    let fixture = Fixture::load("adjacent_same_kind");
    assert!(matches!(fixture.on("a").validity(), SectionValidity::Valid));
    // The exact path at `c`: its run sits inside b's balanced group, and b's group sits
    // inside a's non-balanced group.
    match fixture.on("c") {
        ResolvedBracketNode::Inner(run) => match run.parent {
            BracketItemParent::Bracketed(b_group) => {
                assert_eq!(b_group.inner.opening.item, BracketKind::Brace);
                assert!(matches!(b_group.inner.closing, Closing::Real(_)));
                match b_group.parent {
                    BracketItemParent::Bracketed(a_group) => {
                        assert_eq!(a_group.inner.opening.item, BracketKind::Brace);
                        assert!(matches!(a_group.inner.closing, Closing::Synthetic(())));
                        assert!(matches!(
                            a_group.parent,
                            BracketItemParent::MatchedBrackets(_)
                        ));
                    }
                    parent => panic!("expected a's group above b's, got {parent:?}"),
                }
            }
            parent => panic!("expected b's group above the run, got {parent:?}"),
        },
        node => panic!("expected the run holding c, got {node:?}"),
    }
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
    let fixture = Fixture::load("string_brackets");
    assert!(matches!(
        fixture.on("\"a}\"").validity(),
        SectionValidity::Valid
    ));
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
    let fixture = Fixture::load("balanced");
    assert!(refine(fixture.tree).is_ok());
}

#[test]
fn refining_reports_the_unclosed_paren() {
    let fixture = Fixture::load("unclosed_paren");
    let errors = refine(fixture.tree).expect_err("the fixture's paren never closes");
    match errors.as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.0.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}
