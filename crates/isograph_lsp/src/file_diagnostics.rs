use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{ClientId, HostLanguage, IsographDb, iso_literal_parse_errors_in_file};
use pico_macros::memo;
use prelude::Postfix;

use crate::line_index;

const DIAGNOSTIC_SOURCE: &str = "isograph";

/// `lsp_types::Diagnostic`s of `path` as `client` reads it.
#[memo]
pub fn lsp_diagnostics_for_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::Diagnostic>> {
    let errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
    if errors.is_empty() {
        return Vec::new().wrap_some();
    }
    let line_index = line_index(db, client, path).as_ref()?;
    errors
        .iter()
        .map(|error| lsp_types::Diagnostic {
            range: line_index.range(error.location),
            severity: lsp_types::DiagnosticSeverity::ERROR.wrap_some(),
            source: DIAGNOSTIC_SOURCE.to_owned().wrap_some(),
            message: error.item.to_string(),
            ..Default::default()
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_compiler::{ClientId, IsographDb, iso_literal_parse_errors_in_file};
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use prelude::Postfix;

    use super::lsp_diagnostics_for_file;

    fn client() -> ClientId {
        ClientId(1)
    }

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn diagnostics_of(contents: &str) -> Vec<lsp_types::Diagnostic> {
        let mut db = IsographDb::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, contents.to_owned());
        lsp_diagnostics_for_file(&db, client(), path)
            .clone()
            .expect("the test interned this path")
    }

    #[test]
    fn missing_file_has_no_diagnostics() {
        let db = IsographDb::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        assert_eq!(lsp_diagnostics_for_file(&db, client(), path).as_ref(), None);
    }

    #[test]
    fn file_with_no_literals_has_empty_diagnostics() {
        assert_eq!(diagnostics_of("const a = 1;"), Vec::new());
    }

    #[test]
    fn each_diagnostic_is_an_error_from_isograph_with_the_parse_error_message() {
        let mut db = IsographDb::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "iso(`entrypoint`)".to_owned());
        let errors = iso_literal_parse_errors_in_file(&db, client(), path)
            .clone()
            .expect("the test interned this path");
        let diagnostics = lsp_diagnostics_for_file(&db, client(), path)
            .clone()
            .expect("the test interned this path");
        assert!(!errors.is_empty());
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
            errors
                .iter()
                .map(|error| error.item.to_string())
                .collect::<Vec<_>>()
        );
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.severity == lsp_types::DiagnosticSeverity::ERROR.wrap_some()
                && diagnostic.source == "isograph".to_owned().wrap_some()
                && diagnostic.code.is_none()
        }));
    }

    #[test]
    fn crlf_and_lone_cr_before_the_literal_put_the_range_on_line_one() {
        for contents in [
            "const a = 1;\r\niso(`entrypoint`)",
            "const a = 1;\riso(`entrypoint`)",
        ] {
            let diagnostics = diagnostics_of(contents);
            assert!(!diagnostics.is_empty());
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.range.start.line == 1),
                "{contents:?}"
            );
        }
    }

    #[test]
    fn a_surrogate_pair_before_the_literal_adds_two_to_the_start_character() {
        let plain = diagnostics_of("iso(`entrypoint`)");
        let wide = diagnostics_of("😀iso(`entrypoint`)");
        assert!(!plain.is_empty());
        assert_eq!(
            wide.iter()
                .map(|diagnostic| diagnostic.range.start.character)
                .collect::<Vec<_>>(),
            plain
                .iter()
                .map(|diagnostic| diagnostic.range.start.character + 2)
                .collect::<Vec<_>>()
        );
    }
}
