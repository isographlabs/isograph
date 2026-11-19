mod argument_errors;
mod fetch_plan;
mod print_operation;
mod protocol_data;
mod query_text;
mod root_operation_types;
mod schema_files;
mod schema_lookup;
#[cfg(test)]
mod tests;

use std::path::Path;

use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, EntityKind, EntityShape, EntrypointOperation, EntrypointWrapper,
    IncludeFile, IsographDb, NetworkProtocol, ProtocolEntity, ProtocolQueryText,
    ProtocolSelectable, ScalarKind, ScalarValueKind, SelectableKey, SelectableShape,
};
use isograph_parser::{ArgumentList, VariableDeclarationList};
use prelude::Postfix;
use span::WithSpan;

pub use argument_errors::{
    ArgumentOf, GraphQLArgumentError, InputFieldOf, LiteralKind, LiteralTypeMismatch,
    VariableTypeMismatch,
};
pub use fetch_plan::{
    GraphQLFetchPlan, GraphQLNotFetchable, NodeFieldArgument, NodeFieldError,
    NodeFieldTypeMismatch, graphql_fetch_plan,
};
pub use protocol_data::{
    DuplicateInputField, GraphQLCompositeData, GraphQLEntityData, GraphQLSelectableData,
    GraphQLSelectableLookupError, IdField, RefinementCollision, RefinementTargetError,
    graphql_entity, graphql_selectable, graphql_selectables, object_types_implementing,
};
use protocol_data::{TYPENAME, graphql_entity_shape, graphql_selectable_shape, union_members};
pub use query_text::QUERY_TEXT_FILE_NAME;
pub use root_operation_types::{
    DuplicateOperation, DuplicateSchemaDefinition, RootOperationTypes, RootOperationTypesError,
    root_operation_types,
};
pub use schema_files::{GraphQLSchemaFileError, graphql_schema_file_errors, parsed_schema_file};
pub use schema_lookup::{
    DuplicateDefinition, DuplicateField, EntityLookupError, EntitySyntaxError,
    ExtensionKindMismatch, ExtensionWithoutDefinition, InSchemaFile, SchemaEntity,
    SelectableLookupError, entity_named, selectable_named,
};

pub struct GraphQLNetworkProtocol;

/// GraphQL's built-in scalars (spec 3.5) by name; every other scalar is custom.
pub fn scalar_kind(name: EntityName) -> ScalarKind {
    match name.lookup() {
        "String" | "ID" => ScalarKind::Known(ScalarValueKind::String),
        "Int" | "Float" => ScalarKind::Known(ScalarValueKind::Number),
        "Boolean" => ScalarKind::Known(ScalarValueKind::Boolean),
        _ => ScalarKind::Custom(name),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum GraphQLLookupError {
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("{0}")]
    Selectable(GraphQLSelectableLookupError),
}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type EntityData = GraphQLEntityData;
    type SelectableData = GraphQLSelectableData;
    type FetchPlan = GraphQLFetchPlan;
    type NotFetchable = GraphQLNotFetchable;
    type LookupError = GraphQLLookupError;
    type SchemaFileError = GraphQLSchemaFileError;
    type ArgumentError = GraphQLArgumentError;

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql" | "graphqls") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }

    fn schema_file_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<WithSpan<GraphQLSchemaFileError>>> {
        graphql_schema_file_errors(db, client, path).clone()
    }

    fn selection_argument_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        selectable: SelectableKey,
        selection_name: span::Span,
        arguments: Option<&WithSpan<ArgumentList>>,
        variables: Option<&WithSpan<VariableDeclarationList>>,
    ) -> Vec<WithSpan<GraphQLArgumentError>> {
        argument_errors::selection_argument_errors(
            db,
            client,
            selectable,
            selection_name,
            arguments,
            variables,
        )
    }

    fn field_argument_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        field_variables: Option<&WithSpan<VariableDeclarationList>>,
        arguments: Option<&WithSpan<ArgumentList>>,
        variables: Option<&WithSpan<VariableDeclarationList>>,
    ) -> Vec<WithSpan<GraphQLArgumentError>> {
        argument_errors::field_argument_errors(db, client, field_variables, arguments, variables)
    }

    fn entity_kind(data: &GraphQLEntityData) -> EntityKind {
        match data {
            GraphQLEntityData::Scalar(name) => EntityKind::Scalar(scalar_kind(name.dereference())),
            GraphQLEntityData::Enum => {
                EntityKind::Scalar(ScalarKind::Known(ScalarValueKind::String))
            }
            GraphQLEntityData::InputObject
            | GraphQLEntityData::Object(_)
            | GraphQLEntityData::Interface(_)
            | GraphQLEntityData::Union => EntityKind::Object,
        }
    }

    fn concrete_types<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        data: &GraphQLEntityData,
    ) -> Vec<EntityName> {
        match data {
            GraphQLEntityData::Object(_) | GraphQLEntityData::InputObject => entity.wrap_vec(),
            GraphQLEntityData::Interface(_) => {
                object_types_implementing(db, client, entity).clone()
            }
            GraphQLEntityData::Union => match entity_named(db, client, entity) {
                Ok(Some(schema_entity)) => union_members(schema_entity).into_iter().collect(),
                Ok(None) | Err(_) => Vec::new(),
            },
            GraphQLEntityData::Scalar(_) | GraphQLEntityData::Enum => Vec::new(),
        }
    }

    fn entity<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        name: EntityName,
    ) -> Result<Option<ProtocolEntity<GraphQLEntityData>>, GraphQLLookupError> {
        graphql_entity(db, client, name)
            .clone()
            .map_err(GraphQLLookupError::from)
    }

    fn selectable<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        selectable: SelectableName,
    ) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLLookupError> {
        graphql_selectable(db, client, entity, selectable)
            .clone()
            .map_err(GraphQLLookupError::from)
    }

    fn selectables<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Vec<(SelectableName, ProtocolSelectable<GraphQLSelectableData>)> {
        graphql_selectables(db, client, entity).clone()
    }

    fn fetch_plan<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Result<GraphQLFetchPlan, GraphQLNotFetchable> {
        graphql_fetch_plan(db, client, entity).clone()
    }

    fn entity_shape(data: &GraphQLEntityData) -> EntityShape {
        graphql_entity_shape(data)
    }

    fn selectable_shape(data: &GraphQLSelectableData) -> SelectableShape {
        graphql_selectable_shape(data)
    }

    fn discriminator() -> SelectableName {
        TYPENAME.intern().to()
    }

    fn entrypoint_wrapper(plan: &GraphQLFetchPlan) -> EntrypointWrapper {
        plan.wrapper.clone()
    }

    fn query_text(operation: &EntrypointOperation<'_, Self>) -> ProtocolQueryText {
        query_text::query_text(operation)
    }
}
