use common_lang_types::{Location, WithLocation};

use crate::{
    ProcessTypeDefinitionError, ProcessTypeDefinitionResult, ResolverVariant, TypeRefinementMap,
    UnvalidatedSchema,
};

impl UnvalidatedSchema {
    /// For each supertype (e.g. Node), add the fields defined on it (e.g. Node.MyComponent)
    /// to subtypes (e.g. creating User.MyComponent).
    ///
    /// We do not transfer server fields (because that makes no sense in GraphQL, but does
    /// it make sense otherwise??) and refetch fields (which are already defined on all valid
    /// types.)
    pub fn add_fields_to_subtypes(
        &mut self,
        supertype_to_subtype_map: &TypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (supertype_id, subtype_ids) in supertype_to_subtype_map {
            let supertype = self.schema_data.object(*supertype_id);

            // TODO is there a way to do this without cloning? I would think so, in theory,
            // if you could prove (or check at runtime?) that the supertype and subtype are not
            // the same item.
            let supertype_encountered_fields = supertype.encountered_fields.clone();

            for subtype_id in subtype_ids {
                'field: for (supertype_field_name, defined_field) in &supertype_encountered_fields {
                    match defined_field {
                        crate::DefinedField::ServerField(_) => {
                            // Should we transfer server fields??? That makes no sense for
                            // GraphQL, but ... does it make sense otherwise? Who knows.
                        }
                        crate::DefinedField::ResolverField(supertype_resolver_field_id) => {
                            // ------ HACK ------
                            // We error if there are fields that are duplicated. This makes sense — defining
                            // Interface.foo and also ConcreteType.foo is a recipe for confusion. So, if you
                            // define a field on an abstract type, it had better not already exist on the
                            // concrete type.
                            //
                            // __refetch fields are on *all* types, though. So, we have to skip those.
                            // However, if __refetch is defined on Node (or some other suitable abstract
                            // type), we *probably* can do this.
                            //
                            // What we have here is not currently a satisfactory conclusion.
                            // ---- END HACK ----
                            let resolver = self.resolver(*supertype_resolver_field_id);
                            if matches!(resolver.variant, ResolverVariant::RefetchField) {
                                continue 'field;
                            }
                            let subtype = self.schema_data.object_mut(*subtype_id);

                            if let Some(_) = subtype
                                .encountered_fields
                                .insert(*supertype_field_name, defined_field.clone())
                            {
                                return Err(WithLocation::new(
                                    ProcessTypeDefinitionError::FieldExistsOnSubtype {
                                        field_name: *supertype_field_name,
                                        parent_type: subtype.name,
                                    },
                                    Location::generated(),
                                ));
                            }
                            subtype.resolvers.push(*supertype_resolver_field_id);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
