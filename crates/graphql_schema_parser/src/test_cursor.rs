use crate::token_cursor::TokenCursor;
use crate::tokenize::tokenize;

pub(crate) fn with_cursor<R>(text: &str, parse: impl FnOnce(&mut TokenCursor<'_>) -> R) -> R {
    let items = tokenize(text);
    parse(&mut TokenCursor::new(items.as_slice(), text))
}
