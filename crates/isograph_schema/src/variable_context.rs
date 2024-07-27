use intern::Lookup;
use std::collections::HashMap;

use common_lang_types::{VariableName, WithLocation, WithSpan};
use isograph_lang_types::{ConstantValue, NonConstantValue, SelectionFieldArgument};

use crate::{ClientField, ValidatedVariableDefinition};

#[derive(Debug)]
pub struct VariableContext(pub HashMap<VariableName, NonConstantValue>);

impl VariableContext {
    pub fn child_variable_context(
        &self,
        selection_arguments: &[WithLocation<SelectionFieldArgument>],
        child_variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
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
                        if let Some(default_value) = variable_definition.item.default_value.as_ref()
                        {
                            return (variable_name, default_value.clone().item.into());
                        } else {
                            panic!(
                                "Expected variable definition `${variable_name}` \
                                to have a default value. \
                                This should have been validated already. \
                                This is indicative of a bug in Isograph."
                            );
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
                            .clone()
                            .into(),
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
        TClientFieldVariableDefinitionAssociatedData,
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
