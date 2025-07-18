use std::{fmt, ops::Deref};

use crate::{GraphQLDirective, GraphQLTypeAnnotation};

use super::{write_arguments, write_directives, GraphQLConstantValue};
use common_lang_types::{
    DescriptionValue, DirectiveName, EnumLiteralValue, GraphQLInterfaceTypeName,
    GraphQLObjectTypeName, GraphQLUnionTypeName, InputTypeName, InputValueName,
    ServerScalarEntityName, ServerSelectableName, UnvalidatedTypeName, WithLocation, WithSpan,
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
pub struct GraphQLTypeSystemDocument(pub Vec<WithLocation<GraphQLTypeSystemDefinition>>);

impl Deref for GraphQLTypeSystemDocument {
    type Target = Vec<WithLocation<GraphQLTypeSystemDefinition>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLTypeSystemExtensionDocument(
    pub Vec<WithLocation<GraphQLTypeSystemExtensionOrDefinition>>,
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
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLObjectTypeName>,
    pub interfaces: Vec<WithLocation<GraphQLInterfaceTypeName>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLObjectTypeExtension {
    pub name: WithLocation<GraphQLObjectTypeName>,
    pub interfaces: Vec<WithLocation<GraphQLInterfaceTypeName>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLScalarTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<ServerScalarEntityName>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLInterfaceTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLInterfaceTypeName>,
    pub interfaces: Vec<WithLocation<GraphQLInterfaceTypeName>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLInputObjectTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLInterfaceTypeName>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLInputValueDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLSchemaDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub query: Option<WithLocation<GraphQLObjectTypeName>>,
    pub subscription: Option<WithLocation<GraphQLObjectTypeName>>,
    pub mutation: Option<WithLocation<GraphQLObjectTypeName>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[allow(unused)]
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
    Schema,
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
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<DirectiveName>,
    pub arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    pub repeatable: Option<WithSpan<()>>,
    pub locations: Vec<WithSpan<DirectiveLocation>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLEnumDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<DirectiveName>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub enum_value_definitions: Vec<WithLocation<GraphQLEnumValueDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLEnumValueDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub value: WithLocation<EnumLiteralValue>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLUnionTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLUnionTypeName>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub union_member_types: Vec<WithLocation<GraphQLObjectTypeName>>,
}

impl From<GraphQLInputValueDefinition> for GraphQLFieldDefinition {
    fn from(value: GraphQLInputValueDefinition) -> Self {
        Self {
            description: value.description,
            // TODO make this zero cost?
            name: value.name.map(|x| x.unchecked_conversion()),
            type_: value.type_.map(|x| x.unchecked_conversion()),
            // Input object fields do not take arguments
            arguments: vec![],
            directives: value.directives,
            is_inline_fragment: false,
        }
    }
}

/// A server field definition on an object or interface
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLFieldDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<ServerSelectableName>,
    pub type_: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    pub arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,

    // TODO we can probably restructure things to make this less awkward.
    // As in, we should not return GraphQLFieldDefinitions to the isograph side,
    // which is GraphQL-agnostic, and instead pass field definitions. These field
    // definitions should have an associated_data: TNetworkProtocol::FieldAssociatedData
    // or the like, which should carry this info.
    //
    // Then, that should be consumed by NetworkProtocol::generate_query_text, and also
    // somehow by generate_merged_selection_set. (Is a merged selection set something
    // that the network protocol should care about?? I don't think so, but how else
    // do we add the __typename and link selections?)
    pub is_inline_fragment: bool,
}

impl fmt::Display for GraphQLFieldDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        write_arguments(f, &self.arguments)?;
        write!(f, ": {}", self.type_)?;
        write_directives(f, &self.directives)?;
        Ok(())
    }
}

/// This is an argument definition, but we're using the GraphQL spec lingo here.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct GraphQLInputValueDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<InputValueName>,
    pub type_: GraphQLTypeAnnotation<InputTypeName>,
    // This unused, except for printing. Isograph does not care about this,
    // except inasmuch as it means that the type is nullable.
    pub default_value: Option<WithLocation<GraphQLConstantValue>>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
}

impl fmt::Display for GraphQLInputValueDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.type_)?;
        if let Some(v) = &self.default_value {
            write!(f, " = {v}")?;
        }

        write_directives(f, &self.directives)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum RootOperationKind {
    Query,
    Subscription,
    Mutation,
}
