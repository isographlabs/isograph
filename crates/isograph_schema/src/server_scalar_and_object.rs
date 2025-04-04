use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, GraphQLScalarTypeName, IsographObjectTypeName, JavascriptName,
    SelectableName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLConstantValue, GraphQLDirective};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    DefinitionLocation, HasId, SelectionType, ServerObjectId, ServerScalarId, ServerStrongIdFieldId,
};

use crate::{ClientFieldOrPointerId, NetworkProtocol, ServerSelectableId};

/// A scalar type in the schema.
#[derive(Debug)]
pub struct SchemaScalar<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLScalarTypeName>,
    pub javascript_name: JavascriptName,
    pub output_format: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> HasId for SchemaScalar<TNetworkProtocol> {
    type Id = ServerScalarId;
}

impl<TNetworkProtocol: NetworkProtocol> HasId for &SchemaScalar<TNetworkProtocol> {
    type Id = ServerScalarId;
}

impl<TNetworkProtocol: NetworkProtocol> HasId for &mut SchemaScalar<TNetworkProtocol> {
    type Id = ServerScalarId;
}

pub type ObjectEncounteredFields =
    BTreeMap<SelectableName, DefinitionLocation<ServerSelectableId, ClientFieldOrPointerId>>;

/// An object type in the schema.
#[derive(Debug)]
pub struct SchemaObject<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: IsographObjectTypeName,
    // We probably don't want this
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    /// TODO remove id_field from fields, and change the type of Option<ServerFieldId>
    /// to something else.
    pub id_field: Option<ServerStrongIdFieldId>,
    pub encountered_fields: ObjectEncounteredFields,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,

    pub output_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

impl<TNetworkProtocol: NetworkProtocol> HasId for SchemaObject<TNetworkProtocol> {
    type Id = ServerObjectId;
}

impl<TNetworkProtocol: NetworkProtocol> HasId for &SchemaObject<TNetworkProtocol> {
    type Id = ServerObjectId;
}

impl<TNetworkProtocol: NetworkProtocol> HasId for &mut SchemaObject<TNetworkProtocol> {
    type Id = ServerObjectId;
}

pub type SchemaType<'a, TNetworkProtocol> =
    SelectionType<&'a SchemaScalar<TNetworkProtocol>, &'a SchemaObject<TNetworkProtocol>>;

#[impl_for_selection_type]
pub trait SchemaScalarOrObject {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName>;
    fn description(&self) -> Option<DescriptionValue>;
}

impl<TNetworkProtocol: NetworkProtocol> SchemaScalarOrObject for &SchemaScalar<TNetworkProtocol> {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName> {
        SelectionType::Scalar(self.name.item)
    }

    fn description(&self) -> Option<DescriptionValue> {
        self.description.map(|x| x.item)
    }
}

impl<TNetworkProtocol: NetworkProtocol> SchemaScalarOrObject for &SchemaObject<TNetworkProtocol> {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName> {
        SelectionType::Object(self.name)
    }

    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }
}
