use common_lang_types::{
    ConstExportName, FieldArgumentName, FieldNameOrAlias, HasName, IsographDirectiveName,
    LinkedFieldAlias, LinkedFieldName, ResolverDefinitionPath, ScalarFieldAlias, ScalarFieldName,
    SelectableFieldName, UnvalidatedTypeName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::TypeAnnotation;

pub type UnvalidatedSelection = Selection<
    // <UnvalidatedSchemaState as SchemaValidationState>::ResolverSelectionScalarFieldAssociatedData,
    (),
    // <UnvalidatedSchemaState as SchemaValidationState>::ResolverSelectionLinkedFieldAssociatedData,
    (),
>;
pub type UnvalidatedScalarFieldSelection = ScalarFieldSelection<
    // <UnvalidatedSchemaState as SchemaValidationState>::ResolverSelectionScalarFieldAssociatedData,
    (),
>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ResolverDeclaration {
    pub const_export_name: ConstExportName,
    pub parent_type: WithSpan<UnvalidatedTypeName>,
    pub resolver_field_name: WithSpan<ScalarFieldName>,
    pub selection_set_and_unwraps:
        Option<(Vec<WithSpan<UnvalidatedSelection>>, Vec<WithSpan<Unwrap>>)>,
    pub directives: Vec<WithSpan<FragmentDirectiveUsage>>,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,
    pub resolver_definition_path: ResolverDefinitionPath,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
/// Ugly name, but at least it makes clear this isn't a schema directive.
pub struct FragmentDirectiveUsage {
    pub name: WithSpan<IsographDirectiveName>,
    // TODO arguments and such
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
        map_scalar_field: &mut impl FnMut(
            ScalarFieldSelection<TScalarField>,
        ) -> Result<ScalarFieldSelection<TNewScalarField>, E>,
        map_linked_field: &mut impl FnMut(
            LinkedFieldSelection<TScalarField, TLinkedField>,
        ) -> Result<
            LinkedFieldSelection<TNewScalarField, TNewLinkedField>,
            E,
        >,
    ) -> Result<ServerFieldSelection<TNewScalarField, TNewLinkedField>, E> {
        match self {
            ServerFieldSelection::ScalarField(s) => {
                Ok(ServerFieldSelection::ScalarField(map_scalar_field(s)?))
            }
            ServerFieldSelection::LinkedField(l) => {
                Ok(ServerFieldSelection::LinkedField(map_linked_field(l)?))
            }
        }
    }

    pub fn name_or_alias(&self) -> WithLocation<FieldNameOrAlias> {
        match self {
            ServerFieldSelection::ScalarField(scalar_field) => scalar_field.name_or_alias(),
            ServerFieldSelection::LinkedField(linked_field) => linked_field.name_or_alias(),
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
    pub normalization_alias: Option<WithLocation<ScalarFieldAlias>>,
    pub associated_data: TScalarField,
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
}

impl<TScalarField> ScalarFieldSelection<TScalarField> {
    pub fn map<U>(self, map: &mut impl FnMut(TScalarField) -> U) -> ScalarFieldSelection<U> {
        ScalarFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            associated_data: map(self.associated_data),
            unwraps: self.unwraps,
            arguments: self.arguments,
            normalization_alias: self.normalization_alias,
        }
    }

    pub fn and_then<U, E>(
        self,
        map: &mut impl FnMut(TScalarField) -> Result<U, E>,
    ) -> Result<ScalarFieldSelection<U>, E> {
        Ok(ScalarFieldSelection {
            name: self.name,
            reader_alias: self.reader_alias,
            associated_data: map(self.associated_data)?,
            unwraps: self.unwraps,
            arguments: self.arguments,
            normalization_alias: self.normalization_alias,
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
    pub reader_alias: Option<WithLocation<LinkedFieldAlias>>,
    pub normalization_alias: Option<WithLocation<LinkedFieldAlias>>,
    pub associated_data: TLinkedField,
    pub selection_set: Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
}

impl<TScalarField, TLinkedField> LinkedFieldSelection<TScalarField, TLinkedField> {
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
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum NonConstantValue {
    Variable(VariableName),
    Integer(u64),
}

impl NonConstantValue {
    pub fn reachable_variables(&self) -> Vec<VariableName> {
        match self {
            NonConstantValue::Variable(name) => vec![*name],
            NonConstantValue::Integer(_) => vec![],
        }
    }

    pub fn to_alias_str_chunk(&self) -> String {
        match self {
            NonConstantValue::Variable(name) => format!("v_{}", name),
            // l for literal, i.e. this is shared with others
            NonConstantValue::Integer(int_value) => format!("l_{}", int_value),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct VariableDefinition<TValue> {
    pub name: WithLocation<VariableName>,
    pub type_: TypeAnnotation<TValue>,
    // pub default_value: Option<WithLocation<ConstantValue>>,
}

impl<TValue> VariableDefinition<TValue> {
    pub fn map<TNewValue>(
        self,
        map: &mut impl FnMut(TValue) -> TNewValue,
    ) -> VariableDefinition<TNewValue> {
        VariableDefinition {
            name: self.name,
            type_: self.type_.map(map),
        }
    }

    pub fn and_then<TNewValue, E>(
        self,
        map: &mut impl FnMut(TValue) -> Result<TNewValue, E>,
    ) -> Result<VariableDefinition<TNewValue>, E> {
        Ok(VariableDefinition {
            name: self.name,
            type_: self.type_.and_then(map)?,
        })
    }
}
