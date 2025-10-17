use common_lang_types::{ClientScalarSelectableName, IsoLiteralText, WithSpan};
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;

use crate::{
    IsographResolvedNode, IsographSemanticToken, ServerObjectEntityNameWrapper,
    entrypoint_directive_set::EntrypointDirectiveSet,
};

// TODO should this be ObjectTypeAndFieldNames?
#[derive(Debug, Clone, Eq, PartialEq, Hash, ResolvePosition)]
#[resolve_position(parent_type=(), resolved_node=IsographResolvedNode<'a>)]
pub struct EntrypointDeclaration {
    #[resolve_field]
    pub parent_type: WithSpan<ServerObjectEntityNameWrapper>,
    // N.B. there is no reason this can't be a server field name /shrug
    pub client_field_name: WithSpan<ClientScalarSelectableName>,

    // TODO consider moving this behind a cfg flag, since this is only used
    // by the language server.
    pub entrypoint_keyword: WithSpan<()>,
    pub dot: WithSpan<()>,
    pub iso_literal_text: IsoLiteralText,
    pub entrypoint_directive_set: EntrypointDirectiveSet,

    pub semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub type EntrypointDeclarationPath<'a> = PositionResolutionPath<&'a EntrypointDeclaration, ()>;
