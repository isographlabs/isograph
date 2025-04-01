use common_lang_types::{
    EnumLiteralValue, GraphQLScalarTypeName, IsographObjectTypeName, SelectableName,
    UnvalidatedTypeName, ValueKeyName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLTypeAnnotation, NameValuePair};
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, DefinitionLocation, LinkedFieldSelection,
    LoadableDirectiveParameters, NonConstantValue, ObjectSelectionDirectiveSet,
    ScalarFieldSelection, ScalarSelectionDirectiveSet, SelectionFieldArgument, SelectionType,
    SelectionTypeContainingSelections, ServerEntityId, ServerObjectId, ServerScalarSelectableId,
    VariableDefinition,
};

use thiserror::Error;

use crate::{
    ClientField, ClientFieldVariant, ClientPointer, ImperativelyLoadedFieldVariant, OutputFormat,
    Schema, ServerScalarSelectable, UseRefetchFieldRefetchStrategy,
};

pub type ValidatedSchemaServerField<TOutputFormat> = ServerScalarSelectable<TOutputFormat>;

pub type ValidatedSelection = SelectionTypeContainingSelections<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;

pub type ValidatedLinkedFieldSelection = LinkedFieldSelection<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;
pub type ValidatedScalarFieldSelection =
    ScalarFieldSelection<ValidatedScalarSelectionAssociatedData>;

pub type ValidatedVariableDefinition = VariableDefinition<ServerEntityId>;

pub type ValidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;

/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedFieldDefinitionLocation =
    DefinitionLocation<ServerScalarSelectableId, ClientFieldId>;

#[derive(Debug, Clone)]
pub struct ValidatedLinkedFieldAssociatedData {
    pub parent_object_id: ServerObjectId,
    pub field_id: DefinitionLocation<ServerScalarSelectableId, ClientPointerId>,
    pub selection_variant: ObjectSelectionDirectiveSet,
    /// Some if the (destination?) object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,
}

// TODO this should encode whether the scalar selection points to a
// client field or to a server scalar
#[derive(Debug, Clone)]
pub struct ValidatedScalarSelectionAssociatedData {
    pub location: ValidatedFieldDefinitionLocation,
    pub selection_variant: ScalarSelectionDirectiveSet,
}

pub type MissingArguments = Vec<ValidatedVariableDefinition>;

pub type ValidatedSelectionType<'a, TOutputFormat> =
    SelectionType<&'a ClientField<TOutputFormat>, &'a ClientPointer<TOutputFormat>>;

pub type ValidatedSchema<TOutputFormat> = Schema<TOutputFormat>;

pub fn get_all_errors_or_all_ok<T, E>(
    items: impl Iterator<Item = Result<T, Vec<E>>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut oks = vec![];
    let mut errors = vec![];

    for item in items {
        match item {
            Ok(ok) => oks.push(ok),
            Err(e) => errors.extend(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

pub enum Loadability<'a> {
    LoadablySelectedField(&'a LoadableDirectiveParameters),
    ImperativelyLoadedField(&'a ImperativelyLoadedFieldVariant),
}

/// Why do we do this? Because how we handle a field is determined by both the
/// the field defition (e.g. exposed fields can only be fetched imperatively)
/// and the selection (i.e. we can also take non-imperative fields and make them
/// imperative.)
///
/// The eventual plan is to clean this model up. Instead, imperative fields will
/// need to be explicitly selected loadably. If they are not, they will be fetched
/// as an immediate follow-up request. Once we do this, there will always be one
/// source of truth for whether a field is fetched imperatively: the presence of the
/// @loadable directive.
pub fn categorize_field_loadability<'a, TOutputFormat: OutputFormat>(
    client_field: &'a ClientField<TOutputFormat>,
    selection_variant: &'a ScalarSelectionDirectiveSet,
) -> Option<Loadability<'a>> {
    match &client_field.variant {
        ClientFieldVariant::Link => None,
        ClientFieldVariant::UserWritten(_) => match selection_variant {
            ScalarSelectionDirectiveSet::None(_) => None,
            ScalarSelectionDirectiveSet::Updatable(_) => None,
            ScalarSelectionDirectiveSet::Loadable(l) => {
                Some(Loadability::LoadablySelectedField(&l.loadable))
            }
        },
        ClientFieldVariant::ImperativelyLoadedField(i) => {
            Some(Loadability::ImperativelyLoadedField(i))
        }
    }
}

pub fn get_provided_arguments<'a>(
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> Vec<ValidatedVariableDefinition> {
    argument_definitions
        .filter_map(|definition| {
            let user_has_supplied_argument = arguments
                .iter()
                .any(|arg| definition.name.item == arg.item.name.item);
            if user_has_supplied_argument {
                Some(definition.clone())
            } else {
                None
            }
        })
        .collect()
}

pub type ValidateSchemaResult<T> = Result<T, WithLocation<ValidateSchemaError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ValidateSchemaError {
    #[error("Expected input of type {expected_type}, found variable {variable_name} of type {variable_type}")]
    ExpectedTypeFoundVariable {
        expected_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        variable_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        variable_name: VariableName,
    },

    #[error("Expected input of type {expected}, found {actual} scalar literal")]
    ExpectedTypeFoundScalar {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        actual: GraphQLScalarTypeName,
    },

    #[error("Expected input of type {expected}, found object literal")]
    ExpectedTypeFoundObject {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    },

    #[error("Expected input of type {expected}, found list literal")]
    ExpectedTypeFoundList {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    },

    #[error("Expected non null input of type {expected}, found null")]
    ExpectedNonNullTypeFoundNull {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    },

    #[error("Expected input of type {expected}, found {actual} enum literal")]
    ExpectedTypeFoundEnum {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        actual: EnumLiteralValue,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected, but that \
        field does not exist on `{field_parent_type_name}`"
    )]
    SelectionTypeSelectionFieldDoesNotExist {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a scalar, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    SelectionTypeSelectionFieldIsNotScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field's type is `{target_type_name}`, which is a scalar."
    )]
    SelectionTypeSelectionFieldIsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, the \
        pointer `{field_parent_type_name}.{field_name}` is selected as a scalar. \
        However, client pointers can only be selected as linked fields."
    )]
    SelectionTypeSelectionClientPointerSelectedAsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error("`{server_field_name}` is a server field, and cannot be selected with `@loadable`")]
    ServerFieldCannotBeSelectedLoadably { server_field_name: SelectableName },

    #[error(
        "This field has missing arguments: {0}",
        missing_arguments.iter().map(|arg| format!("${}", arg.name.item)).collect::<Vec<_>>().join(", ")
    )]
    MissingArguments { missing_arguments: MissingArguments },

    #[error(
        "This object has missing fields: {0}",
        missing_fields_names.iter().map(|field_name| format!("${}", field_name)).collect::<Vec<_>>().join(", ")
    )]
    MissingFields {
        missing_fields_names: Vec<SelectableName>,
    },

    #[error(
        "This field has extra arguments: {0}",
        extra_arguments.iter().map(|arg| format!("{}", arg.item.name)).collect::<Vec<_>>().join(", ")
    )]
    ExtraneousArgument {
        extra_arguments: Vec<WithLocation<SelectionFieldArgument>>,
    },

    #[error(
        "This object has extra fields: {0}",
        extra_fields.iter().map(|field| format!("{}", field.name.item)).collect::<Vec<_>>().join(", ")
    )]
    ExtraneousFields {
        extra_fields: Vec<NameValuePair<ValueKeyName, NonConstantValue>>,
    },

    #[error(
        "The field `{type_name}.{field_name}` has unused variables: {0}",
        unused_variables.iter().map(|variable| format!("${}", variable.item.name.item)).collect::<Vec<_>>().join(", ")
    )]
    UnusedVariables {
        unused_variables: Vec<WithSpan<ValidatedVariableDefinition>>,
        type_name: IsographObjectTypeName,
        field_name: SelectableName,
    },

    #[error("This variable is not defined: ${undefined_variable}")]
    UsedUndefinedVariable { undefined_variable: VariableName },
}
