use common_lang_types::{IsoLiteralText, ServerScalarSelectableName, WithSpan};

use crate::{entrypoint_directive_set::EntrypointDirectiveSet, IsographSemanticToken, ParentType};

// TODO should this be ObjectTypeAndFieldNames?
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EntrypointDeclaration {
    pub parent_type: WithSpan<ParentType>,
    // N.B. there is no reason this can't be a server field name /shrug
    pub client_field_name: WithSpan<ServerScalarSelectableName>,

    // TODO consider moving this behind a cfg flag, since this is only used
    // by the language server.
    pub entrypoint_keyword: WithSpan<()>,
    pub dot: WithSpan<()>,
    pub iso_literal_text: IsoLiteralText,
    pub entrypoint_directive_set: EntrypointDirectiveSet,

    pub semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,
}
