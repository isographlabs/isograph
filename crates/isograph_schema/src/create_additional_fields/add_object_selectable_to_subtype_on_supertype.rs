use common_lang_types::{Span, WithLocation, WithSpan};
use graphql_lang_types::{GraphQLNamedTypeAnnotation, GraphQLTypeAnnotation};
use intern::string_key::Intern;
use isograph_lang_types::{DefinitionLocation, SelectionType, TypeAnnotation};

use crate::{NetworkProtocol, Schema, SchemaServerObjectSelectableVariant, ServerObjectSelectable};
use common_lang_types::Location;

use super::create_additional_fields_error::{
    CreateAdditionalFieldsError, ProcessTypeDefinitionResult, ValidatedTypeRefinementMap,
};
impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    /// For each supertype (e.g. Node), add the pointers for each subtype (e.g. User implements Node)
    /// to supertype (e.g. creating Node.asUser).
    pub fn add_object_selectable_to_subtype_on_supertypes(
        &mut self,
        subtype_to_supertype_map: &ValidatedTypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (subtype_id, supertype_ids) in subtype_to_supertype_map {
            let subtype_entity = self.server_entity_data.server_object_entity(*subtype_id);
            let subtype_entity_name = subtype_entity.name;

            for supertype_id in supertype_ids {
                let as_concrete_type_selectable_name =
                    format!("as{}", subtype_entity_name).intern().into();

                let next_server_object_selectable_id = self.server_object_selectables.len().into();

                let server_object_selectable = ServerObjectSelectable {
                    description: Some(
                        format!("A client pointer for the {} type.", subtype_entity_name)
                            .intern()
                            .into(),
                    ),
                    name: WithLocation::new(
                        as_concrete_type_selectable_name,
                        Location::generated(),
                    ),
                    parent_object_entity_id: *supertype_id,
                    arguments: vec![],
                    target_object_entity: TypeAnnotation::from_graphql_type_annotation(
                        GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                            *subtype_id,
                            Span::todo_generated(),
                        ))),
                    ),

                    phantom_data: std::marker::PhantomData,
                    object_selectable_variant: SchemaServerObjectSelectableVariant::InlineFragment,
                };

                self.server_object_selectables
                    .push(server_object_selectable);

                if self
                    .server_entity_data
                    .server_object_entity_available_selectables
                    .entry(*supertype_id)
                    .or_default()
                    .0
                    .insert(
                        as_concrete_type_selectable_name.into(),
                        DefinitionLocation::Server(SelectionType::Object(
                            next_server_object_selectable_id,
                        )),
                    )
                    .is_some()
                {
                    let supertype = self.server_entity_data.server_object_entity(*supertype_id);
                    return Err(WithLocation::new(
                        CreateAdditionalFieldsError::CompilerCreatedFieldExistsOnType {
                            field_name: as_concrete_type_selectable_name.into(),
                            parent_type: supertype.name,
                        },
                        Location::generated(),
                    ));
                }
            }
        }
        Ok(())
    }
}
