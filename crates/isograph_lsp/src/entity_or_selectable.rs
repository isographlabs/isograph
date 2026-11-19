use common_lang_types::DiagnosticResult;
use isograph_compiler::{
    ClientId, CompilationProfile, DefinitionLocation, Entity, IsographDb, Selectable, entity_named,
    selectable_for_selectable_name_path, selectable_for_selection_path,
};
use isograph_parser::IsographResolutionNode;
use prelude::Postfix;

/// The schema item a resolved position names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EntityOrSelectable {
    Entity(Entity),
    Selectable(Selectable),
}

impl EntityOrSelectable {
    pub(crate) fn definition(&self) -> DefinitionLocation {
        match self {
            EntityOrSelectable::Entity(entity) => entity.definition,
            EntityOrSelectable::Selectable(selectable) => selectable.definition,
        }
    }
}

/// The lookup of the schema item `node` names, as `client` reads the schema. `None` when
/// `node` names no schema item.
pub(crate) fn entity_or_selectable_of_node<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: &IsographResolutionNode<'_>,
) -> Option<DiagnosticResult<EntityOrSelectable>> {
    match node {
        IsographResolutionNode::EntityNameWrapper(entity_name_path) => {
            entity_named(db, client, entity_name_path.inner.0)
                .clone()
                .map(EntityOrSelectable::Entity)
                .wrap_some()
        }
        IsographResolutionNode::SelectableNameWrapper(selectable_name_path) => {
            selectable_for_selectable_name_path(db, client, selectable_name_path)
                .map(EntityOrSelectable::Selectable)
                .wrap_some()
        }
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => {
            selectable_for_selection_path(db, client, selection_name_path.parent.reference())
                .map(EntityOrSelectable::Selectable)
                .wrap_some()
        }
        IsographResolutionNode::IsoLiteralSlot(_)
        | IsographResolutionNode::ChunkedRoot(_)
        | IsographResolutionNode::EntityDeclaration(_)
        | IsographResolutionNode::EntrypointDeclaration(_)
        | IsographResolutionNode::SelectableDeclaration(_)
        | IsographResolutionNode::Description(_)
        | IsographResolutionNode::UnparsedChunkItems(_)
        | IsographResolutionNode::ChunkedLevel(_)
        | IsographResolutionNode::ChunkedGroup(_)
        | IsographResolutionNode::Chunk(_)
        | IsographResolutionNode::ChunkSeparator(_)
        | IsographResolutionNode::NonBracketToken(_)
        | IsographResolutionNode::OpenBracket(_)
        | IsographResolutionNode::CloseBracket(_)
        | IsographResolutionNode::ArgumentSlot(_)
        | IsographResolutionNode::ObjectEntrySlot(_)
        | IsographResolutionNode::ArgumentList(_)
        | IsographResolutionNode::ObjectLiteral(_)
        | IsographResolutionNode::Argument(_)
        | IsographResolutionNode::ObjectEntry(_)
        | IsographResolutionNode::ArgumentNameWrapper(_)
        | IsographResolutionNode::ValueKeyNameWrapper(_)
        | IsographResolutionNode::VariableUse(_)
        | IsographResolutionNode::VariableNameWrapper(_)
        | IsographResolutionNode::VariableDeclarationOrUsage(_)
        | IsographResolutionNode::VariableDeclarationSlot(_)
        | IsographResolutionNode::VariableDeclarationList(_)
        | IsographResolutionNode::VariableDeclaration(_)
        | IsographResolutionNode::StringLiteralValueWrapper(_)
        | IsographResolutionNode::IntegerValue(_)
        | IsographResolutionNode::BooleanValue(_)
        | IsographResolutionNode::NullValue(_)
        | IsographResolutionNode::SelectionSlot(_)
        | IsographResolutionNode::SelectionSet(_)
        | IsographResolutionNode::Selection(_)
        | IsographResolutionNode::NamedTypeAnnotation(_)
        | IsographResolutionNode::ListTypeAnnotation(_)
        | IsographResolutionNode::UnionTypeAnnotation(_)
        | IsographResolutionNode::IsographFieldDirectiveList(_)
        | IsographResolutionNode::IsographFieldDirective(_)
        | IsographResolutionNode::IsographDirectiveNameWrapper(_)
        | IsographResolutionNode::ListLiteral(_)
        | IsographResolutionNode::ListLiteralValue(_)
        | IsographResolutionNode::ListLiteralValueSlot(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, RelativePathToSourceFile};
    use intern::string_key::Intern;
    use isograph_compiler::{DefinitionLocation, Entity, Selectable};
    use prelude::Postfix;
    use span::Span;

    use super::EntityOrSelectable;

    fn location(path: &str, start: u32, end: u32) -> DefinitionLocation {
        DefinitionLocation {
            path: path.intern().to::<RelativePathToSourceFile>(),
            span: Span::new(start, end),
        }
    }

    #[test]
    fn the_definition_is_the_items_own() {
        let entity_definition = location("src/schema.graphql", 5, 8);
        let selectable_definition = location("src/Pet.ts", 20, 25);
        let entity = EntityOrSelectable::Entity(Entity {
            name: "Pet".intern().to(),
            definition: entity_definition,
        });
        let selectable = EntityOrSelectable::Selectable(Selectable {
            name: "owner".intern().to(),
            parent_entity: "Pet".intern().to(),
            description: None,
            target_entity: "User".intern().to::<EntityName>().wrap_some(),
            definition: selectable_definition,
        });
        assert_eq!(entity.definition(), entity_definition);
        assert_eq!(selectable.definition(), selectable_definition);
    }
}
