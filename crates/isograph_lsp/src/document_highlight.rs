use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{
    ClientId, HostLanguage, IsographDb, LiteralId, iso_literal_extraction, parsed_iso_literal_by_id,
};
use isograph_parser::IsographSemanticToken;
use prelude::Postfix;
use span::{Span, WithOptionalSpan};

use crate::hover::iso_literal_at_lsp_position;
use crate::line_index::line_index;

/// The file range of the semantic token under `position` in `path` as `client` reads it.
pub fn token_range_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Range> {
    let span = token_span_at_lsp_position(db, client, path, position)?;
    line_index(db, client, path)
        .as_ref()?
        .range(span)
        .wrap_some()
}

/// The file span of the semantic token under `position`.
fn token_span_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<Span> {
    let (literal_id, token) = literal_token_at_lsp_position(db, client, path, position)?;
    let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
    token
        .with_offset(extraction.iso_literal_start_index.0 as u32)
        .wrap_some()
}

/// The literal under `position`, and the span within it of the semantic token there.
fn literal_token_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Span)> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    let parsed = parsed_iso_literal_by_id(db, literal_id)?;
    (literal_id, semantic_token_span_at(&parsed.tokens, cursor)?).wrap_some()
}

/// The span of the token that contains `cursor`. Tokens without a span and commas are
/// not candidates; semantic tokens do not emit them.
fn semantic_token_span_at(
    tokens: &[WithOptionalSpan<IsographSemanticToken>],
    cursor: Span,
) -> Option<Span> {
    tokens
        .iter()
        .filter(|token| token.item != IsographSemanticToken::Comma)
        .filter_map(|token| token.location)
        .find(|span| span.contains(cursor))
}

#[cfg(test)]
mod tests {
    use isograph_parser::parse_iso_literal;
    use prelude::Postfix;
    use span::Span;

    use super::semantic_token_span_at;

    fn token_at(text: &str, offset: usize) -> Option<Span> {
        let character = text[offset..]
            .chars()
            .next()
            .expect("the test passes an offset inside the text");
        let cursor = Span::from_usize(offset, offset + character.len_utf8());
        semantic_token_span_at(&parse_iso_literal(text).tokens, cursor)
    }

    fn span_of(text: &str, needle: &str) -> Span {
        let start = text.find(needle).expect("the fixture contains the needle");
        Span::from_usize(start, start + needle.len())
    }

    #[test]
    fn every_char_of_a_name_is_that_names_token() {
        let text = "entrypoint Query.HomeRoute";
        let name = span_of(text, "HomeRoute");
        (name.start..name.end).for_each(|offset| {
            assert_eq!(
                token_at(text, offset as usize),
                name.wrap_some(),
                "offset {offset}"
            );
        });
    }

    #[test]
    fn a_period_is_its_own_token() {
        let text = "entrypoint Query.HomeRoute";
        let period = span_of(text, ".");
        assert_eq!(token_at(text, period.start as usize), period.wrap_some());
    }

    #[test]
    fn whitespace_has_no_token() {
        let text = "entrypoint Query.HomeRoute";
        assert_eq!(token_at(text, span_of(text, " ").start as usize), None);
    }

    #[test]
    fn a_comma_has_no_token() {
        let text = "field Query.x { a, b }";
        assert_eq!(token_at(text, span_of(text, ",").start as usize), None);
    }

    #[test]
    fn a_cursor_past_the_last_token_has_no_token() {
        let text = "entrypoint Query.A  ";
        assert_eq!(token_at(text, text.len() - 1), None);
    }
}
