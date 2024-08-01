use common_lang_types::{HasName, QueryOperationName, UnvalidatedTypeName};
use graphql_lang_types::GraphQLTypeAnnotation;
use isograph_lang_types::{ArgumentKeyAndValue, NonConstantValue};
use isograph_schema::{
    MergedSelectionMap, MergedServerSelection, RootOperationName, ValidatedSchema,
    ValidatedVariableDefinition,
};

use crate::generate_artifacts::QueryText;

pub(crate) fn generate_query_text<'a>(
    query_name: QueryOperationName,
    schema: &ValidatedSchema,
    selection_map: &MergedSelectionMap,
    query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    root_operation_name: &RootOperationName,
) -> QueryText {
    let mut query_text = String::new();

    let variable_text = write_variables_to_string(schema, query_variables);

    query_text.push_str(&format!(
        "{} {} {} {{\\\n",
        root_operation_name.0, query_name, variable_text
    ));
    write_selections_for_query_text(&mut query_text, selection_map.values(), 1);
    query_text.push('}');
    QueryText(query_text)
}

fn write_variables_to_string<'a>(
    schema: &ValidatedSchema,
    variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
) -> String {
    let mut empty = true;
    let mut first = true;
    let mut variable_text = String::new();
    variable_text.push('(');
    for variable in variables {
        empty = false;
        if !first {
            variable_text.push_str(", ");
        } else {
            first = false;
        }
        // TODO can we consume the variables here?
        let x: GraphQLTypeAnnotation<UnvalidatedTypeName> =
            variable.type_.clone().map(|input_type_id| {
                let schema_input_type = schema
                    .server_field_data
                    .lookup_unvalidated_type(input_type_id);
                schema_input_type.name()
            });
        // TODO this is dangerous, since variable.item.name is a WithLocation, which impl's Display.
        // We should find a way to make WithLocation not impl display, without making error's hard
        // to work with.
        variable_text.push_str(&format!("${}: {}", variable.name.item, x));
        if let Some(default_value) = &variable.default_value {
            variable_text.push_str(&format!(" = {}", default_value.item.print_to_string()));
        }
    }

    if empty {
        String::new()
    } else {
        variable_text.push(')');
        variable_text
    }
}

#[allow(clippy::only_used_in_recursion)]
fn write_selections_for_query_text<'a>(
    query_text: &mut String,
    items: impl Iterator<Item = &'a MergedServerSelection> + 'a,
    indentation_level: u8,
) {
    for item in items {
        match &item {
            MergedServerSelection::ScalarField(scalar_field) => {
                query_text.push_str(&"  ".repeat(indentation_level as usize).to_string());
                if let Some(alias) = scalar_field.normalization_alias() {
                    query_text.push_str(&format!("{}: ", alias));
                }
                let name = scalar_field.name;
                let arguments = get_serialized_arguments_for_query_text(&scalar_field.arguments);
                query_text.push_str(&format!("{}{},\\\n", name, arguments));
            }
            MergedServerSelection::LinkedField(linked_field) => {
                query_text.push_str(&"  ".repeat(indentation_level as usize).to_string());
                if let Some(alias) = linked_field.normalization_alias() {
                    // This is bad, alias is WithLocation
                    query_text.push_str(&format!("{}: ", alias));
                }
                let name = linked_field.name;
                let arguments = get_serialized_arguments_for_query_text(&linked_field.arguments);
                query_text.push_str(&format!("{}{} {{\\\n", name, arguments));
                write_selections_for_query_text(
                    query_text,
                    linked_field.selection_map.values(),
                    indentation_level + 1,
                );
                query_text.push_str(&format!(
                    "{}}},\\\n",
                    "  ".repeat(indentation_level as usize)
                ));
            }
            MergedServerSelection::InlineFragment(inline_fragment) => {
                query_text.push_str(&"  ".repeat(indentation_level as usize).to_string());
                query_text.push_str(&format!(
                    "... on {} {{\\\n",
                    inline_fragment.type_to_refine_to
                ));
                write_selections_for_query_text(
                    query_text,
                    inline_fragment.selection_map.values(),
                    indentation_level + 1,
                );
                query_text.push_str(&"  ".repeat(indentation_level as usize).to_string());
                query_text.push_str("},\\\n")
            }
        }
    }
}

fn get_serialized_arguments_for_query_text(arguments: &[ArgumentKeyAndValue]) -> String {
    if arguments.is_empty() {
        "".to_string()
    } else {
        let mut arguments = arguments.iter();
        let first = arguments.next().unwrap();
        let mut s = format!(
            "({}: {}",
            first.key,
            serialize_non_constant_value_for_graphql(&first.value)
        );
        for argument in arguments {
            s.push_str(&format!(
                ", {}: {}",
                argument.key,
                serialize_non_constant_value_for_graphql(&argument.value)
            ));
        }
        s.push(')');
        s
    }
}

fn serialize_non_constant_value_for_graphql(value: &NonConstantValue) -> String {
    match value {
        NonConstantValue::Variable(variable_name) => format!("${}", variable_name),
        NonConstantValue::Integer(int_value) => int_value.to_string(),
        NonConstantValue::Boolean(bool) => bool.to_string(),
        // This clearly isn't correct — the string might have quotes in it and such
        NonConstantValue::String(s) => format!("\"{}\"", s),
        NonConstantValue::Float(f) => f.as_float().to_string(),
        NonConstantValue::Null => "null".to_string(),
        NonConstantValue::Enum(e) => e.to_string(),
        NonConstantValue::List(_) => panic!("Lists are not supported here"),
        NonConstantValue::Object(_) => panic!("Objects not supported here"),
    }
}
