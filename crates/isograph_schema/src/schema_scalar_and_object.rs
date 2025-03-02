use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, GraphQLScalarTypeName, IsographObjectTypeName, JavascriptName,
    SelectableName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLConstantValue, GraphQLDirective};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    DefinitionLocation, SelectionType, ServerFieldId, ServerObjectId, ServerScalarId,
    ServerStrongIdFieldId,
};

use crate::{ClientFieldOrPointerId, OutputFormat};

/// A scalar type in the schema.
#[derive(Debug)]
pub struct SchemaScalar<TOutputFormat: OutputFormat> {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLScalarTypeName>,
    pub id: ServerScalarId,
    pub javascript_name: JavascriptName,
    pub output_format: PhantomData<TOutputFormat>,
}

/// An object type in the schema.
#[derive(Debug)]
pub struct SchemaObject<TOutputFormat: OutputFormat> {
    pub description: Option<DescriptionValue>,
    pub name: IsographObjectTypeName,
    pub id: ServerObjectId,
    // We probably don't want this
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    /// TODO remove id_field from fields, and change the type of Option<ServerFieldId>
    /// to something else.
    pub id_field: Option<ServerStrongIdFieldId>,
    pub encountered_fields:
        BTreeMap<SelectableName, DefinitionLocation<ServerFieldId, ClientFieldOrPointerId>>,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,

    pub output_associated_data: TOutputFormat::SchemaObjectAssociatedData,
}

pub type SchemaType<'a, TOutputFormat> =
    SelectionType<&'a SchemaScalar<TOutputFormat>, &'a SchemaObject<TOutputFormat>>;

#[impl_for_selection_type]
pub trait SchemaScalarOrObject {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName>;
}

impl<TOutputFormat: OutputFormat> SchemaScalarOrObject for &SchemaScalar<TOutputFormat> {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName> {
        SelectionType::Scalar(self.name.item)
    }
}

impl<TOutputFormat: OutputFormat> SchemaScalarOrObject for &SchemaObject<TOutputFormat> {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName> {
        SelectionType::Object(self.name)
    }
}
