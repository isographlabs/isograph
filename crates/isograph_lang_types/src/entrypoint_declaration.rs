use common_lang_types::{ScalarFieldName, UnvalidatedTypeName, WithSpan};

// TODO should this be ObjectTypeAndFieldNames?
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct EntrypointTypeAndField {
    pub parent_type: WithSpan<UnvalidatedTypeName>,
    // N.B. there is no reason this can't be a server field name /shrug
    pub client_field_name: WithSpan<ScalarFieldName>,

    // TODO consider moving this behind a cfg flag, since this is only used
    // by the language server.
    pub entrypoint_keyword: WithSpan<()>,
    pub dot: WithSpan<()>,
}
