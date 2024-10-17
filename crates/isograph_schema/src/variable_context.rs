use intern::Lookup;
use std::{collections::HashMap, fmt::Debug};

use common_lang_types::{SelectableFieldName, VariableName, WithLocation, WithSpan};
use isograph_lang_types::{
    ArgumentKeyAndValue, ConstantValue, NonConstantValue, SelectionFieldArgument,
};

use crate::{
    ClientField, NameAndArguments, ValidatedIsographSelectionVariant, ValidatedVariableDefinition,
};

#[derive(Debug)]
pub struct VariableContext(pub HashMap<VariableName, NonConstantValue>);

impl VariableContext {
    pub fn child_variable_context(
        &self,
        selection_arguments: &[WithLocation<SelectionFieldArgument>],
        child_variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
        selection_variant: &ValidatedIsographSelectionVariant,
    ) -> Self {
        // We need to take a parent context ({$id: NonConstantValue1 }), the argument parameters ({blah: $id}),
        // and the child variable definitions ({ $blah: Option<NonConstantValue2> }) and create a new child
        // context ({ $blah: NonConstantValue1 }), which, for each child variable def:
        // - if a matching argument exists:
        //   - contains the argument value if it is not a variable, or
        //   - if it is a variable, contains the parent context's value for that variable, which
        //   must exist
        // - or if a matching argument does not exist, contains the default value for that variable,
        //   which must exist.
        // Panicking is okay, because we have previously validated this. However, we should consider
        // how to make this not panic.
        let variable_context = child_variable_definitions
            .iter()
            .map(|variable_definition| {
                let variable_name = variable_definition.item.name.item;

                let matching_arg = match selection_arguments.iter().find(|s| {
                    // TODO don't require a lookup
                    s.item.name.item.lookup() == variable_name.lookup()
                }) {
                    Some(arg) => arg,
                    None => {
                        if matches!(
                            selection_variant,
                            ValidatedIsographSelectionVariant::Loadable(_)
                        ) {
                            // If this field was selected loadably, missing arguments are allowed.
                            // These missing arguments become variables that are provided at
                            // runtime. If they are missing at runtime, they will fall back to
                            // their default value (if any is present.) (Or at least, that's the
                            // intended behavior.)
                            return (variable_name, NonConstantValue::Variable(variable_name));
                        } else if let Some(default_value) =
                            variable_definition.item.default_value.as_ref()
                        {
                            return (variable_name, default_value.clone().item.into());
                        } else {
                            // TODO this is only valid if the arg is nullable, which we should
                            // validate
                            return (variable_name, NonConstantValue::Null);
                        }
                    }
                };

                let child_value =
                    // TODO avoid cloning
                    match ConstantValue::try_from(matching_arg.item.clone().value.item) {
                        Ok(_) => matching_arg.item.value.item.clone(),
                        Err(e) => self
                            .0
                            .get(&e)
                            .expect(
                                "Parent context has missing variable. \
                                This should have been validated already. \
                                This is indicative of a bug in Isograph.",
                            )
                            .clone(),
                    };

                (variable_name, child_value)
            })
            .collect();
        VariableContext(variable_context)
    }
}

impl<
        TClientFieldSelectionScalarFieldAssociatedData,
        TClientFieldSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    >
    ClientField<
        TClientFieldSelectionScalarFieldAssociatedData,
        TClientFieldSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData,
    >
{
    pub fn initial_variable_context(&self) -> VariableContext {
        // This is used in two places: before we generate a merged selection set for an
        // entrypoint and when generating reader ASTs.
        //
        // For entrypoints:
        // This seems fishy. We should be taking note of the defualt values, somehow. However,
        // the variables that are provided/missing are done so at runtime, so this is not
        // really possible.
        //
        // The replacement of missing values with default values is done by the GraphQL server,
        // so in practice this is probably fine.
        //
        // But it is odd that variables and default values behave differently for fields that
        // act as entrypoints vs. fields that are used within the entrypoint.
        //
        // For reader ASTs:
        // This makes sense, but seems somewhat superfluous. Perhaps we can refactor code such
        // that we do not need to call this.
        let variable_context = self
            .variable_definitions
            .iter()
            .map(|variable_definition| {
                (
                    variable_definition.item.name.item,
                    NonConstantValue::Variable(variable_definition.item.name.item),
                )
            })
            .collect();
        VariableContext(variable_context)
    }
}

fn transform_selection_field_argument_into_merged_arg_with_child_context(
    arg: ArgumentKeyAndValue,
    variable_context: &VariableContext,
) -> ArgumentKeyAndValue {
    if let NonConstantValue::Variable(used_variable_name) = arg.value {
        // Look up the variable in the variables in context, and use that value
        //
        // This will give us the *actual value* that we need for the merged selection set.
        let value = variable_context.0.get(&used_variable_name);

        return match value {
            Some(value) => ArgumentKeyAndValue {
                key: arg.key,
                value: value.clone(),
            },
            None => {
                // There is no variable. The value is missing! It had better be optional.
                // TODO we should validate that
                ArgumentKeyAndValue {
                    key: arg.key,
                    value: NonConstantValue::Null,
                }
            }
        };
    }

    arg
}

pub fn transform_arguments_with_child_context(
    arguments: impl Iterator<Item = ArgumentKeyAndValue>,
    transformed_child_variable_context: &VariableContext,
) -> Vec<ArgumentKeyAndValue> {
    arguments
        .map(|arg| {
            transform_selection_field_argument_into_merged_arg_with_child_context(
                arg,
                transformed_child_variable_context,
            )
        })
        .collect::<Vec<_>>()
}

pub fn transform_name_and_arguments_with_child_variable_context(
    name_and_arguments: NameAndArguments,
    transformed_child_variable_context: &VariableContext,
) -> NameAndArguments {
    NameAndArguments {
        name: name_and_arguments.name,
        arguments: transform_arguments_with_child_context(
            name_and_arguments.arguments.into_iter(),
            transformed_child_variable_context,
        ),
    }
}

pub fn create_transformed_name_and_arguments(
    name: SelectableFieldName,
    arguments: &[WithLocation<SelectionFieldArgument>],
    variable_context: &VariableContext,
) -> NameAndArguments {
    NameAndArguments {
        name,
        arguments: transform_arguments_with_child_context(
            arguments
                .iter()
                .map(|selection_field_argument| selection_field_argument.item.into_key_and_value()),
            variable_context,
        ),
    }
}
