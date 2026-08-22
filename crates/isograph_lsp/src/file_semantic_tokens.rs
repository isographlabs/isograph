use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{HostLanguage, IsographState, parsed_iso_literals_in_file};
use pico_macros::memo;
use prelude::Postfix;

use crate::lsp_semantic_tokens;

#[memo]
pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let literals = parsed_iso_literals_in_file(db, path).as_ref()?;
    let page_content = db.disk_file(path)?.contents.reference();
    lsp_semantic_tokens(
        page_content,
        literals.iter().map(|(extraction, parsed)| {
            (extraction.iso_literal_start_index, parsed.tokens.as_slice())
        }),
    )
    .wrap_some()
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_compiler::IsographState;
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use prelude::Postfix;

    use super::lsp_semantic_tokens_for_file;

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn intern_file(
        db: &mut IsographState<TypeScriptHostLanguage>,
        path: RelativePathToSourceFile,
        contents: &str,
    ) {
        db.insert_disk_file(path, contents.to_owned());
    }

    const KEYWORD: u32 = 15;

    #[test]
    fn one_literal_encodes_entrypoint_as_keyword() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        intern_file(
            &mut db,
            path,
            "export const Home = iso(`entrypoint Query.HomeRoute`)",
        );
        let lsp = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].length, 10);
        assert_eq!(
            lsp[0].delta_start,
            "export const Home = iso(`".encode_utf16().count() as u32
        );
    }

    #[test]
    fn two_iso_interiors_encode_both_entrypoints() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        intern_file(
            &mut db,
            path,
            "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)",
        );
        let lsp = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        let mut keywords = lsp.iter().filter(|token| token.token_type == KEYWORD);
        let first = *keywords.next().expect("the first interior has entrypoint");
        let second = *keywords.next().expect("the second interior has entrypoint");
        assert!(keywords.next().is_none());
        assert_eq!(first.length, 10);
        assert_eq!(second.length, 10);
        assert_eq!(first.delta_start, "iso(`".encode_utf16().count() as u32);
        assert_eq!(second.delta_line, 1);
        assert_eq!(second.delta_start, "iso(`".encode_utf16().count() as u32);
    }

    #[test]
    fn empty_file_encodes_empty() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        intern_file(&mut db, path, "");
        let lsp = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert!(lsp.is_empty());
    }

    #[test]
    fn space_only_literal_encodes_empty() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        intern_file(&mut db, path, "iso(` `)");
        let lsp = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert!(lsp.is_empty());
    }

    #[test]
    fn remove_disk_file_is_none() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        intern_file(&mut db, path, "iso(`entrypoint Query.HomeRoute`)");
        assert!(lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path).is_some());
        db.remove_disk_file(path);
        assert!(lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path).is_none());
    }

    #[test]
    fn no_disk_file_is_none() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        assert!(lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path).is_none());
    }

    #[test]
    fn append_after_the_literal_keeps_encoded_tokens() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        let contents = "export const Home = iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path, contents);
        let before = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path")
            .clone();
        intern_file(&mut db, path, &(contents.to_owned() + "\nconst y = 1;\n"));
        let after = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(&before, after);
    }

    #[test]
    fn prefix_increments_delta_line_and_keeps_delta_start() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        let contents = "export const Home = iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path, contents);
        let before = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path")
            .clone();
        intern_file(&mut db, path, &("const x = 1;\n".to_owned() + contents));
        let after = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(after[0].delta_line, 1);
        assert_eq!(after[0].delta_start, before[0].delta_start);
        assert_eq!(after[0].token_type, KEYWORD);
        assert_eq!(after[0].length, 10);
    }

    #[test]
    fn context_only_export_rename_keeps_encoded_tokens() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        intern_file(
            &mut db,
            path,
            "export const Home = iso(`entrypoint Query.HomeRoute`)",
        );
        let before = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path")
            .clone();
        intern_file(
            &mut db,
            path,
            "export const Page = iso(`entrypoint Query.HomeRoute`)",
        );
        let after = lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(&before, after);
    }
}
