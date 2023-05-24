use std::{fmt, ops::Deref};

use crate::{Directive, TypeAnnotation};

use super::{write_arguments, write_directives, ConstantValue};
use common_lang_types::{
    DescriptionValue, FieldDefinitionName, InputTypeName, InputValueName, InterfaceTypeName,
    ObjectTypeName, UnvalidatedTypeName, WithSpan,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum TypeSystemDefinition {
    ObjectTypeDefinition(ObjectTypeDefinition),
    // Scalar
    // Interface
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

// TODO maybe parametrize with T: TypeTrait
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct OutputFieldDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<FieldDefinitionName>,
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
