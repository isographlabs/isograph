use common_lang_types::RelativePathToSourceFile;
use isograph_parser::ParseError;
use pico_macros::memo;
use prelude::Postfix;
use span::WithSpan;

use crate::{
    ClientId, CompilationProfile, HostLanguage, IsographDb, NetworkProtocol, QueryGenerationError,
    ValidationError, iso_literal_generation_errors_in_file, iso_literal_host_errors_in_file,
    iso_literal_parse_errors_in_file, iso_literal_validation_errors_in_file,
};

/// An error the compiler reports for a source file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompilerError<THostError: std::fmt::Display, TNotFetchable: std::fmt::Display> {
    #[error("{0}")]
    Parse(ParseError),
    #[error("{0}")]
    Host(THostError),
    #[error("{0}")]
    Validation(ValidationError<TNotFetchable>),
    #[error("{0}")]
    Generation(QueryGenerationError),
}

pub type ProfileCompilerError<TProfile> = CompilerError<
    <<TProfile as CompilationProfile>::HostLanguage as HostLanguage>::Error,
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

pub type ProfileSchemaFileError<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SchemaFileError;

/// Every error the protocol reports for the schema file `path` as `client` reads it, as
/// file-absolute spans; `None` when the file has no text.
#[memo]
pub fn schema_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ProfileSchemaFileError<TProfile>>>> {
    TProfile::NetworkProtocol::schema_file_errors(db, client, path)
}

/// Every compiler error of `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn compiler_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ProfileCompilerError<TProfile>>>> {
    let parse_errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
    let host_errors = iso_literal_host_errors_in_file(db, client, path).as_ref()?;
    let validation_errors = iso_literal_validation_errors_in_file(db, client, path).as_ref()?;
    let generation_errors = iso_literal_generation_errors_in_file(db, client, path).as_ref()?;
    parse_errors
        .iter()
        .map(|error| error.clone().map(CompilerError::Parse))
        .chain(
            host_errors
                .iter()
                .map(|error| error.clone().map(CompilerError::Host)),
        )
        .chain(
            validation_errors
                .iter()
                .map(|error| error.clone().map(CompilerError::Validation)),
        )
        .chain(
            generation_errors
                .iter()
                .map(|error| error.clone().map(CompilerError::Generation)),
        )
        .collect::<Vec<_>>()
        .wrap_some()
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_parser::{AstError, ParseError};
    use prelude::Postfix;
    use span::WithSpan;

    use super::{CompilerError, ProfileCompilerError, compiler_errors_in_file};
    use crate::test_profile::TestProfile;
    use crate::{
        ClientId, FileMembership, IncludeFile, IsographDb, iso_literal_parse_errors_in_file,
        iso_literal_validation_errors_in_file,
    };

    const CLIENT: ClientId = ClientId(1);

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };

    fn path() -> RelativePathToSourceFile {
        "src/a.ts".intern().to()
    }

    fn errors(
        db: &IsographDb<TestProfile>,
    ) -> Option<Vec<WithSpan<ProfileCompilerError<TestProfile>>>> {
        compiler_errors_in_file(db, CLIENT, path()).clone()
    }

    #[test]
    fn missing_file_has_no_errors() {
        let db = IsographDb::<TestProfile>::default();
        assert_eq!(errors(db.reference()), None);
    }

    #[test]
    fn a_file_with_no_literals_has_no_errors() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(path(), SOURCE_FILE, "export const a = 1;\n".to_owned());
        assert_eq!(errors(db.reference()), Vec::new().wrap_some());
    }

    #[test]
    fn parse_errors_are_compiler_errors_with_the_same_spans() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(path(), SOURCE_FILE, "iso(`entrypoint`)".to_owned());
        let parse_errors = iso_literal_parse_errors_in_file(db.reference(), CLIENT, path())
            .clone()
            .expect("the file has text");
        assert!(!parse_errors.is_empty());
        assert_eq!(
            errors(db.reference()),
            parse_errors
                .into_iter()
                .map(|error| error.map(CompilerError::Parse))
                .collect::<Vec<_>>()
                .wrap_some()
        );
    }

    #[test]
    fn a_whitespace_only_literal_is_the_empty_literal_error() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(path(), SOURCE_FILE, "iso(` `)".to_owned());
        let errors = errors(db.reference()).expect("the file has text");
        assert_eq!(
            errors
                .into_iter()
                .map(|error| error.item)
                .collect::<Vec<_>>(),
            CompilerError::Parse(ParseError::Ast(AstError::EmptyLiteral)).wrap_vec()
        );
    }

    #[test]
    fn validation_errors_are_compiler_errors_with_the_same_spans() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path(),
            SOURCE_FILE,
            "iso(`entity Query`) iso(`field Query.x { nope }`)".to_owned(),
        );
        let validation_errors =
            iso_literal_validation_errors_in_file(db.reference(), CLIENT, path())
                .clone()
                .expect("the file has text");
        assert!(!validation_errors.is_empty());
        assert_eq!(
            errors(db.reference()),
            validation_errors
                .into_iter()
                .map(|error| error.map(CompilerError::Validation))
                .collect::<Vec<_>>()
                .wrap_some()
        );
    }

    #[test]
    fn parse_errors_precede_validation_errors() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path(),
            SOURCE_FILE,
            "iso(`entrypoint Query.Missing`) iso(` `)".to_owned(),
        );
        let errors = errors(db.reference()).expect("the file has text");
        assert!(matches!(
            errors
                .iter()
                .map(|error| error.item.reference())
                .collect::<Vec<_>>()
                .as_slice(),
            [CompilerError::Parse(_), CompilerError::Validation(_)]
        ));
    }

    #[test]
    fn a_file_whose_declarations_resolve_has_no_errors() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path(),
            SOURCE_FILE,
            "iso(`entity Query`) iso(`field Query.x { y }`) iso(`field Query.y`)".to_owned(),
        );
        assert_eq!(errors(db.reference()), Vec::new().wrap_some());
    }
}
