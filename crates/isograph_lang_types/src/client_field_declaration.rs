use std::fmt::Debug;

use common_lang_types::{
    ConstExportName, DescriptionValue, EnumLiteralValue, FieldArgumentName, FieldNameOrAlias,
    FilePath, HasName, LinkedFieldAlias, LinkedFieldName, ScalarFieldAlias, ScalarFieldName,
    SelectableFieldName, StringLiteralValue, UnvalidatedTypeName, ValueKeyName, VariableName,
    WithLocation, WithSpan,
};
use graphql_lang_types::{FloatValue, GraphQLTypeAnnotation, NameValuePair};
use serde::Deserialize;

use crate::IsographFieldDirective;

pub type UnvalidatedSelectionWithUnvalidatedDirectives = Selection<(), ()>;

pub type UnvalidatedSelection = Selection<
    // <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    IsographSelectionVariant,
    // <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
    IsographSelectionVariant,
>;
pub type UnvalidatedScalarFieldSelection = ScalarFieldSelection<
    // <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    IsographSelectionVariant,
>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ClientFieldDeclaration<TScalarField, TLinkedField> {
    pub const_export_name: ConstExportName,
    pub parent_type: WithSpan<UnvalidatedTypeName>,
    pub client_field_name: WithSpan<ScalarFieldName>,
    pub description: Option<WithSpan<DescriptionValue>>,
    pub selection_set: Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub directives: Vec<WithSpan<IsographFieldDirective>>,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,
    pub definition_path: FilePath,

    // TODO consider making these behind a cfg flag, since they're only used
    // by the LSP
    pub field_keyword: WithSpan<()>,
    pub dot: WithSpan<()>,
}

pub type ClientFieldDeclarationWithUnvalidatedDirectives = ClientFieldDeclaration<(), ()>;
pub type ClientFieldDeclarationWithValidatedDirectives =
    ClientFieldDeclaration<IsographSelectionVariant, IsographSelectionVariant>;

// TODO we should not have an enum, but instead a struct with fields lazy_load_artifact_info
// and loadable_info or something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsographSelectionVariant {
    Regular,
    Loadable(LoadableDirectiveParameters),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadableDirectiveParameters {
    #[serde(default)]
    complete_selection_set: bool,
    #[serde(default)]
    pub lazy_load_artifact: bool,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Selection<TScalarField, TLinkedField> {
    ServerField(ServerFieldSelection<TScalarField, TLinkedField>),
    // FieldGroup(FieldGroupSelection),
}

impl<TScalarField, TLinkedField> Selection<TScalarField, TLinkedField> {
    pub fn map<TNewScalarField, TNewLinkedField>(
        self,
        map: &mut impl FnMut(
            ServerFieldSelection<TScalarField, TLinkedField>,
        ) -> ServerFieldSelection<TNewScalarField, TNewLinkedField>,
    ) -> Selection<TNewScalarField, TNewLinkedField> {
        match self {
            Selection::ServerField(field_selection) => Selection::ServerField(map(field_selection)),
        }
    }

    pub fn and_then<TNewScalarField, TNewLinkedField, E>(
        self,
        map: &mut impl FnMut(
            ServerFieldSelection<TScalarField, TLinkedField>,
        )
            -> Result<ServerFieldSelection<TNewScalarField, TNewLinkedField>, E>,
    ) -> Result<Selection<TNewScalarField, TNewLinkedField>, E> {
        match self {
            Selection::ServerField(field_selection) => {
                Ok(Selection::ServerField(map(field_selection)?))
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum ServerFieldSelection<TScalarField, TLinkedField> {
    ScalarField(ScalarFieldSelection<TScalarField>),
    LinkedField(LinkedFieldSelection<TScalarField, TLinkedField>),
}

impl<TScalarField, TLinkedField> ServerFieldSelection<TScalarField, TLinkedField> {
    pub fn map<TNewScalarField, TNewLinkedField>(
        self,
        map_scalar_field: &mut impl FnMut(
            ScalarFieldSelection<TScalarField>,
        ) -> ScalarFieldSelection<TNewScalarField>,
        map_linked_field: &mut impl FnMut(
            LinkedFieldSelection<TScalarField, TLinkedField>,
        )
            -> LinkedFieldSelection<TNewScalarField, TNewLinkedField>,
    ) -> ServerFieldSelection<TNewScalarField, TNewLinkedField> {
        match self {
            ServerFieldSelection::ScalarField(s) => {
                ServerFieldSelection::ScalarField(map_scalar_field(s))
            }
            ServerFieldSelection::LinkedField(l) => {
                ServerFieldSelection::LinkedField(map_linked_field(l))
            }
        }
    }

    pub fn and_then<TNewScalarField, TNewLinkedField, E>(
        self,
        and_then_scalar_field: &mut impl FnMut(
            ScalarFieldSelection<TScalarField>,
        )
            -> Result<ScalarFieldSelection<TNewScalarField>, E>,
        and_then_linked_field: &mut impl FnMut(
            LinkedFieldSelection<TScalarField, TLinkedField>,
        ) -> Result<
            LinkedFieldSelection<TNewScalarField, TNewLinkedField>,
            E,
        >,
    ) -> Result<ServerFieldSelection<TNewScalarField, TNewLinkedField>, E> {
        match self {
            ServerFieldSelection::ScalarField(s) => {
                Ok(ServerFieldSelection::ScalarField(and_then_scalar_field(s)?))
            }
            ServerFieldSelection::LinkedField(l) => {
                Ok(ServerFieldSelection::LinkedField(and_then_linked_field(l)?))
            }
        }
    }

    pub fn name_or_alias(&self) -> WithLocation<FieldNameOrAlias> {
        match self {
            ServerFieldSelection::ScalarField(scalar_field) => scalar_field.name_or_alias(),
            ServerFieldSelection::LinkedField(linked_field) => linked_field.name_or_alias(),
        }
    }

    pub fn variables<'a>(&'a self) -> impl Iterator<Item = VariableName> + 'a {
        let get_variable = |x: &'a WithLocation<SelectionFieldArgument>| match x.item.value.item {
            NonConstantValue::Variable(v) => Some(v),
            _ => None,
        };
        match self {
            ServerFieldSelection::ScalarField(scalar_field) => {
                scalar_field.arguments.iter().flat_map(get_variable)
            }
            ServerFieldSelection::LinkedField(linked_field) => {
                linked_field.arguments.iter().flat_map(get_variable)
            }
        }
    }
}

impl<TScalarField, TLinkedField> HasName for ServerFieldSelection<TScalarField, TLinkedField> {
    type Name = SelectableFieldName;
    fn name(&self) -> Self::Name {
        match self {
            ServerFieldSelection::ScalarField(s) => s.name.item.into(),
            ServerFieldSelection::LinkedField(l) => l.name.item.into(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ScalarFieldSelection<TScalarField> {
    pub name: WithLocation<ScalarFieldName>,
    pub reader_alias: Option<WithLocation<ScalarFieldAlias>>,
    pub associated_data: TScalarField,
    // TODO remove until we are ready for this :/
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
    pub directives: Vec<WithSpan<IsographFieldDirective>>,
}

impl<TScalarField> ScalarFieldSelection<TScalarField> {
    pub fn map<U>(self, map: &impl Fn(TScalarField) -> U) -> ScalarFieldSelection<U> {
        ScalarFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            associated_data: map(self.associated_data),
            unwraps: self.unwraps,
            arguments: self.arguments,
            directives: self.directives,
        }
    }

    pub fn and_then<U, E>(
        self,
        map: &impl Fn(TScalarField) -> Result<U, E>,
    ) -> Result<ScalarFieldSelection<U>, E> {
        Ok(ScalarFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            associated_data: map(self.associated_data)?,
            unwraps: self.unwraps,
            arguments: self.arguments,
            directives: self.directives,
        })
    }

    pub fn name_or_alias(&self) -> WithLocation<FieldNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(FieldNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(FieldNameOrAlias::from))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct LinkedFieldSelection<TScalarField, TLinkedField> {
    pub name: WithLocation<LinkedFieldName>,
    // pub alias
    pub reader_alias: Option<WithLocation<LinkedFieldAlias>>,
    pub associated_data: TLinkedField,
    pub selection_set: Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
    pub directives: Vec<WithSpan<IsographFieldDirective>>,
}

impl<TScalarField, TLinkedField> LinkedFieldSelection<TScalarField, TLinkedField> {
    pub fn map<U, V>(
        self,
        map_scalar: &impl Fn(TScalarField) -> U,
        map_linked: &impl Fn(TLinkedField) -> V,
    ) -> LinkedFieldSelection<U, V> {
        LinkedFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            associated_data: map_linked(self.associated_data),
            selection_set: self
                .selection_set
                .into_iter()
                .map(|with_span| {
                    with_span.map(|selection| {
                        selection.map(&mut |server_field_selection| {
                            server_field_selection.map(
                                &mut |scalar_field_selection| {
                                    scalar_field_selection.map(map_scalar)
                                },
                                &mut |linked_field_selection| {
                                    linked_field_selection.map(map_scalar, map_linked)
                                },
                            )
                        })
                    })
                })
                .collect(),
            unwraps: self.unwraps,
            arguments: self.arguments,
            directives: self.directives,
        }
    }

    pub fn name_or_alias(&self) -> WithLocation<FieldNameOrAlias> {
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

impl SelectionFieldArgument {
    /// A function called on each SelectionFieldArgument when
    /// generating queries. This must be kept in sync with @isograph/react
    pub fn to_alias_str_chunk(&self) -> String {
        format!(
            "{}___{}",
            self.name.item,
            self.value.item.to_alias_str_chunk()
        )
    }

    pub fn into_key_and_value(&self) -> ArgumentKeyAndValue {
        ArgumentKeyAndValue {
            key: self.name.item,
            value: self.value.item.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArgumentKeyAndValue {
    pub key: FieldArgumentName,
    pub value: NonConstantValue,
}

impl ArgumentKeyAndValue {
    pub fn to_alias_str_chunk(&self) -> String {
        format!("{}___{}", self.key, self.value.to_alias_str_chunk())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum NonConstantValue {
    Variable(VariableName),
    Integer(i64),
    Boolean(bool),
    String(StringLiteralValue),
    Float(FloatValue),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithLocation<NonConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, NonConstantValue>>),
}

impl NonConstantValue {
    pub fn reachable_variables(&self) -> Vec<VariableName> {
        match self {
            NonConstantValue::Variable(name) => vec![*name],
            NonConstantValue::Object(object) => {
                return object
                    .iter()
                    .flat_map(|pair| pair.value.item.reachable_variables())
                    .collect();
            }
            NonConstantValue::List(list) => {
                return list
                    .iter()
                    .flat_map(|element| element.item.reachable_variables())
                    .collect();
            }
            _ => vec![],
        }
    }

    pub fn to_alias_str_chunk(&self) -> String {
        match self {
            NonConstantValue::Variable(name) => format!("v_{}", name),
            // l for literal, i.e. this is shared with others
            NonConstantValue::Integer(int_value) => format!("l_{}", int_value),
            NonConstantValue::Boolean(bool) => format!("l_{}", bool),
            // N.B. This clearly isn't correct, the string can (for example) include
            // spaces, which would break things.
            // TODO get a solution or validate
            NonConstantValue::String(string) => format!("s_{}", string),
            // Also not correct
            NonConstantValue::Float(f) => format!("l_{}", f.as_float()),
            NonConstantValue::Null => "l_null".to_string(),
            NonConstantValue::Enum(e) => format!("e_{e}"),
            NonConstantValue::List(_) => panic!("Lists are not supported here"),
            NonConstantValue::Object(_) => panic!("Objects not supported here"),
        }
    }
}

impl From<ConstantValue> for NonConstantValue {
    fn from(value: ConstantValue) -> Self {
        match value {
            ConstantValue::Integer(i) => NonConstantValue::Integer(i),
            ConstantValue::Boolean(value) => NonConstantValue::Boolean(value),
            ConstantValue::String(value) => NonConstantValue::String(value),
            ConstantValue::Float(value) => NonConstantValue::Float(value),
            ConstantValue::Null => NonConstantValue::Null,
            ConstantValue::Enum(value) => NonConstantValue::Enum(value),
            ConstantValue::List(value) => NonConstantValue::List(
                value
                    .into_iter()
                    .map(|with_location| with_location.map(NonConstantValue::from))
                    .collect(),
            ),
            ConstantValue::Object(value) => NonConstantValue::Object(
                value
                    .into_iter()
                    .map(|name_value_pair| NameValuePair {
                        name: name_value_pair.name,
                        value: name_value_pair.value.map(NonConstantValue::from),
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum ConstantValue {
    Integer(i64),
    Boolean(bool),
    String(StringLiteralValue),
    Float(FloatValue),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithLocation<ConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, ConstantValue>>),
}

impl TryFrom<NonConstantValue> for ConstantValue {
    type Error = VariableName;

    fn try_from(value: NonConstantValue) -> Result<Self, Self::Error> {
        match value {
            NonConstantValue::Variable(variable_name) => Err(variable_name),
            NonConstantValue::Integer(i) => Ok(ConstantValue::Integer(i)),
            NonConstantValue::Boolean(b) => Ok(ConstantValue::Boolean(b)),
            NonConstantValue::String(s) => Ok(ConstantValue::String(s)),
            NonConstantValue::Float(f) => Ok(ConstantValue::Float(f)),
            NonConstantValue::Null => Ok(ConstantValue::Null),
            NonConstantValue::Enum(e) => Ok(ConstantValue::Enum(e)),
            NonConstantValue::List(l) => {
                let converted_list = l
                    .into_iter()
                    .map(|x| {
                        Ok::<_, Self::Error>(WithLocation::new(x.item.try_into()?, x.location))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ConstantValue::List(converted_list))
            }
            NonConstantValue::Object(o) => {
                let converted_object = o
                    .into_iter()
                    .map(|name_value_pair| {
                        Ok::<_, Self::Error>(NameValuePair {
                            name: name_value_pair.name,
                            value: WithLocation::new(
                                name_value_pair.value.item.try_into()?,
                                name_value_pair.value.location,
                            ),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ConstantValue::Object(converted_object))
            }
        }
    }
}

impl ConstantValue {
    pub fn print_to_string(&self) -> String {
        match self {
            ConstantValue::Integer(i) => i.to_string(),
            ConstantValue::Boolean(b) => b.to_string(),
            ConstantValue::String(s) => format!("\"{s}\""),
            ConstantValue::Float(f) => f.as_float().to_string(),
            ConstantValue::Null => "null".to_string(),
            ConstantValue::Enum(e) => e.to_string(),
            ConstantValue::List(l) => {
                let inner = l
                    .iter()
                    .map(|value| value.item.print_to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            ConstantValue::Object(o) => {
                let inner = o
                    .iter()
                    .map(|key_value| {
                        format!(
                            "{}: {}",
                            key_value.name.item,
                            key_value.value.item.print_to_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct VariableDefinition<TValue: Ord + Debug> {
    pub name: WithLocation<VariableName>,
    pub type_: GraphQLTypeAnnotation<TValue>,
    pub default_value: Option<WithLocation<ConstantValue>>,
}

impl<TValue: Ord + Debug> VariableDefinition<TValue> {
    pub fn map<TNewValue: Ord + Debug>(
        self,
        map: &mut impl FnMut(TValue) -> TNewValue,
    ) -> VariableDefinition<TNewValue> {
        VariableDefinition {
            name: self.name,
            type_: self.type_.map(map),
            default_value: self.default_value,
        }
    }

    pub fn and_then<TNewValue: Ord + Debug, E>(
        self,
        map: &mut impl FnMut(TValue) -> Result<TNewValue, E>,
    ) -> Result<VariableDefinition<TNewValue>, E> {
        Ok(VariableDefinition {
            name: self.name,
            type_: self.type_.and_then(map)?,
            default_value: self.default_value,
        })
    }
}
