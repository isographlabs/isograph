use common_lang_types::{
    SelectableName, ServerObjectEntityName, ServerScalarEntityName, WithLocation,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    DefinitionLocation, Description, ObjectSelectionPath, ScalarSelectionPath, SelectionParentType,
    SelectionType, ServerObjectEntityNameWrapper,
};
use std::ops::Deref;
use thiserror::Error;

use crate::{
    ClientOrServerObjectSelectable, EntityAccessError, IsographDatabase, NetworkProtocol,
    ObjectSelectable, ScalarSelectable, Schema, Selectable, ServerObjectEntity, ServerScalarEntity,
    server_object_entity_named, server_object_selectable_named, server_scalar_selectable_named,
};

#[impl_for_selection_type]
pub trait ServerScalarOrObjectEntity {
    fn name(&self) -> SelectionType<ServerScalarEntityName, ServerObjectEntityName>;
    fn description(&self) -> Option<Description>;
}

impl<T: ServerScalarOrObjectEntity> ServerScalarOrObjectEntity for WithLocation<T> {
    fn name(&self) -> SelectionType<ServerScalarEntityName, ServerObjectEntityName> {
        self.item.name()
    }

    fn description(&self) -> Option<Description> {
        self.item.description()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for ServerScalarEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<ServerScalarEntityName, ServerObjectEntityName> {
        SelectionType::Scalar(self.name.item)
    }

    fn description(&self) -> Option<Description> {
        self.description.map(|x| x.item)
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for ServerObjectEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<ServerScalarEntityName, ServerObjectEntityName> {
        SelectionType::Object(self.name.item)
    }

    fn description(&self) -> Option<Description> {
        self.description
    }
}

pub fn get_parent_and_selectable_for_scalar_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    scalar_path: &ScalarSelectionPath<'a>,
    validated_schema: &'a Schema<TNetworkProtocol>,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        ScalarSelectable<'a, TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    let ScalarSelectionPath { parent, inner } = scalar_path;
    let scalar_selectable_name = inner.name.item;

    let (parent, selectable) = get_parent_and_selectable_for_selection_parent(
        db,
        parent,
        scalar_selectable_name.into(),
        validated_schema,
    )?;

    let selectable =
        selectable
            .as_scalar()
            .ok_or_else(|| GetParentAndSelectableError::FieldWrongType {
                parent_type_name: parent.name.item.into(),
                field_name: scalar_selectable_name.into(),
                must_be: "a scalar",
                is: "an object",
            })?;

    Ok((parent, selectable))
}

pub fn get_parent_and_selectable_for_object_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    object_path: &ObjectSelectionPath<'a>,
    validated_schema: &'a Schema<TNetworkProtocol>,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        ObjectSelectable<'a, TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    let ObjectSelectionPath { parent, inner } = object_path;
    let object_selectable_name = inner.name.item;

    let (parent, selectable) = get_parent_and_selectable_for_selection_parent(
        db,
        parent,
        object_selectable_name.into(),
        validated_schema,
    )?;

    let selectable =
        selectable
            .as_object()
            .ok_or_else(|| GetParentAndSelectableError::FieldWrongType {
                parent_type_name: parent.name.item.into(),
                field_name: object_selectable_name.into(),
                must_be: "an object",
                is: "a scalar",
            })?;

    Ok((parent, selectable))
}

pub fn get_parent_and_selectable_for_selection_parent<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_parent: &SelectionParentType<'a>,
    selectable_name: SelectableName,
    validated_schema: &'a Schema<TNetworkProtocol>,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        Selectable<'a, TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    match selection_parent {
        SelectionParentType::ObjectSelection(object_selection_path) => {
            let (_, object_selectable) = get_parent_and_selectable_for_object_path(
                db,
                object_selection_path,
                validated_schema,
            )?;

            let object_parent_entity_name = *object_selectable.target_object_entity_name().inner();

            parent_object_entity_and_selectable(
                db,
                validated_schema,
                object_parent_entity_name.into(),
                selectable_name,
            )
        }
        SelectionParentType::ClientFieldDeclaration(client_field_declaration_path) => {
            let parent_type_name = client_field_declaration_path.inner.parent_type.item;

            parent_object_entity_and_selectable(
                db,
                validated_schema,
                parent_type_name,
                selectable_name,
            )
        }
        SelectionParentType::ClientPointerDeclaration(client_pointer_declaration_path) => {
            let parent_type_name = client_pointer_declaration_path.inner.parent_type.item;

            parent_object_entity_and_selectable(
                db,
                validated_schema,
                parent_type_name,
                selectable_name,
            )
        }
    }
}

pub fn parent_object_entity_and_selectable<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    validated_schema: &'a Schema<TNetworkProtocol>,
    parent_type_name: ServerObjectEntityNameWrapper,
    selectable_name: SelectableName,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        Selectable<'a, TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    let parent_entity = server_object_entity_named(db, parent_type_name.0)
        .to_owned()?
        .ok_or(GetParentAndSelectableError::ParentTypeNotDefined { parent_type_name })?
        .item;

    let extra_info = validated_schema
        .server_entity_data
        .get(&parent_type_name.0.unchecked_conversion())
        .expect(
            "Expected extra info to exist. \
            This is indicative of a bug in Isograph.",
        );

    let selectable_id = extra_info.selectables.get(&selectable_name).ok_or(
        GetParentAndSelectableError::FieldMustExist {
            parent_type_name,
            field_name: selectable_name,
        },
    )?;

    let selectable = match selectable_id {
        DefinitionLocation::Server(s) => match s {
            SelectionType::Scalar((parent_object_entity_name, server_scalar_selectable_name)) => {
                let memo_ref = server_scalar_selectable_named(
                    db,
                    *parent_object_entity_name,
                    (*server_scalar_selectable_name).into(),
                );
                let server_scalar_selectable = memo_ref
                    .deref()
                    .as_ref()
                    .expect(
                        "Expected validation to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    )
                    .as_ref()
                    .expect(
                        "Expected selectable to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .clone();
                DefinitionLocation::Server(SelectionType::Scalar(server_scalar_selectable))
            }
            SelectionType::Object((parent_object_entity_name, server_object_selectable_name)) => {
                let memo_ref = server_object_selectable_named(
                    db,
                    *parent_object_entity_name,
                    (*server_object_selectable_name).into(),
                );
                let server_object_selectable = memo_ref
                    .deref()
                    .as_ref()
                    .expect(
                        "Expected validation to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    )
                    .as_ref()
                    .expect(
                        "Expected selectable to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .clone();
                DefinitionLocation::Server(SelectionType::Object(server_object_selectable))
            }
        },
        DefinitionLocation::Client(client_selectable_id) => DefinitionLocation::Client(
            validated_schema
                .client_selectable(*client_selectable_id)
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                ),
        ),
    };

    Ok((parent_entity, selectable))
}

#[derive(Error, Debug)]
pub enum GetParentAndSelectableError<TNetworkProtocol: NetworkProtocol> {
    #[error("`{parent_type_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_type_name: ServerObjectEntityNameWrapper,
    },

    #[error("The field `{parent_type_name}.{field_name}` is not defined.")]
    FieldMustExist {
        parent_type_name: ServerObjectEntityNameWrapper,
        field_name: SelectableName,
    },

    #[error("The field `{parent_type_name}.{field_name}` is {is}, but must be {must_be}.")]
    FieldWrongType {
        parent_type_name: ServerObjectEntityNameWrapper,
        field_name: SelectableName,
        must_be: &'static str,
        is: &'static str,
    },

    #[error("{0}")]
    EntityAccessError(#[from] EntityAccessError<TNetworkProtocol>),
}
