use common_lang_types::{QueryOperationName, QueryText};
use isograph_lang_types::{
    ArgumentKeyAndValue, NonConstantValue, VariableDeclaration,
    graphql_type_annotation_from_type_annotation,
};
use isograph_schema::{
    Format, MergedSelectionMap, MergedServerSelection, WrappedMergedSelectionMap,
};
use prelude::Postfix;

use crate::GraphQLOperationKind;

pub(crate) fn generate_query_text<'a>(
    operation_kind: GraphQLOperationKind,
    query_name: QueryOperationName,
    selection_map: &WrappedMergedSelectionMap,
    query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
    format: Format,
) -> QueryText {
    let mut query_text = String::new();

    let variable_text = write_variables_to_string(query_variables);
    query_text.push_str(&format!("{operation_kind} {query_name}{variable_text} {{"));
    match format {
        Format::Pretty => query_text.push_str("\\\n"),
        Format::Compact => query_text.push(' '),
    }
    write_selections_for_query_text(
        &mut query_text,
        selection_map.clone().inner().reference(),
        1,
        format,
    );
    query_text.push('}');
    QueryText(query_text)
}

fn write_variables_to_string<'a>(
    variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
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
        variable_text.push_str(&format!(
            "${}: {}",
            variable.name.item,
            graphql_type_annotation_from_type_annotation(variable.type_.item.reference())
        ));
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

fn write_selections_for_query_text(
    query_text: &mut String,
    items: &MergedSelectionMap,
    indentation_level: u8,
    format: Format,
) {
    let (new_line, indent) = match format {
        Format::Pretty => ("\\\n", &"  ".repeat(indentation_level as usize).to_string()),
        Format::Compact => (" ", &"".to_string()),
    };

    if items.is_empty() {
        query_text.push_str(indent);
        query_text.push_str("__typename,");
        query_text.push_str(new_line);
    } else {
        for item in items.values() {
            match item.reference() {
                MergedServerSelection::ScalarField(scalar_field) => {
                    query_text.push_str(indent);
                    if let Some(alias) = scalar_field.normalization_alias() {
                        query_text.push_str(&format!("{alias}: "));
                    }
                    let name = scalar_field.name;
                    let arguments =
                        get_serialized_arguments_for_query_text(&scalar_field.arguments);
                    query_text.push_str(&format!("{name}{arguments},{new_line}"));
                }
                MergedServerSelection::LinkedField(linked_field) => {
                    query_text.push_str(indent);
                    if let Some(alias) = linked_field.normalization_alias() {
                        query_text.push_str(&format!("{alias}: "));
                    }
                    let name = linked_field.name;
                    let arguments =
                        get_serialized_arguments_for_query_text(&linked_field.arguments);
                    query_text.push_str(&format!("{name}{arguments} {{{new_line}"));
                    write_selections_for_query_text(
                        query_text,
                        &linked_field.selection_map,
                        indentation_level + 1,
                        format,
                    );
                    query_text.push_str(&format!("{indent}}},{new_line}"));
                }
                MergedServerSelection::ClientObjectSelectable(_) => {}
                MergedServerSelection::InlineFragment(inline_fragment) => {
                    query_text.push_str(indent);
                    query_text.push_str(&format!(
                        "... on {} {{{}",
                        inline_fragment.type_to_refine_to, new_line,
                    ));
                    write_selections_for_query_text(
                        query_text,
                        &inline_fragment.selection_map,
                        indentation_level + 1,
                        format,
                    );
                    query_text.push_str(&format!("{indent}}},{new_line}"));
                }
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
        NonConstantValue::Variable(variable_name) => format!("${variable_name}"),
        NonConstantValue::Integer(int_value) => int_value.to_string(),
        NonConstantValue::Boolean(bool) => bool.to_string(),
        // This clearly isn't correct — the string might have quotes in it and such
        NonConstantValue::String(s) => format!("\"{s}\""),
        NonConstantValue::Float(f) => f.as_float().to_string(),
        NonConstantValue::Null => "null".to_string(),
        NonConstantValue::Enum(e) => e.to_string(),
        NonConstantValue::List(_) => panic!("Lists are not supported here"),
        NonConstantValue::Object(object) => format!(
            "{{ {} }}",
            object
                .iter()
                .map(|entry| format!(
                    "{}: {}",
                    entry.name.item,
                    serialize_non_constant_value_for_graphql(&entry.value.item)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
