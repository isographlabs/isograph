use common_lang_types::{Location, WithLocation};
use isograph_lang_types::DefinitionLocation;

use crate::{NetworkProtocol, Schema};

use super::create_additional_fields_error::{
    CreateAdditionalFieldsError, ProcessTypeDefinitionResult, ValidatedTypeRefinementMap,
};

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    /// For each supertype (e.g. Node), add the client selectables defined on it (e.g. Node.MyComponent)
    /// to subtypes (e.g. creating User.MyComponent).
    ///
    /// We do not transfer server selectables (because that makes no sense in GraphQL, but does
    /// it make sense otherwise??) and refetch fields (which are already defined on all valid
    /// types.)
    ///
    /// TODO confirm we don't do this for unions...
    pub fn transfer_supertype_client_selectables_to_subtypes(
        &mut self,
        supertype_to_subtype_map: &ValidatedTypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (supertype_id, subtype_ids) in supertype_to_subtype_map {
            let supertype = self.server_entity_data.server_object_entity(*supertype_id);

            // TODO is there a way to do this without cloning? I would think so, in theory,
            // if you could prove (or check at runtime?) that the supertype and subtype are not
            // the same item.
            let supertype_encountered_fields = supertype.available_selectables.clone();

            for subtype_id in subtype_ids {
                for (supertype_field_name, defined_field) in &supertype_encountered_fields {
                    match defined_field {
                        DefinitionLocation::Server(_) => {
                            // Should we transfer server fields??? That makes no sense for
                            // GraphQL, but ... does it make sense otherwise? Who knows.
                        }
                        DefinitionLocation::Client(_) => {
                            let subtype = self.server_entity_data.server_object_entity_mut(*subtype_id);

                            if subtype
                                .available_selectables
                                .insert(*supertype_field_name, *defined_field)
                                .is_some()
                            {
                                return Err(WithLocation::new(
                                    CreateAdditionalFieldsError::FieldExistsOnType {
                                        field_name: *supertype_field_name,
                                        parent_type: subtype.name,
                                    },
                                    Location::generated(),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
