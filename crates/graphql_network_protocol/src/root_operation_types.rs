use common_lang_types::EntityName;
use graphql_schema_parser::{
    DefinitionError, DefinitionName, OperationType, RootOperationTypeDefinition, SchemaDefinition,
    SchemaExtension, TypeSystemDefinition, TypeSystemDefinitionOrExtension, TypeSystemExtension,
};
use intern::string_key::Intern;
use isograph_compiler::{ClientId, CompilationProfile, IsographDb};
use pico_macros::memo;
use prelude::Postfix;
use span::WithSpanPostfix;

use crate::schema_files::schema_entries;
use crate::schema_lookup::InSchemaFile;

/// The root operation types of the schema as `client` reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootOperationTypes {
    pub query: EntityName,
    pub mutation: Option<EntityName>,
    pub subscription: Option<EntityName>,
}

impl RootOperationTypes {
    /// The operation whose root type is `entity`.
    pub fn operation_type_of(&self, entity: EntityName) -> Option<OperationType> {
        [
            (OperationType::Query, self.query.wrap_some()),
            (OperationType::Mutation, self.mutation),
            (OperationType::Subscription, self.subscription),
        ]
        .into_iter()
        .find_map(|(operation_type, root)| (root == entity.wrap_some()).then_some(operation_type))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RootOperationTypesError {
    #[error("The schema definition has a syntax error.")]
    Syntax(InSchemaFile<DefinitionError>),
    #[error("The schema is defined more than once.")]
    DuplicateSchemaDefinition(Box<DuplicateSchemaDefinition>),
    #[error("The schema is extended but never defined.")]
    ExtensionWithoutDefinition(Box<InSchemaFile<SchemaExtension>>),
    #[error("The root operation type `{}` is defined more than once.", .0.operation)]
    DuplicateOperation(DuplicateOperation),
    #[error("The schema definition has no `query` root operation type.")]
    MissingQuery(Box<InSchemaFile<SchemaDefinition>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateSchemaDefinition {
    pub first: InSchemaFile<SchemaDefinition>,
    pub second: InSchemaFile<SchemaDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateOperation {
    pub operation: OperationType,
    pub second: InSchemaFile<RootOperationTypeDefinition>,
}

/// The schema definitions, schema extensions, and first failed schema entry across every
/// schema file.
struct SchemaEntries {
    definitions: Vec<InSchemaFile<SchemaDefinition>>,
    extensions: Vec<InSchemaFile<SchemaExtension>>,
    first_failure: Option<InSchemaFile<DefinitionError>>,
}

/// The root type each operation type names so far.
#[derive(Default)]
struct RootSlots {
    query: Option<EntityName>,
    mutation: Option<EntityName>,
    subscription: Option<EntityName>,
}

#[memo]
pub fn root_operation_types<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Result<RootOperationTypes, RootOperationTypesError> {
    let entries = schema_entries(db, client).fold(
        SchemaEntries {
            definitions: Vec::new(),
            extensions: Vec::new(),
            first_failure: None,
        },
        |mut entries, schema_entry| {
            let path = schema_entry.path;
            match schema_entry.entry {
                Ok(entry) => match entry.item.reference() {
                    TypeSystemDefinitionOrExtension::Definition(TypeSystemDefinition::Schema(
                        definition,
                    )) => entries.definitions.push(InSchemaFile {
                        path,
                        item: definition.clone().with_span(entry.location),
                    }),
                    TypeSystemDefinitionOrExtension::Extension(TypeSystemExtension::Schema(
                        extension,
                    )) => entries.extensions.push(InSchemaFile {
                        path,
                        item: extension.clone().with_span(entry.location),
                    }),
                    TypeSystemDefinitionOrExtension::Definition(
                        TypeSystemDefinition::Type(_) | TypeSystemDefinition::Directive(_),
                    )
                    | TypeSystemDefinitionOrExtension::Extension(TypeSystemExtension::Type(_)) => {}
                },
                Err(failed) => {
                    if failed.item.name == DefinitionName::Schema.wrap_some()
                        && entries.first_failure.is_none()
                    {
                        entries.first_failure = InSchemaFile {
                            path,
                            item: failed.dereference(),
                        }
                        .wrap_some();
                    }
                }
            }
            entries
        },
    );
    if let Some(failed) = entries.first_failure {
        return RootOperationTypesError::Syntax(failed).wrap_err();
    }
    let mut definitions = entries.definitions.into_iter();
    let Some(definition) = definitions.next() else {
        return match entries.extensions.into_iter().next() {
            None => RootOperationTypes {
                query: "Query".intern().to(),
                mutation: "Mutation".intern().to::<EntityName>().wrap_some(),
                subscription: "Subscription".intern().to::<EntityName>().wrap_some(),
            }
            .wrap_ok(),
            Some(extension) => {
                RootOperationTypesError::ExtensionWithoutDefinition(extension.boxed()).wrap_err()
            }
        };
    };
    if let Some(second) = definitions.next() {
        return RootOperationTypesError::DuplicateSchemaDefinition(
            DuplicateSchemaDefinition {
                first: definition,
                second,
            }
            .boxed(),
        )
        .wrap_err();
    }
    let slots = std::iter::once((
        definition.path,
        definition.item.item.root_operation_types.as_slice(),
    ))
    .chain(entries.extensions.iter().map(|extension| {
        (
            extension.path,
            extension.item.item.root_operation_types.as_slice(),
        )
    }))
    .flat_map(|(path, roots)| {
        roots.iter().map(move |root| InSchemaFile {
            path,
            item: root.clone(),
        })
    })
    .try_fold(RootSlots::default(), |mut slots, root| {
        let operation = root.item.item.operation_type.item.0;
        let slot = match operation {
            OperationType::Query => &mut slots.query,
            OperationType::Mutation => &mut slots.mutation,
            OperationType::Subscription => &mut slots.subscription,
        };
        match slot {
            Some(_) => RootOperationTypesError::DuplicateOperation(DuplicateOperation {
                operation,
                second: root,
            })
            .wrap_err(),
            None => {
                *slot = root.item.item.type_name.item.0.wrap_some();
                slots.wrap_ok()
            }
        }
    })?;
    let Some(query) = slots.query else {
        return RootOperationTypesError::MissingQuery(definition.boxed()).wrap_err();
    };
    RootOperationTypes {
        query,
        mutation: slots.mutation,
        subscription: slots.subscription,
    }
    .wrap_ok()
}
