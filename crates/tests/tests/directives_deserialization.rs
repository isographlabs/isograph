use common_lang_types::{Diagnostic, SelectableName, StringLiteralValue, TextSource};
use graphql_lang_types::{GraphQLConstantValue, GraphQLDirective, from_graphql_directives};
use graphql_lang_types::{GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionOrDefinition};
use intern::string_key::Intern;
use isograph_schema::ServerObjectEntityDirectives;
use isograph_schema::{ExposeFieldDirective, FieldMapItem, ID_FIELD_NAME};
use std::error::Error;

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
        relative_path_to_source_file: "dummy".intern().into(),
        span: None,
        current_working_directory: "cwd".intern().into(),
    };
    let document = graphql_schema_parser::parse_schema_extensions(source, text_source)?;
    let directives = document
        .0
        .into_iter()
        .map(|dir| unwrap_directive(dir.item))
        .collect::<Result<Vec<_>, _>>()?;
    let directives: Vec<GraphQLDirective<GraphQLConstantValue>> =
        directives.into_iter().flatten().collect();

    let server_object_entity_directives: ServerObjectEntityDirectives =
        from_graphql_directives(&directives)?;

    Ok(server_object_entity_directives.expose_field)
}

#[test]
fn test_test_mutation_extension_expose_as() -> Result<(), Box<dyn Error>> {
    let expose_field_mutations = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_valid_as.graphql"
    ))?;
    let expose_as = Some(SelectableName::from("set_puppy_tagline".intern()));
    let field_map = vec![FieldMapItem {
        from: ID_FIELD_NAME.unchecked_conversion(),
        to: StringLiteralValue::from("input.id".intern()),
    }];
    let field = StringLiteralValue::from("set_pet_tagline.pet".intern());
    let set_tagline_mutation = ExposeFieldDirective {
        expose_as,
        field_map,
        field,
        limit_to: None,
    };

    assert_eq!(expose_field_mutations[0], set_tagline_mutation);
    Ok(())
}

#[test]
fn test_test_mutation_extension_set_pet_tagline_parsing() -> Result<(), Box<dyn Error>> {
    let expose_field_mutations = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_valid.graphql"
    ))?;
    let field_map = vec![FieldMapItem {
        from: ID_FIELD_NAME.unchecked_conversion(),
        to: StringLiteralValue::from("input.id".intern()),
    }];
    let field = StringLiteralValue::from("set_pet_tagline.pet".intern());
    let set_tagline_mutation = ExposeFieldDirective {
        expose_as: None,
        field_map,
        field,
        limit_to: None,
    };

    assert_eq!(expose_field_mutations[0], set_tagline_mutation);
    Ok(())
}

#[test]
fn test_mutation_extension_set_pet_bestfriend_parsing() -> Result<(), Box<dyn Error>> {
    let expose_field_directives = parse_mutation(include_str!(
        "fixtures/directives/mutation_extension_valid.graphql"
    ))?;
    let field_map = vec![FieldMapItem {
        from: ID_FIELD_NAME.unchecked_conversion(),
        to: ID_FIELD_NAME.unchecked_conversion(),
    }];
    let field = StringLiteralValue::from("set_pet_best_friend.pet".intern());
    let set_pet_best_friend = ExposeFieldDirective {
        expose_as: None,
        field_map,
        field,
        limit_to: None,
    };
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
            if let Some(deserialization_error) = e.downcast_ref::<Diagnostic>() {
                assert!(
                    deserialization_error.0.message == message,
                    "Expected DeserializationError::Custom, got {deserialization_error:?}"
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
        "unknown field `weight`, expected one of `as`, `fieldMap`, `field`",
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
