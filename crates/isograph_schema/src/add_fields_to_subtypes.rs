use common_lang_types::{Location, WithLocation};

use crate::{
    ProcessTypeDefinitionError, ProcessTypeDefinitionResult, TypeRefinementMap, UnvalidatedSchema,
};

impl UnvalidatedSchema {
    /// For each supertype (e.g. Node), add the fields defined on it (e.g. Node.MyComponent)
    /// to subtypes (e.g. creating User.MyComponent).
    ///
    /// We do not transfer server fields (because that makes no sense in GraphQL, but does
    /// it make sense otherwise??) and refetch fields (which are already defined on all valid
    /// types.)
    ///
    /// TODO confirm we don't do this for unions...
    pub fn add_fields_to_subtypes(
        &mut self,
        supertype_to_subtype_map: &TypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (supertype_id, subtype_ids) in supertype_to_subtype_map {
            let supertype = self.server_field_data.object(*supertype_id);

            // TODO is there a way to do this without cloning? I would think so, in theory,
            // if you could prove (or check at runtime?) that the supertype and subtype are not
            // the same item.
            let supertype_encountered_fields = supertype.encountered_fields.clone();

            for subtype_id in subtype_ids {
                for (supertype_field_name, defined_field) in &supertype_encountered_fields {
                    match defined_field {
                        crate::FieldDefinitionLocation::Server(_) => {
                            // Should we transfer server fields??? That makes no sense for
                            // GraphQL, but ... does it make sense otherwise? Who knows.
                        }
                        crate::FieldDefinitionLocation::Client(_) => {
                            let subtype = self.server_field_data.object_mut(*subtype_id);

                            if subtype
                                .encountered_fields
                                .insert(*supertype_field_name, *defined_field)
                                .is_some()
                            {
                                return Err(WithLocation::new(
                                    ProcessTypeDefinitionError::FieldExistsOnSubtype {
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
