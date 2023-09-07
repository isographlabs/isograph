use std::{fmt, ops::Deref};

use crate::{Directive, TypeAnnotation};

use super::{write_arguments, write_directives, ConstantValue};
use common_lang_types::{
    DescriptionValue, InputTypeName, InputValueName, InterfaceTypeName, ObjectTypeName,
    ScalarTypeName, SelectableFieldName, UnvalidatedTypeName, WithSpan,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum TypeSystemDefinition {
    ObjectTypeDefinition(ObjectTypeDefinition),
    ScalarTypeDefinition(ScalarTypeDefinition),
    InterfaceTypeDefinition(InterfaceTypeDefinition),
    // Union
    // Enum
    // InputObject

    // Schema
    // Directive
}

impl From<ObjectTypeDefinition> for TypeSystemDefinition {
    fn from(type_definition: ObjectTypeDefinition) -> Self {
        Self::ObjectTypeDefinition(type_definition)
    }
}

impl From<InterfaceTypeDefinition> for TypeSystemDefinition {
    fn from(type_definition: InterfaceTypeDefinition) -> Self {
        Self::InterfaceTypeDefinition(type_definition)
    }
}

impl From<ScalarTypeDefinition> for TypeSystemDefinition {
    fn from(type_definition: ScalarTypeDefinition) -> Self {
        Self::ScalarTypeDefinition(type_definition)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct TypeSystemDocument(pub Vec<TypeSystemDefinition>);

impl Deref for TypeSystemDocument {
    type Target = Vec<TypeSystemDefinition>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TypeSystemDefinition: SchemaDef, TypeDef, DirectiveDef

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ObjectTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<ObjectTypeName>,
    pub interfaces: Vec<WithSpan<InterfaceTypeName>>,
    pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<WithSpan<OutputFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ScalarTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<ScalarTypeName>,
    pub directives: Vec<Directive<ConstantValue>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InterfaceTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<InterfaceTypeName>,
    pub interfaces: Vec<WithSpan<InterfaceTypeName>>,
    pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<WithSpan<OutputFieldDefinition>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct OutputFieldDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<SelectableFieldName>,
    pub type_: TypeAnnotation<UnvalidatedTypeName>,
    pub arguments: Vec<WithSpan<InputValueDefinition>>,
    pub directives: Vec<Directive<ConstantValue>>,
}

impl fmt::Display for OutputFieldDefinition {
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
pub struct InputValueDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<InputValueName>,
    pub type_: TypeAnnotation<InputTypeName>,
    pub default_value: Option<WithSpan<ConstantValue>>,
    pub directives: Vec<Directive<ConstantValue>>,
}

impl fmt::Display for InputValueDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.type_)?;
        if let Some(v) = &self.default_value {
            write!(f, " = {}", v)?;
        }

        write_directives(f, &self.directives)?;

        Ok(())
    }
}
