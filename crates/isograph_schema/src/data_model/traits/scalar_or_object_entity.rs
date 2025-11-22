use common_lang_types::{
    SelectableName, ServerObjectEntityName, ServerScalarEntityName, WithLocation,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    Description, ObjectSelectionPath, ScalarSelectionPath, SelectionParentType, SelectionType,
    SelectionTypePostfix, ServerObjectEntityNameWrapper,
};
use thiserror::Error;

use crate::{
    ClientOrServerObjectSelectable, EntityAccessError, IsographDatabase, NetworkProtocol,
    OwnedObjectSelectable, ScalarSelectable, Selectable, SelectableNamedError, ServerObjectEntity,
    ServerScalarEntity, selectable_named, server_object_entity_named,
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
        self.name.scalar_selected()
    }

    fn description(&self) -> Option<Description> {
        self.description.map(|x| x.item)
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for ServerObjectEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<ServerScalarEntityName, ServerObjectEntityName> {
        self.name.object_selected()
    }

    fn description(&self) -> Option<Description> {
        self.description
    }
}

pub fn get_parent_and_selectable_for_scalar_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    scalar_path: &ScalarSelectionPath<'a>,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        ScalarSelectable<TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    let ScalarSelectionPath { parent, inner } = scalar_path;
    let scalar_selectable_name = inner.name.item;

    let (parent, selectable) =
        get_parent_and_selectable_for_selection_parent(db, parent, scalar_selectable_name.into())?;

    let selectable =
        selectable
            .as_scalar()
            .ok_or_else(|| GetParentAndSelectableError::FieldWrongType {
                parent_type_name: parent.name.into(),
                field_name: scalar_selectable_name.into(),
                must_be: "a scalar",
                is: "an object",
            })?;

    Ok((parent, selectable))
}

pub fn get_parent_and_selectable_for_object_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    object_path: &ObjectSelectionPath<'a>,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        OwnedObjectSelectable<TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    let ObjectSelectionPath { parent, inner } = object_path;
    let object_selectable_name = inner.name.item;

    let (parent, selectable) =
        get_parent_and_selectable_for_selection_parent(db, parent, object_selectable_name.into())?;

    let selectable =
        selectable
            .as_object()
            .ok_or_else(|| GetParentAndSelectableError::FieldWrongType {
                parent_type_name: parent.name.into(),
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
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        Selectable<TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    match selection_parent {
        SelectionParentType::ObjectSelection(object_selection_path) => {
            let (_, object_selectable) =
                get_parent_and_selectable_for_object_path(db, object_selection_path)?;

            let object_parent_entity_name = *object_selectable.target_object_entity_name().inner();

            parent_object_entity_and_selectable(
                db,
                object_parent_entity_name.into(),
                selectable_name,
            )
        }
        SelectionParentType::ClientFieldDeclaration(client_field_declaration_path) => {
            let parent_type_name = client_field_declaration_path.inner.parent_type.item;

            parent_object_entity_and_selectable(db, parent_type_name, selectable_name)
        }
        SelectionParentType::ClientPointerDeclaration(client_pointer_declaration_path) => {
            let parent_type_name = client_pointer_declaration_path.inner.parent_type.item;

            parent_object_entity_and_selectable(db, parent_type_name, selectable_name)
        }
    }
}

pub fn parent_object_entity_and_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityNameWrapper,
    selectable_name: SelectableName,
) -> Result<
    (
        ServerObjectEntity<TNetworkProtocol>,
        Selectable<TNetworkProtocol>,
    ),
    GetParentAndSelectableError<TNetworkProtocol>,
> {
    let parent_entity = server_object_entity_named(db, parent_server_object_entity_name.0)
        .to_owned()?
        .ok_or(GetParentAndSelectableError::ParentTypeNotDefined {
            parent_type_name: parent_server_object_entity_name,
        })?
        .item;

    match selectable_named(db, parent_server_object_entity_name.0, selectable_name).to_owned()? {
        Some(selectable) => Ok((parent_entity, selectable)),
        None => Err(GetParentAndSelectableError::FieldMustExist {
            parent_type_name: parent_server_object_entity_name,
            field_name: selectable_name,
        }),
    }
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

    #[error("{0}")]
    SelectableNamedError(#[from] SelectableNamedError<TNetworkProtocol>),
}
