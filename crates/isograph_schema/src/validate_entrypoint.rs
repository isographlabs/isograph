use common_lang_types::IsoLiteralText;
use isograph_lang_types::EntrypointDirectiveSet;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EntrypointDeclarationInfo {
    pub iso_literal_text: IsoLiteralText,
    pub directive_set: EntrypointDirectiveSet,
}
