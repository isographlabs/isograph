use std::fmt;

use common_lang_types::{
    IsographDirectiveName, IsographObjectTypeName, Location, SelectableFieldName, TextSource,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLInputValueDefinition;
use intern::string_key::Intern;
use isograph_lang_types::{
    ClientFieldDeclaration, FragmentDirectiveUsage, ObjectId, SelectableFieldId,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    ClientField, ClientFieldActionKind, FieldDefinitionLocation, ObjectTypeAndFieldNames,
    UnvalidatedSchema,
};

impl UnvalidatedSchema {
    pub fn process_client_field_declaration(
        &mut self,
        client_field_declaration: WithSpan<ClientFieldDeclaration>,
        text_source: TextSource,
    ) -> Result<(), WithLocation<ProcessClientFieldDeclarationError>> {
        let parent_type_id = self
            .schema_data
            .defined_types
            .get(&client_field_declaration.item.parent_type.item.into())
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: client_field_declaration.item.parent_type.item,
                },
                Location::new(text_source, client_field_declaration.item.parent_type.span),
            ))?;

        match parent_type_id {
            SelectableFieldId::Object(object_id) => {
                self.add_resolver_field_to_object(*object_id, client_field_declaration)
                    .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?;
            }
            SelectableFieldId::Scalar(scalar_id) => {
                let scalar_name = self.schema_data.scalars[scalar_id.as_usize()].name;
                return Err(WithLocation::new(
                    ProcessClientFieldDeclarationError::InvalidParentType {
                        parent_type_name: scalar_name.item.into(),
                    },
                    Location::new(text_source, client_field_declaration.item.parent_type.span),
                ));
            }
        }

        Ok(())
    }

    fn add_resolver_field_to_object(
        &mut self,
        parent_object_id: ObjectId,
        client_field_declaration: WithSpan<ClientFieldDeclaration>,
    ) -> ProcessResolverDeclarationResult<()> {
        let object = &mut self.schema_data.objects[parent_object_id.as_usize()];
        let resolver_field_name_ws = client_field_declaration.item.client_field_name;
        let resolver_field_name = resolver_field_name_ws.item;
        let resolver_field_name_span = resolver_field_name_ws.span;

        let next_resolver_id = self.client_fields.len().into();

        if object
            .encountered_fields
            .insert(
                resolver_field_name.into(),
                FieldDefinitionLocation::Client(next_resolver_id),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                    parent_type_name: object.name.into(),
                    resolver_field_name: resolver_field_name.into(),
                },
                resolver_field_name_span,
            ));
        }

        object.resolvers.push(next_resolver_id);

        let name = client_field_declaration.item.client_field_name.item.into();
        let variant = get_resolver_variant(&client_field_declaration.item.directives);
        let action_kind = ClientFieldActionKind::NamedImport((
            client_field_declaration.item.const_export_name,
            client_field_declaration.item.definition_path,
        ));

        // TODO variant should carry payloads, instead of this check
        if variant == ClientFieldVariant::Component {
            if !matches!(action_kind, ClientFieldActionKind::NamedImport(_)) {
                return Err(WithSpan::new(
                    ProcessClientFieldDeclarationError::ComponentResolverMissingJsFunction,
                    resolver_field_name_span,
                ));
            }
        }

        self.client_fields.push(ClientField {
            description: None,
            name,
            id: next_resolver_id,
            selection_set_and_unwraps: client_field_declaration.item.selection_set_and_unwraps,
            variant,
            variable_definitions: client_field_declaration.item.variable_definitions,
            type_and_field: ObjectTypeAndFieldNames {
                type_name: object.name,
                field_name: name,
            },

            parent_object_id,
            action_kind,
        });
        Ok(())
    }
}

type ProcessResolverDeclarationResult<T> = Result<T, WithSpan<ProcessClientFieldDeclarationError>>;

#[derive(Error, Debug)]
pub enum ProcessClientFieldDeclarationError {
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
pub struct MutationFieldClientFieldVariant {
    pub mutation_field_name: SelectableFieldName,
    pub server_schema_mutation_field_name: SelectableFieldName,
    pub mutation_primary_field_name: SelectableFieldName,
    pub mutation_primary_field_return_type_object_id: ObjectId,
    pub mutation_field_arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    pub filtered_mutation_field_arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientFieldVariant {
    Component,
    Eager,
    RefetchField,
    MutationField(MutationFieldClientFieldVariant),
}

impl fmt::Display for ClientFieldVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientFieldVariant::Component => write!(f, "Component"),
            ClientFieldVariant::Eager => write!(f, "Eager"),
            ClientFieldVariant::RefetchField => write!(f, "RefetchField"),
            ClientFieldVariant::MutationField(_) => write!(f, "MutationField"),
        }
    }
}

lazy_static! {
    static ref COMPONENT: IsographDirectiveName = "component".intern().into();
}

fn get_resolver_variant(directives: &[WithSpan<FragmentDirectiveUsage>]) -> ClientFieldVariant {
    for directive in directives.iter() {
        if directive.item.name.item == *COMPONENT {
            return ClientFieldVariant::Component;
        }
    }
    return ClientFieldVariant::Eager;
}
