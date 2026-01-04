use std::ops::Deref;

use crate::{GraphQLDirective, GraphQLTypeAnnotation};

use super::GraphQLConstantValue;
use common_lang_types::{
    DescriptionValue, DirectiveName, EntityName, EnumLiteralValue, InputValueName, SelectableName,
    WithEmbeddedLocation,
};
use strum::EnumString;

// also Schema
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum GraphQLTypeSystemDefinition {
    ObjectTypeDefinition(GraphQLObjectTypeDefinition),
    ScalarTypeDefinition(GraphQLScalarTypeDefinition),
    InterfaceTypeDefinition(GraphQLInterfaceTypeDefinition),
    InputObjectTypeDefinition(GraphQLInputObjectTypeDefinition),
    DirectiveDefinition(GraphQLDirectiveDefinition),
    EnumDefinition(GraphQLEnumDefinition),
    UnionTypeDefinition(GraphQLUnionTypeDefinition),
    SchemaDefinition(GraphQLSchemaDefinition),
}

impl From<GraphQLObjectTypeDefinition> for GraphQLTypeSystemDefinition {
    fn from(type_definition: GraphQLObjectTypeDefinition) -> Self {
        Self::ObjectTypeDefinition(type_definition)
    }
}

impl From<GraphQLInterfaceTypeDefinition> for GraphQLTypeSystemDefinition {
    fn from(type_definition: GraphQLInterfaceTypeDefinition) -> Self {
        Self::InterfaceTypeDefinition(type_definition)
    }
}

impl From<GraphQLScalarTypeDefinition> for GraphQLTypeSystemDefinition {
    fn from(type_definition: GraphQLScalarTypeDefinition) -> Self {
        Self::ScalarTypeDefinition(type_definition)
    }
}

impl From<GraphQLInputObjectTypeDefinition> for GraphQLTypeSystemDefinition {
    fn from(type_definition: GraphQLInputObjectTypeDefinition) -> Self {
        Self::InputObjectTypeDefinition(type_definition)
    }
}

impl From<GraphQLDirectiveDefinition> for GraphQLTypeSystemDefinition {
    fn from(directive_definition: GraphQLDirectiveDefinition) -> Self {
        Self::DirectiveDefinition(directive_definition)
    }
}

impl From<GraphQLEnumDefinition> for GraphQLTypeSystemDefinition {
    fn from(enum_definition: GraphQLEnumDefinition) -> Self {
        Self::EnumDefinition(enum_definition)
    }
}

impl From<GraphQLUnionTypeDefinition> for GraphQLTypeSystemDefinition {
    fn from(union_type_definition: GraphQLUnionTypeDefinition) -> Self {
        Self::UnionTypeDefinition(union_type_definition)
    }
}

impl From<GraphQLSchemaDefinition> for GraphQLTypeSystemDefinition {
    fn from(schema_definition: GraphQLSchemaDefinition) -> Self {
        Self::SchemaDefinition(schema_definition)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default, Hash)]
pub struct GraphQLTypeSystemDocument(pub Vec<WithEmbeddedLocation<GraphQLTypeSystemDefinition>>);

impl Deref for GraphQLTypeSystemDocument {
    type Target = Vec<WithEmbeddedLocation<GraphQLTypeSystemDefinition>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLTypeSystemExtensionDocument(
    pub Vec<WithEmbeddedLocation<GraphQLTypeSystemExtensionOrDefinition>>,
);

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum GraphQLTypeSystemExtensionOrDefinition {
    Definition(GraphQLTypeSystemDefinition),
    Extension(GraphQLTypeSystemExtension),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum GraphQLTypeSystemExtension {
    ObjectTypeExtension(GraphQLObjectTypeExtension),
    // ScalarTypeExtension
    // InterfaceTypeExtension
    // UnionTypeExtension
    // EnumTypeExtension
    // InputObjectTypeExtension
    // SchemaExtension
}

impl From<GraphQLObjectTypeExtension> for GraphQLTypeSystemExtension {
    fn from(object_type_extension: GraphQLObjectTypeExtension) -> Self {
        Self::ObjectTypeExtension(object_type_extension)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLObjectTypeDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<EntityName>,
    pub interfaces: Vec<WithEmbeddedLocation<EntityName>>,
    // These should have locations
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithEmbeddedLocation<GraphQLFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLObjectTypeExtension {
    pub name: WithEmbeddedLocation<EntityName>,
    pub interfaces: Vec<WithEmbeddedLocation<EntityName>>,
    // These should have locations
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithEmbeddedLocation<GraphQLFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLScalarTypeDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<EntityName>,
    // These should have locations
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLInterfaceTypeDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<EntityName>,
    pub interfaces: Vec<WithEmbeddedLocation<EntityName>>,
    // These should have locations
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithEmbeddedLocation<GraphQLFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLInputObjectTypeDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<EntityName>,
    // These should have locations
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithEmbeddedLocation<GraphQLInputValueDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLSchemaDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub query: Option<WithEmbeddedLocation<EntityName>>,
    pub subscription: Option<WithEmbeddedLocation<EntityName>>,
    pub mutation: Option<WithEmbeddedLocation<EntityName>>,
    // These should have locations
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, EnumString, Hash)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    FragmentDefinition,
    FragmentSpread,
    InlineFragment,
    VariableDefinition,

    Scalar,
    Object,
    FieldDefinition,
    ArgumentDefinition,
    Interface,
    Union,
    Enum,
    EnumValue,
    InputObject,
    InputFieldDefinition,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLDirectiveDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<DirectiveName>,
    pub arguments: Vec<WithEmbeddedLocation<GraphQLInputValueDefinition>>,
    pub repeatable: Option<WithEmbeddedLocation<()>>,
    pub locations: Vec<WithEmbeddedLocation<DirectiveLocation>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLEnumDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<EntityName>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub enum_value_definitions: Vec<WithEmbeddedLocation<GraphQLEnumValueDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLEnumValueDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub value: WithEmbeddedLocation<EnumLiteralValue>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLUnionTypeDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<EntityName>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub union_member_types: Vec<WithEmbeddedLocation<EntityName>>,
}

impl From<GraphQLInputValueDefinition> for GraphQLFieldDefinition {
    fn from(value: GraphQLInputValueDefinition) -> Self {
        Self {
            description: value.description,
            // TODO make this zero cost?
            name: value.name.map(|x| x.unchecked_conversion()),
            type_: value.type_,
            // Input object fields do not take arguments
            arguments: vec![],
            directives: value.directives,
        }
    }
}

/// A server field definition on an object or interface
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLFieldDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<SelectableName>,
    pub type_: WithEmbeddedLocation<GraphQLTypeAnnotation>,
    pub arguments: Vec<WithEmbeddedLocation<GraphQLInputValueDefinition>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

/// This is an argument definition, but we're using the GraphQL spec lingo here.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLInputValueDefinition {
    pub description: Option<WithEmbeddedLocation<DescriptionValue>>,
    pub name: WithEmbeddedLocation<InputValueName>,
    pub type_: WithEmbeddedLocation<GraphQLTypeAnnotation>,
    // This unused, except for printing. Isograph does not care about this,
    // except inasmuch as it means that the type is nullable.
    pub default_value: Option<WithEmbeddedLocation<GraphQLConstantValue>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[derive(Debug)]
pub enum RootOperationKind {
    Query,
    Subscription,
    Mutation,
}
