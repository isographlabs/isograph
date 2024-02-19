use std::fmt;

use common_lang_types::{
    IsographDirectiveName, IsographObjectTypeName, Location, SelectableFieldName, TextSource,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLInputValueDefinition;
use intern::string_key::Intern;
use isograph_lang_types::{DefinedTypeId, FragmentDirectiveUsage, ObjectId, ResolverDeclaration};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    DefinedField, ResolverActionKind, ResolverTypeAndField, SchemaResolver, UnvalidatedSchema,
};

impl UnvalidatedSchema {
    pub fn process_resolver_declaration(
        &mut self,
        resolver_declaration: WithSpan<ResolverDeclaration>,
        text_source: TextSource,
    ) -> Result<(), WithLocation<ProcessResolverDeclarationError>> {
        let parent_type_id = self
            .schema_data
            .defined_types
            .get(&resolver_declaration.item.parent_type.item.into())
            .ok_or(WithLocation::new(
                ProcessResolverDeclarationError::ParentTypeNotDefined {
                    parent_type_name: resolver_declaration.item.parent_type.item,
                },
                Location::new(text_source, resolver_declaration.item.parent_type.span),
            ))?;

        match parent_type_id {
            DefinedTypeId::Object(object_id) => {
                self.add_resolver_field_to_object(*object_id, resolver_declaration)
                    .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?;
            }
            DefinedTypeId::Scalar(scalar_id) => {
                let scalar_name = self.schema_data.scalars[scalar_id.as_usize()].name;
                return Err(WithLocation::new(
                    ProcessResolverDeclarationError::InvalidParentType {
                        parent_type_name: scalar_name.item.into(),
                    },
                    Location::new(text_source, resolver_declaration.item.parent_type.span),
                ));
            }
        }

        Ok(())
    }

    fn add_resolver_field_to_object(
        &mut self,
        parent_object_id: ObjectId,
        resolver_declaration: WithSpan<ResolverDeclaration>,
    ) -> ProcessResolverDeclarationResult<()> {
        let object = &mut self.schema_data.objects[parent_object_id.as_usize()];
        let resolver_field_name_ws = resolver_declaration.item.resolver_field_name;
        let resolver_field_name = resolver_field_name_ws.item;
        let resolver_field_name_span = resolver_field_name_ws.span;

        let next_resolver_id = self.resolvers.len().into();

        if object
            .encountered_fields
            .insert(
                resolver_field_name.into(),
                DefinedField::ResolverField(next_resolver_id),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessResolverDeclarationError::ParentAlreadyHasField {
                    parent_type_name: object.name.into(),
                    resolver_field_name: resolver_field_name.into(),
                },
                resolver_field_name_span,
            ));
        }

        object.resolvers.push(next_resolver_id);

        let name = resolver_declaration.item.resolver_field_name.item.into();
        let variant = get_resolver_variant(&resolver_declaration.item.directives);
        let resolver_action_kind = ResolverActionKind::NamedImport((
            resolver_declaration.item.const_export_name,
            resolver_declaration.item.resolver_definition_path,
        ));

        // TODO variant should carry payloads, instead of this check
        if variant == ResolverVariant::Component {
            if !matches!(resolver_action_kind, ResolverActionKind::NamedImport(_)) {
                return Err(WithSpan::new(
                    ProcessResolverDeclarationError::ComponentResolverMissingJsFunction,
                    resolver_field_name_span,
                ));
            }
        }

        self.resolvers.push(SchemaResolver {
            description: None,
            name,
            id: next_resolver_id,
            selection_set_and_unwraps: resolver_declaration.item.selection_set_and_unwraps,
            variant,
            variable_definitions: resolver_declaration.item.variable_definitions,
            type_and_field: ResolverTypeAndField {
                type_name: object.name,
                field_name: name,
            },

            parent_object_id,
            action_kind: resolver_action_kind,
        });
        Ok(())
    }
}

type ProcessResolverDeclarationResult<T> = Result<T, WithSpan<ProcessResolverDeclarationError>>;

#[derive(Error, Debug)]
pub enum ProcessResolverDeclarationError {
    #[error("`{parent_type_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("Invalid parent type. `{parent_type_name}` is a scalar. You are attempting to define a field on it. \
        In order to do so, the parent object must be an object, interface or union.")]
    InvalidParentType {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The Isograph object type \"{parent_type_name}\" already has a field named \"{resolver_field_name}\"."
    )]
    ParentAlreadyHasField {
        parent_type_name: IsographObjectTypeName,
        resolver_field_name: SelectableFieldName,
    },

    #[error(
        "Resolvers with @component must have associated javascript (i.e. iso(`...`) must be called as a function, as in iso(`...`)(MyComponent))"
    )]
    // TODO add parent type and resolver field name
    ComponentResolverMissingJsFunction,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MutationFieldResolverVariant {
    pub mutation_field_name: SelectableFieldName,
    pub mutation_primary_field_name: SelectableFieldName,
    pub mutation_primary_field_return_type_object_id: ObjectId,
    pub mutation_field_arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    pub filtered_mutation_field_arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolverVariant {
    Component,
    Eager,
    RefetchField,
    MutationField(MutationFieldResolverVariant),
}

impl fmt::Display for ResolverVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolverVariant::Component => write!(f, "Component"),
            ResolverVariant::Eager => write!(f, "Eager"),
            ResolverVariant::RefetchField => write!(f, "RefetchField"),
            ResolverVariant::MutationField(_) => write!(f, "MutationField"),
        }
    }
}

lazy_static! {
    static ref COMPONENT: IsographDirectiveName = "component".intern().into();
}

fn get_resolver_variant(directives: &[WithSpan<FragmentDirectiveUsage>]) -> ResolverVariant {
    for directive in directives.iter() {
        if directive.item.name.item == *COMPONENT {
            return ResolverVariant::Component;
        }
    }
    return ResolverVariant::Eager;
}
