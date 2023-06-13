use std::fmt;

use common_lang_types::{
    BoultonDirectiveName, DescriptionValue, FieldArgumentName, FieldDefinitionName,
    FieldNameOrAlias, HasName, LinkedFieldAlias, LinkedFieldName, ResolverDefinitionPath,
    ScalarFieldAlias, ScalarFieldName, UnvalidatedTypeName, ValidLinkedFieldType,
    ValidScalarFieldType, ValidTypeAnnotationInnerType, VariableName, WithSpan,
};
use graphql_lang_types::TypeAnnotation;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ResolverDeclaration {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub parent_type: WithSpan<UnvalidatedTypeName>,
    pub resolver_field_name: WithSpan<ScalarFieldName>,
    pub selection_set_and_unwraps: Option<SelectionSetAndUnwraps<(), ()>>,
    // TODO intern the path buf instead of the string?
    pub resolver_definition_path: ResolverDefinitionPath,
    pub directives: Vec<WithSpan<FragmentDirectiveUsage>>,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,

    pub has_associated_js_function: bool,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
/// Ugly name, but at least it makes clear this isn't a schema directive.
pub struct FragmentDirectiveUsage {
    pub name: WithSpan<BoultonDirectiveName>,
    // TODO arguments and such
}

/// TODO maybe get rid of
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct SelectionSetAndUnwraps<
    TScalarField: ValidScalarFieldType,
    TLinkedField: ValidLinkedFieldType,
> {
    pub selection_set: Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
}

impl<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType>
    SelectionSetAndUnwraps<TScalarField, TLinkedField>
{
    // pub fn map<TNewScalarField: ValidScalarFieldType, TNewLinkedField: ValidLinkedFieldType>(
    //     self,
    //     map: &mut impl FnMut(
    //         Selection<TScalarField, TLinkedField>,
    //     ) -> Selection<TNewScalarField, TNewLinkedField>,
    // ) -> SelectionSetAndUnwraps<TNewScalarField, TNewLinkedField> {
    //     SelectionSetAndUnwraps {
    //         selection_set: self.selection_set.into_iter().map(|x| x.map(map)).collect(),
    //         unwraps: self.unwraps,
    //     }
    // }

    pub fn and_then<
        TNewScalarField: ValidScalarFieldType,
        TNewLinkedField: ValidLinkedFieldType,
        E,
    >(
        self,
        map: &mut impl FnMut(
            WithSpan<Selection<TScalarField, TLinkedField>>,
        )
            -> Result<WithSpan<Selection<TNewScalarField, TNewLinkedField>>, E>,
    ) -> Result<SelectionSetAndUnwraps<TNewScalarField, TNewLinkedField>, E> {
        Ok(SelectionSetAndUnwraps {
            selection_set: self
                .selection_set
                .into_iter()
                .map(map)
                .collect::<Result<_, _>>()?,
            unwraps: self.unwraps,
        })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Selection<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType> {
    Field(FieldSelection<TScalarField, TLinkedField>),
    // FieldGroup(FieldGroupSelection),
}

impl<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType>
    Selection<TScalarField, TLinkedField>
{
    pub fn map<TNewScalarField: ValidScalarFieldType, TNewLinkedField: ValidLinkedFieldType>(
        self,
        map: &mut impl FnMut(
            FieldSelection<TScalarField, TLinkedField>,
        ) -> FieldSelection<TNewScalarField, TNewLinkedField>,
    ) -> Selection<TNewScalarField, TNewLinkedField> {
        match self {
            Selection::Field(field_selection) => Selection::Field(map(field_selection)),
        }
    }

    pub fn and_then<
        TNewScalarField: ValidScalarFieldType,
        TNewLinkedField: ValidLinkedFieldType,
        E,
    >(
        self,
        map: &mut impl FnMut(
            FieldSelection<TScalarField, TLinkedField>,
        ) -> Result<FieldSelection<TNewScalarField, TNewLinkedField>, E>,
    ) -> Result<Selection<TNewScalarField, TNewLinkedField>, E> {
        match self {
            Selection::Field(field_selection) => Ok(Selection::Field(map(field_selection)?)),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum FieldSelection<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType> {
    ScalarField(ScalarFieldSelection<TScalarField>),
    LinkedField(LinkedFieldSelection<TScalarField, TLinkedField>),
}

impl<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType>
    FieldSelection<TScalarField, TLinkedField>
{
    pub fn map<TNewScalarField: ValidScalarFieldType, TNewLinkedField: ValidLinkedFieldType>(
        self,
        map_scalar_field: &mut impl FnMut(
            ScalarFieldSelection<TScalarField>,
        ) -> ScalarFieldSelection<TNewScalarField>,
        map_linked_field: &mut impl FnMut(
            LinkedFieldSelection<TScalarField, TLinkedField>,
        )
            -> LinkedFieldSelection<TNewScalarField, TNewLinkedField>,
    ) -> FieldSelection<TNewScalarField, TNewLinkedField> {
        match self {
            FieldSelection::ScalarField(s) => FieldSelection::ScalarField(map_scalar_field(s)),
            FieldSelection::LinkedField(l) => FieldSelection::LinkedField(map_linked_field(l)),
        }
    }

    pub fn and_then<
        TNewScalarField: ValidScalarFieldType,
        TNewLinkedField: ValidLinkedFieldType,
        E,
    >(
        self,
        map_scalar_field: &mut impl FnMut(
            ScalarFieldSelection<TScalarField>,
        ) -> Result<ScalarFieldSelection<TNewScalarField>, E>,
        map_linked_field: &mut impl FnMut(
            LinkedFieldSelection<TScalarField, TLinkedField>,
        ) -> Result<
            LinkedFieldSelection<TNewScalarField, TNewLinkedField>,
            E,
        >,
    ) -> Result<FieldSelection<TNewScalarField, TNewLinkedField>, E> {
        match self {
            FieldSelection::ScalarField(s) => Ok(FieldSelection::ScalarField(map_scalar_field(s)?)),
            FieldSelection::LinkedField(l) => Ok(FieldSelection::LinkedField(map_linked_field(l)?)),
        }
    }
}

impl<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType> HasName
    for FieldSelection<TScalarField, TLinkedField>
{
    type Name = FieldDefinitionName;
    fn name(&self) -> Self::Name {
        match self {
            FieldSelection::ScalarField(s) => s.name.item.into(),
            FieldSelection::LinkedField(l) => l.name.item.into(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ScalarFieldSelection<TScalarField: ValidScalarFieldType> {
    pub name: WithSpan<ScalarFieldName>,
    pub reader_alias: Option<WithSpan<ScalarFieldAlias>>,
    pub normalization_alias: Option<WithSpan<ScalarFieldAlias>>,
    pub field: TScalarField,
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub arguments: Vec<WithSpan<SelectionFieldArgument>>,
}

impl<TScalarField: ValidScalarFieldType> ScalarFieldSelection<TScalarField> {
    pub fn map<U: ValidScalarFieldType>(
        self,
        map: &mut impl FnMut(TScalarField) -> U,
    ) -> ScalarFieldSelection<U> {
        ScalarFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            field: map(self.field),
            unwraps: self.unwraps,
            arguments: self.arguments,
            normalization_alias: self.normalization_alias,
        }
    }

    pub fn and_then<U: ValidScalarFieldType, E>(
        self,
        map: &mut impl FnMut(TScalarField) -> Result<U, E>,
    ) -> Result<ScalarFieldSelection<U>, E> {
        Ok(ScalarFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            field: map(self.field)?,
            unwraps: self.unwraps,
            arguments: self.arguments,
            normalization_alias: self.normalization_alias,
        })
    }

    pub fn name_or_alias(&self) -> WithSpan<FieldNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(FieldNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(FieldNameOrAlias::from))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct LinkedFieldSelection<
    TScalarField: ValidScalarFieldType,
    TLinkedField: ValidLinkedFieldType,
> {
    pub name: WithSpan<LinkedFieldName>,
    pub reader_alias: Option<WithSpan<LinkedFieldAlias>>,
    pub normalization_alias: Option<WithSpan<LinkedFieldAlias>>,
    pub field: TLinkedField,
    pub selection_set_and_unwraps: SelectionSetAndUnwraps<TScalarField, TLinkedField>,
    pub arguments: Vec<WithSpan<SelectionFieldArgument>>,
}

impl<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType>
    LinkedFieldSelection<TScalarField, TLinkedField>
{
    pub fn name_or_alias(&self) -> WithSpan<FieldNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(FieldNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(FieldNameOrAlias::from))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Unwrap {
    ActualUnwrap,
    SkippedUnwrap,
    // FakeUnwrap?
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct SelectionFieldArgument {
    pub name: WithSpan<FieldArgumentName>,
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum NonConstantValue {
    Variable(VariableName),
}

impl fmt::Display for NonConstantValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonConstantValue::Variable(name) => write!(f, "${}", name),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct VariableDefinition<TValue: ValidTypeAnnotationInnerType> {
    pub name: WithSpan<VariableName>,
    pub type_: TypeAnnotation<TValue>,
    // pub default_value: Option<WithSpan<ConstantValue>>,
}

impl<TValue: ValidTypeAnnotationInnerType> VariableDefinition<TValue> {
    pub fn map<TNewValue: ValidTypeAnnotationInnerType>(
        self,
        map: &mut impl FnMut(TValue) -> TNewValue,
    ) -> VariableDefinition<TNewValue> {
        VariableDefinition {
            name: self.name,
            type_: self.type_.map(map),
        }
    }

    pub fn and_then<TNewValue: ValidTypeAnnotationInnerType, E>(
        self,
        map: &mut impl FnMut(TValue) -> Result<TNewValue, E>,
    ) -> Result<VariableDefinition<TNewValue>, E> {
        Ok(VariableDefinition {
            name: self.name,
            type_: self.type_.and_then(map)?,
        })
    }
}
