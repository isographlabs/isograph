use std::collections::BTreeSet;

use common_lang_types::{
    Diagnostic, DiagnosticResult, EmbeddedLocation, EntityName, Location, SelectableName,
    ValueKeyName, WithEmbeddedLocation,
};
use graphql_lang_types::NameValuePair;
use intern::{Lookup, string_key::StringKey};
use prelude::{ErrClone, Postfix};

use isograph_lang_types::{
    EntityNameWrapper, NonConstantValue, TypeAnnotationDeclaration, UnionTypeAnnotationDeclaration,
    UnionVariant, VariableDeclaration, VariableNameWrapper,
};

use crate::{
    BOOLEAN_ENTITY_NAME, CompilationProfile, FLOAT_ENTITY_NAME, ID_ENTITY_NAME, INT_ENTITY_NAME,
    IsographDatabase, STRING_ENTITY_NAME, deprecated_server_selectables_map_for_entity,
};

fn scalar_literal_satisfies_type(
    // We supplied an int literal
    supplied_scalar_literal_entity_name: EntityName,
    target_type: &TypeAnnotationDeclaration,
    location: EmbeddedLocation,
    type_description: &'static str,
) -> DiagnosticResult<()> {
    let matches = match target_type {
        TypeAnnotationDeclaration::Scalar(target_entity_name_wrapper) => {
            target_entity_name_wrapper.0 == supplied_scalar_literal_entity_name
        }
        TypeAnnotationDeclaration::Union(target_union) => {
            // The union must contain an int
            target_union.variants.contains(&UnionVariant::Scalar(
                supplied_scalar_literal_entity_name.into(),
            ))
        }
        TypeAnnotationDeclaration::Plural(_) => false,
    };

    if !matches {
        return expected_type_found_something_else_named_diagnostic(
            target_type,
            supplied_scalar_literal_entity_name.unchecked_conversion(),
            type_description,
            location,
        )
        .wrap_err();
    }
    Ok(())
}

fn variable_type_satisfies_argument_type(
    supplied_type: &TypeAnnotationDeclaration,
    target_type: &TypeAnnotationDeclaration,
) -> bool {
    match target_type {
        TypeAnnotationDeclaration::Scalar(target_scalar) => match supplied_type {
            TypeAnnotationDeclaration::Scalar(supplied_scalar) => target_scalar == supplied_scalar,
            TypeAnnotationDeclaration::Union(supplied_union) => {
                // Each variant of the union must match the scalar_arg, and it must not be
                // nullable
                //
                // (Is that a safe assumption? What if scalar is part of a nullable union...?
                // Hopefully our types are well-formed!)
                !supplied_union.nullable
                    && supplied_union.variants.iter().all(|union_variant| {
                        union_variant_matches_scalar_arg(union_variant, target_scalar.dereference())
                    })
            }
            TypeAnnotationDeclaration::Plural(_supplied_plural) => false,
        },
        TypeAnnotationDeclaration::Union(target_union) => {
            match supplied_type {
                TypeAnnotationDeclaration::Scalar(supplied_scalar) => {
                    // This scalar must be a member of the union
                    union_contains(target_union, &UnionVariant::Scalar(*supplied_scalar))
                }
                TypeAnnotationDeclaration::Union(supplied_union) => {
                    // If the variable is a union and the source is a union, then each variant
                    // in the variable must be a valid union member, and
                    // the union_arg must be nullable or the union_var must be not nullable
                    (target_union.nullable || !supplied_union.nullable)
                        && supplied_union.variants.iter().all(|union_variant_var| {
                            union_contains(target_union, union_variant_var)
                        })
                }
                TypeAnnotationDeclaration::Plural(supplied_plural) => {
                    // This plural must be a member of the union
                    union_contains(
                        target_union,
                        &UnionVariant::Plural(*supplied_plural.clone()),
                    )
                }
            }
        }
        TypeAnnotationDeclaration::Plural(target_plural) => match supplied_type {
            TypeAnnotationDeclaration::Scalar(_supplied_scalar) => false,
            TypeAnnotationDeclaration::Union(supplied_union) => {
                // each variant of the union must match the plural_arg_type, and
                // it must not be nullable. Again, assuming that the types are well-formed
                !supplied_union.nullable
                    && supplied_union
                        .variants
                        .iter()
                        .all(|variant_var| match variant_var {
                            UnionVariant::Scalar(_) => false,
                            UnionVariant::Plural(plural_var) => {
                                variable_type_satisfies_argument_type(
                                    plural_var.item.reference(),
                                    target_plural.item.reference(),
                                )
                            }
                        })
            }
            TypeAnnotationDeclaration::Plural(supplied_plural) => {
                variable_type_satisfies_argument_type(
                    supplied_plural.item.reference(),
                    target_plural.item.reference(),
                )
            }
        },
    }
}

fn union_variant_matches_scalar_arg(
    union_variant_var: &UnionVariant,
    scalar_arg: EntityNameWrapper,
) -> bool {
    match union_variant_var {
        UnionVariant::Scalar(union_var_entity_name_wrapper) => {
            union_var_entity_name_wrapper.dereference() == scalar_arg
        }
        UnionVariant::Plural(_) => false,
    }
}

fn union_contains(union: &UnionTypeAnnotationDeclaration, potential_member: &UnionVariant) -> bool {
    union.variants.contains(potential_member)
}

pub fn value_satisfies_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    selection_supplied_argument_value: &WithEmbeddedLocation<NonConstantValue>,
    field_argument_definition_type: &TypeAnnotationDeclaration,
    variable_definitions: &[VariableDeclaration],
) -> DiagnosticResult<()> {
    match selection_supplied_argument_value.item.reference() {
        NonConstantValue::Variable(supplied_variable_name) => {
            let variable_type = get_variable_type(
                supplied_variable_name,
                variable_definitions,
                selection_supplied_argument_value.location,
            )?;
            if variable_type_satisfies_argument_type(variable_type, field_argument_definition_type)
            {
                Ok(())
            } else {
                Diagnostic::new(
                    format!(
                        "Mismatched type. Received ${supplied_variable_name}, with type {variable_type}, \
                        expected {field_argument_definition_type}."
                    ),
                    selection_supplied_argument_value
                        .location
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err()
            }
        }
        NonConstantValue::Integer(_) => scalar_literal_satisfies_type(
            *INT_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
            "an integer literal",
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *FLOAT_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.location,
                "an integer literal",
            )
            .map_err(|_| error)
        })
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *ID_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.location,
                "an integer literal",
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Boolean(_) => scalar_literal_satisfies_type(
            *BOOLEAN_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
            "a boolean literal",
        ),
        NonConstantValue::String(_) => scalar_literal_satisfies_type(
            *STRING_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
            "a string literal",
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *ID_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.location,
                "a string literal",
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Float(_) => scalar_literal_satisfies_type(
            *FLOAT_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
            "a float literal",
        ),
        NonConstantValue::Enum(_enum_literal_value) => {
            todo!("Support validation of enum literals")
        }
        NonConstantValue::Null => {
            if field_argument_definition_type.is_nullable() {
                Ok(())
            } else {
                Diagnostic::new(
                    format!("Expected non null input of type {field_argument_definition_type}, found null"),
                    selection_supplied_argument_value
                        .location
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err()
            }
        }
        NonConstantValue::List(list) => list_satisfies_type(
            db,
            list,
            field_argument_definition_type,
            variable_definitions,
        ),
        NonConstantValue::Object(object_literal) => {
            match field_argument_definition_type {
                TypeAnnotationDeclaration::Scalar(target_scalar) => object_satisfies_type(
                    db,
                    selection_supplied_argument_value,
                    variable_definitions,
                    object_literal,
                    target_scalar.0,
                ),
                TypeAnnotationDeclaration::Union(target_union) => {
                    let matches = target_union.variants.iter().any(|target_union_variant| {
                        match target_union_variant {
                            UnionVariant::Scalar(target_union_name) => object_satisfies_type(
                                db,
                                selection_supplied_argument_value,
                                variable_definitions,
                                object_literal,
                                target_union_name.0,
                            )
                            .is_ok(),
                            UnionVariant::Plural(_) => false,
                        }
                    });

                    if !matches {
                        return Diagnostic::new(
                            "Item did not match any union variants".to_string(),
                            selection_supplied_argument_value
                                .location
                                .to::<Location>()
                                .wrap_some(),
                        )
                        .wrap_err();
                    }

                    Ok(())

                    // Object must satisfy at least one union variant
                }
                TypeAnnotationDeclaration::Plural(_) => Diagnostic::new(
                    "Mismatched type.".to_string(),
                    selection_supplied_argument_value
                        .location
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err(),
            }
        }
    }
}

fn object_satisfies_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    selection_supplied_argument_value: &WithEmbeddedLocation<NonConstantValue>,
    variable_definitions: &[VariableDeclaration],
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object_entity_name: EntityName,
) -> DiagnosticResult<()> {
    validate_no_extraneous_fields(
        db,
        object_entity_name,
        object_literal,
        selection_supplied_argument_value.location,
    )?;

    let missing_fields =
        get_non_nullable_missing_and_provided_fields(db, object_literal, object_entity_name)?
            .iter()
            .filter_map(|field| match field {
                ObjectLiteralFieldType::Provided(
                    field_type_annotation,
                    selection_supplied_argument_value,
                ) => match value_satisfies_type(
                    db,
                    &selection_supplied_argument_value.value,
                    field_type_annotation.reference(),
                    variable_definitions,
                ) {
                    Ok(_) => None,
                    Err(e) => e.wrap_err().wrap_some(),
                },
                ObjectLiteralFieldType::Missing(field_name) => (*field_name).wrap_ok().wrap_some(),
            })
            .collect::<Result<Vec<_>, _>>()?;

    if missing_fields.is_empty() {
        Ok(())
    } else {
        Diagnostic::new(
            format!(
                "This object has missing fields: {}",
                // TODO smart joining: a, b, and c
                // TODO don't materialize a vec... reduce
                missing_fields
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            selection_supplied_argument_value
                .location
                .to::<Location>()
                .wrap_some(),
        )
        .wrap_err()
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
enum ObjectLiteralFieldType {
    Provided(
        TypeAnnotationDeclaration,
        NameValuePair<ValueKeyName, NonConstantValue>,
    ),
    Missing(SelectableName),
}

fn get_non_nullable_missing_and_provided_fields<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    server_object_entity_name: EntityName,
) -> DiagnosticResult<BTreeSet<ObjectLiteralFieldType>> {
    let server_selectables =
        deprecated_server_selectables_map_for_entity(db, server_object_entity_name).clone_err()?;

    server_selectables
        .iter()
        .filter_map(|(field_name, selectable)| {
            let iso_type_annotation = selectable.lookup(db).target_entity.item.as_ref().ok()?;

            let object_literal_supplied_field = object_literal
                .iter()
                .find(|field| field.name.item.lookup() == (*field_name).lookup());

            match object_literal_supplied_field {
                Some(selection_supplied_argument_value) => ObjectLiteralFieldType::Provided(
                    iso_type_annotation.clone(),
                    selection_supplied_argument_value.clone(),
                )
                .wrap_some(),
                None => match iso_type_annotation {
                    TypeAnnotationDeclaration::Scalar(_) => {
                        ObjectLiteralFieldType::Missing(*field_name).wrap_some()
                    }
                    TypeAnnotationDeclaration::Union(_union_type_annotation) => None,
                    TypeAnnotationDeclaration::Plural(_) => None,
                },
            }
        })
        .collect::<BTreeSet<_>>()
        .wrap_ok()
}

fn validate_no_extraneous_fields<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityName,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    location: EmbeddedLocation,
) -> DiagnosticResult<()> {
    let object_fields =
        deprecated_server_selectables_map_for_entity(db, parent_server_object_entity_name)
            .clone_err()?;

    let extra_fields: Vec<_> = object_literal
        .iter()
        .filter_map(|field| {
            let is_defined = object_fields
                .get(&field.name.item.unchecked_conversion())
                .is_some();

            if !is_defined {
                return field.clone().wrap_some();
            }
            None
        })
        .collect();

    if !extra_fields.is_empty() {
        return Diagnostic::new(
            format!(
                "This object has extra fields: {0}",
                // TODO smart join
                extra_fields
                    .iter()
                    .map(|field| format!("{}", field.name.item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            location.to::<Location>().wrap_some(),
        )
        .wrap_err();
    }
    Ok(())
}

fn list_satisfies_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    supplied_list: &[WithEmbeddedLocation<NonConstantValue>],
    target_type: &TypeAnnotationDeclaration,
    variable_definitions: &[VariableDeclaration],
) -> DiagnosticResult<()> {
    supplied_list.iter().try_for_each(|element| {
        value_satisfies_type(db, element, target_type.reference(), variable_definitions)
    })
}

fn get_variable_type<'a>(
    variable_name: &'a VariableNameWrapper,
    variable_definitions: &'a [VariableDeclaration],
    location: EmbeddedLocation,
) -> DiagnosticResult<&'a TypeAnnotationDeclaration> {
    match variable_definitions
        .iter()
        .find(|definition| definition.name.item == *variable_name)
    {
        Some(variable) => (variable.type_.item.reference()).wrap_ok(),
        None => Diagnostic::new(
            format!("This variable is not defined: ${}", *variable_name),
            location.to::<Location>().wrap_some(),
        )
        .wrap_err(),
    }
}

fn expected_type_found_something_else_named_diagnostic(
    expected: &TypeAnnotationDeclaration,
    actual: StringKey,
    type_description: &str,
    location: EmbeddedLocation,
) -> Diagnostic {
    Diagnostic::new(
        format!("Expected input of type {expected}, found {actual} {type_description}"),
        location.to::<Location>().wrap_some(),
    )
}
