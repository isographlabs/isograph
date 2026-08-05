use std::path::Path;

use isograph_parser::{
    match_brackets, tokenize, BracketsMatched, MatchedBrackets, ResolvedBracketNode,
};
use resolve_position::ResolvePosition;
use span::Span;

/// The span of `pattern`, which must occur exactly once in `text`. The preferred way for a
/// test to point at a position: it needs nothing but the original string, editing the fixture
/// cannot silently shift what the test asserts about, and a pattern that stops being unique
/// fails loudly instead.
pub fn span_of(text: &str, pattern: &str) -> Span {
    let mut occurrences = text.match_indices(pattern);
    let (offset, _) = occurrences
        .next()
        .expect("the pattern the test anchors on occurs in the fixture");
    assert!(
        occurrences.next().is_none(),
        "the pattern the test anchors on occurs exactly once in the fixture"
    );
    Span::from_usize(offset, offset + pattern.len())
}

pub struct Fixture {
    pub text: String,
    pub tree: MatchedBrackets<BracketsMatched>,
}

impl Fixture {
    /// Load `file_name` (extension included) from `crates/tests/fixtures`.
    pub fn load(file_name: &str) -> Fixture {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(file_name);
        let text = std::fs::read_to_string(&path)
            .expect("the fixture named by the test exists under crates/tests/fixtures");
        let tree = match_brackets(tokenize(&text));
        Fixture { text, tree }
    }

    /// The path to the node containing `span`.
    pub fn resolve(&self, span: Span) -> ResolvedBracketNode<'_, BracketsMatched> {
        self.tree.resolve((), span)
    }

    /// `resolve` at the unique occurrence of `pattern` in the fixture's text.
    pub fn on(&self, pattern: &str) -> ResolvedBracketNode<'_, BracketsMatched> {
        self.resolve(span_of(&self.text, pattern))
    }

    /// The node at a 0-indexed line and character; the character indexes bytes in the
    /// line. For positions no distinctive text names.
    pub fn at(&self, line: u32, character: u32) -> ResolvedBracketNode<'_, BracketsMatched> {
        let offset = self.offset(line, character);
        self.resolve(Span::new(offset, offset + 1))
    }

    fn offset(&self, line: u32, character: u32) -> u32 {
        let line_start: usize = self
            .text
            .split_inclusive('\n')
            .take(line as usize)
            .map(str::len)
            .sum();
        line_start as u32 + character
    }
}
