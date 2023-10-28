use common_lang_types::{ScalarFieldName, UnvalidatedTypeName, WithSpan};

#[derive(Debug, Copy, Clone)]
pub struct ResolverFetch {
    pub parent_type: WithSpan<UnvalidatedTypeName>,
    pub resolver_field_name: WithSpan<ScalarFieldName>,
}
