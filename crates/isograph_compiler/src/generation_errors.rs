use common_lang_types::{Diagnostic, RelativePathToSourceFile};
use isograph_parser::IsoLiteralItem;
use pico_macros::memo;
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::{
    ClientId, CompilationProfile, EntrypointQueryError, FieldSelectionMapError, IntegerOutOfRange,
    IsographDb, SelectionMapErrorKind, TopLevelError, field_selection_map,
    iso_literal_host_errors_in_file, iso_literal_parse_errors_in_file,
    iso_literal_validation_errors_in_file, parsed_iso_literals_in_file, query_for_entrypoint,
};

/// A query generation error of a declaration in a source file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QueryGenerationError {
    #[error("{0}")]
    SelectionMap(SelectionMapErrorKind),
    /// The lookup of the declared field failed after validation accepted it.
    #[error("{}", .0.0.message)]
    FieldLookup(Diagnostic),
    #[error("{0}")]
    TopLevel(TopLevelError),
    #[error("{0}")]
    IntegerOutOfRange(IntegerOutOfRange),
}

/// Query generation errors of the declarations in `path` as `client` reads it, as file-absolute
/// spans. Empty when the file has a parse, host, or validation error, which generation would
/// otherwise report a second time.
#[memo]
pub fn iso_literal_generation_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<QueryGenerationError>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    if !iso_literal_parse_errors_in_file(db, client, path)
        .as_ref()?
        .is_empty()
        || !iso_literal_host_errors_in_file(db, client, path)
            .as_ref()?
            .is_empty()
        || !iso_literal_validation_errors_in_file(db, client, path)
            .as_ref()?
            .is_empty()
    {
        return Vec::new().wrap_some();
    }
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            let item = parsed.item.as_ref()?.item.item.as_ref()?;
            declaration_generation_error(db, client, path, extraction.span(), item.item.reference())
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

/// The generation error of `item`, the item of the literal at `literal` in `path`, located in
/// that literal.
fn declaration_generation_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    literal: Span,
    item: &IsoLiteralItem,
) -> Option<WithSpan<QueryGenerationError>> {
    match item {
        IsoLiteralItem::Entity(_) => None,
        IsoLiteralItem::Selectable(declaration) => {
            let name = declaration.name.location.with_offset(literal.start);
            match field_selection_map(
                db,
                client,
                declaration.parent_type.item.0,
                declaration.name.item.0,
            ) {
                Ok(_) | Err(FieldSelectionMapError::NotASourceField(_)) => None,
                Err(FieldSelectionMapError::FieldLookup(diagnostic)) => {
                    QueryGenerationError::FieldLookup(diagnostic.clone())
                        .with_span(name)
                        .wrap_some()
                }
                Err(FieldSelectionMapError::Located(error)) => (error.location.path == path
                    && literal.contains(error.location.span))
                .then(|| {
                    QueryGenerationError::SelectionMap(error.kind.clone())
                        .with_span(error.location.span)
                }),
            }
        }
        IsoLiteralItem::Entrypoint(declaration) => {
            let name = declaration.name.location.with_offset(literal.start);
            match query_for_entrypoint(
                db,
                client,
                declaration.parent_type.item.0,
                declaration.name.item.0,
            ) {
                Ok(_)
                | Err(
                    EntrypointQueryError::Lookup(_)
                    | EntrypointQueryError::NotFetchable(_)
                    | EntrypointQueryError::SelectionMap(_),
                ) => None,
                Err(EntrypointQueryError::TopLevel(error)) => {
                    QueryGenerationError::TopLevel(error.clone())
                        .with_span(name)
                        .wrap_some()
                }
                Err(EntrypointQueryError::IntegerOutOfRange(error)) => {
                    QueryGenerationError::IntegerOutOfRange(error.clone())
                        .with_span(name)
                        .wrap_some()
                }
            }
        }
    }
}
