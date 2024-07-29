use common_lang_types::{SelectableFieldName, StringLiteralValue, TextSource};
use graphql_lang_types::{
    from_graph_ql_directive, DeserializationError, GraphQLConstantValue, GraphQLDirective,
};
use intern::string_key::Intern;
use isograph_schema::{ExposeFieldDirective, FieldMapItem};
use std::error::Error;

use graphql_lang_types::{GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionOrDefinition};

fn unwrap_directive(
    extension_or_definition: GraphQLTypeSystemExtensionOrDefinition,
) -> Result<Vec<GraphQLDirective<GraphQLConstantValue>>, Box<dyn Error>> {
    if let GraphQLTypeSystemExtensionOrDefinition::Extension(extension) = extension_or_definition {
        let GraphQLTypeSystemExtension::ObjectTypeExtension(object_type_extension) = extension;
        return Ok(object_type_extension.directives.clone());
    }
    Err("unexpected structure of directive".into())
}

fn parse_mutation(source: &str) -> Result<Vec<ExposeFieldDirective>, Box<dyn Error>> {
    let text_source = TextSource {
        path: "dummy".intern().into(),
        span: None,
    };
    let document =
        graphql_schema_parser::parse_schema_extensions(source, text_source).map_err(|e| e.item)?;
    let directives = document
        .0
        .into_iter()
        .map(|dir| unwrap_directive(dir.item))
        .collect::<Result<Vec<_>, _>>()?;
    let directives: Vec<GraphQLDirective<GraphQLConstantValue>> =
        directives.into_iter().flatten().collect();
    let expose_field_directives: Result<Vec<ExposeFieldDirective>, _> = directives
        .into_iter()
        .map(|directive| from_graph_ql_directive::<ExposeFieldDirective>(&directive))
        .collect();
    Ok(expose_field_directives?)
}

#[test]
fn test_test_mutation_extension_expose_as() -> Result<(), Box<dyn Error>> {
    let expose_field_mutations = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_valid_as.graphql"
    ))?;
    let set_tagline_mutation = ExposeFieldDirective::new(
        Some(SelectableFieldName::from("set_puppy_tagline".intern())),
        StringLiteralValue::from("pet".intern()),
        vec![FieldMapItem {
            from: StringLiteralValue::from("id".intern()),
            to: StringLiteralValue::from("input.id".intern()),
        }],
        StringLiteralValue::from("set_pet_tagline".intern()),
    );

    assert_eq!(expose_field_mutations[0], set_tagline_mutation);
    Ok(())
}

#[test]
fn test_test_mutation_extension_set_pet_tagline_parsing() -> Result<(), Box<dyn Error>> {
    let expose_field_mutations = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_valid.graphql"
    ))?;
    let set_tagline_mutation = ExposeFieldDirective::new(
        None,
        StringLiteralValue::from("pet".intern()),
        vec![FieldMapItem {
            from: StringLiteralValue::from("id".intern()),
            to: StringLiteralValue::from("input.id".intern()),
        }],
        StringLiteralValue::from("set_pet_tagline".intern()),
    );

    assert_eq!(expose_field_mutations[0], set_tagline_mutation);
    Ok(())
}

#[test]
fn test_mutation_extension_set_pet_bestfriend_parsing() -> Result<(), Box<dyn Error>> {
    let expose_field_directives = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_valid.graphql"
    ))?;
    let set_pet_best_friend = ExposeFieldDirective::new(
        None,
        StringLiteralValue::from("pet".intern()),
        vec![FieldMapItem {
            from: StringLiteralValue::from("id".intern()),
            to: StringLiteralValue::from("id".intern()),
        }],
        StringLiteralValue::from("set_pet_best_friend".intern()),
    );
    assert_eq!(expose_field_directives[1], set_pet_best_friend);
    Ok(())
}

fn match_failure_message(
    expose_field_directives: Result<Vec<ExposeFieldDirective>, Box<dyn Error>>,
    message: &str,
) {
    match expose_field_directives {
        Ok(_) => panic!("Expected an error, but got Ok"),
        Err(e) => {
            if let Some(deserialization_error) = e.downcast_ref::<DeserializationError>() {
                assert!(
                    matches!(deserialization_error, DeserializationError::Custom(ref msg) if msg==message),
                    "Expected DeserializationError::Custom, got {:?}",
                    deserialization_error
                );
            } else {
                panic!("Expected DeserializationError, got a different error type");
            }
        }
    }
}
#[test]
fn test_mutation_extension_extra_topfield_parsing_failure() -> Result<(), Box<dyn Error>> {
    let expose_field_directives = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_extra_toplevelfield.graphql"
    ));
    match_failure_message(
        expose_field_directives,
        "unknown field `weight`, expected one of `as`, `path`, `fieldMap`, `field`",
    );
    Ok(())
}

#[test]
fn test_mutation_extension_extra_nestedfield_parsing_failure() -> Result<(), Box<dyn Error>> {
    let expose_field_directives = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_extra_nestedfield.graphql"
    ));
    match_failure_message(
        expose_field_directives,
        "unknown field `day`, expected `from` or `to`",
    );
    Ok(())
}

#[test]
fn test_mutation_extension_missing_topfield_parsing_failure() -> Result<(), Box<dyn Error>> {
    let expose_field_directives = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_missing_toplevelfield.graphql"
    ));
    match_failure_message(expose_field_directives, "missing field `field`");
    Ok(())
}

#[test]
fn test_mutation_extension_missing_nestedfield_parsing_failure() -> Result<(), Box<dyn Error>> {
    let expose_field_directives = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_missing_nestedfield.graphql"
    ));
    match_failure_message(expose_field_directives, "missing field `from`");
    Ok(())
}
