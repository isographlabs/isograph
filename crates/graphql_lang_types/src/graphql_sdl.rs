use std::{fmt, ops::Deref};

use crate::{Directive, TypeAnnotation};

use super::{write_arguments, write_directives, ConstantValue};
use common_lang_types::{
    DescriptionValue, InputTypeName, InputValueName, InterfaceTypeName, ObjectTypeName,
    ScalarTypeName, SelectableFieldName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use intern::{string_key::Intern, Lookup};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum GraphQLTypeSystemDefinition {
    ObjectTypeDefinition(GraphQLObjectTypeDefinition),
    ScalarTypeDefinition(GraphQLScalarTypeDefinition),
    InterfaceTypeDefinition(GraphQLInterfaceTypeDefinition),
    InputObjectTypeDefinition(GraphQLInputObjectTypeDefinition),
    // Union
    // Enum
    // Schema
    // Directive
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLTypeSystemDocument(pub Vec<GraphQLTypeSystemDefinition>);

impl Deref for GraphQLTypeSystemDocument {
    type Target = Vec<GraphQLTypeSystemDefinition>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TypeSystemDefinition: SchemaDef, TypeDef, DirectiveDef

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLObjectTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<ObjectTypeName>,
    pub interfaces: Vec<WithSpan<InterfaceTypeName>>,
    pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLOutputFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLScalarTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<ScalarTypeName>,
    pub directives: Vec<Directive<ConstantValue>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLInterfaceTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<InterfaceTypeName>,
    pub interfaces: Vec<WithSpan<InterfaceTypeName>>,
    pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLOutputFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLInputObjectTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<InterfaceTypeName>,
    pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<WithLocation<GraphQLInputValueDefinition>>,
}

impl From<GraphQLInputValueDefinition> for GraphQLOutputFieldDefinition {
    fn from(value: GraphQLInputValueDefinition) -> Self {
        Self {
            description: value.description,
            // TODO make this zero cost?
            name: value.name.map(|x| x.lookup().intern().into()),
            type_: value.type_.map(|x| x.lookup().intern().into()),
            // Input object fields do not take arguments
            arguments: vec![],
            directives: value.directives,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLOutputFieldDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<SelectableFieldName>,
    pub type_: TypeAnnotation<UnvalidatedTypeName>,
    pub arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    pub directives: Vec<Directive<ConstantValue>>,
}

impl fmt::Display for GraphQLOutputFieldDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        write_arguments(f, &self.arguments)?;
        write!(f, ": {}", self.type_)?;
        write_directives(f, &self.directives)?;
        Ok(())
    }
}

/// This is an argument definition, but we're using the GraphQL spec lingo here.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GraphQLInputValueDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<InputValueName>,
    pub type_: TypeAnnotation<InputTypeName>,
    pub default_value: Option<WithSpan<ConstantValue>>,
    pub directives: Vec<Directive<ConstantValue>>,
}

impl fmt::Display for GraphQLInputValueDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.type_)?;
        if let Some(v) = &self.default_value {
            write!(f, " = {}", v)?;
        }

        write_directives(f, &self.directives)?;

        Ok(())
    }
}
