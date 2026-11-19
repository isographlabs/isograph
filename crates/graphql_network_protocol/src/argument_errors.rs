use common_lang_types::{ArgumentName, EntityName, ValueKeyName, VariableName};
use graphql_schema_parser::{NullableTypeAnnotation, TypeAnnotation};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, IsographDb, NonNullVariableType, ProtocolEntity, SelectableKey,
    VariableType, variable_type,
};
use isograph_parser::{ArgumentList, NonConstantValue, VariableDeclarationList};
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::print_operation::print_variable_type;
use crate::protocol_data::{GraphQLEntityData, graphql_entity, input_fields};
use crate::schema_lookup::{entity_named, selectable_named};

/// What is wrong with an argument a selection passes.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLArgumentError {
    #[error("`{}` requires the argument `{}`, and this selection does not pass it.", .0.field, .0.argument)]
    MissingArgument(ArgumentOf),
    #[error("`{}` takes no argument `{}`.", .0.field, .0.argument)]
    UnknownArgument(ArgumentOf),
    #[error("This variable is not defined: ${0}")]
    UndefinedVariable(VariableName),
    #[error(
        "Mismatched type. Received ${}, with type {}, expected {}.",
        .0.variable,
        print_variable_type(.0.variable_type.reference()),
        print_variable_type(.0.expected.reference())
    )]
    VariableTypeMismatch(Box<VariableTypeMismatch>),
    #[error(
        "Expected input of type {}, found {}.",
        print_variable_type(.0.expected.reference()),
        .0.literal
    )]
    LiteralTypeMismatch(Box<LiteralTypeMismatch>),
    #[error("Expected non null input of type {}, found null", print_variable_type(.0))]
    NullForNonNull(Box<VariableType>),
    #[error("`{0}` is outside GraphQL's 32-bit `Int` range.")]
    IntegerOutOfRange(i64),
    #[error("`{}` requires the field `{}`, and this object does not have it.", .0.entity, .0.field)]
    MissingInputField(InputFieldOf),
    #[error("`{}` has no field `{}`.", .0.entity, .0.field)]
    UnknownInputField(InputFieldOf),
}

/// An argument of a selectable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgumentOf {
    pub field: SelectableKey,
    pub argument: ArgumentName,
}

/// A field of an input object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputFieldOf {
    pub entity: EntityName,
    pub field: ValueKeyName,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableTypeMismatch {
    pub variable: VariableName,
    pub variable_type: VariableType,
    pub expected: VariableType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralTypeMismatch {
    pub expected: VariableType,
    pub literal: LiteralKind,
}

/// The kind of a literal argument value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display)]
pub enum LiteralKind {
    #[strum(to_string = "a string literal")]
    String,
    #[strum(to_string = "an integer literal")]
    Integer,
    #[strum(to_string = "a boolean literal")]
    Boolean,
    #[strum(to_string = "an object literal")]
    Object,
    #[strum(to_string = "a list literal")]
    List,
}

/// Whether an input value definition has a default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DefaultValue {
    Present,
    Absent,
}

/// One argument or input field the schema defines.
struct Expected {
    name: &'static str,
    type_: VariableType,
    default: DefaultValue,
}

impl Expected {
    /// Required: non-null with no default.
    fn is_required(&self) -> bool {
        matches!(
            (&self.type_, self.default),
            (VariableType::NonNull(_), DefaultValue::Absent)
        )
    }
}

/// The variables of the `field` declaration a selection is in, with their types when they
/// converted.
struct Scope(Vec<(VariableName, Option<VariableType>)>);

impl Scope {
    fn of(variables: Option<&WithSpan<VariableDeclarationList>>) -> Self {
        Scope(
            variables
                .iter()
                .flat_map(|list| list.item.0.iter())
                .filter_map(|slot| slot.item.item.as_ref())
                .map(|variable| {
                    (
                        variable.item.name.item.0.item.0,
                        variable_type(variable.item.type_.reference()).ok(),
                    )
                })
                .collect(),
        )
    }
}

/// The schema type `annotation` as a variable type.
fn variable_type_of_graphql_type(annotation: &TypeAnnotation) -> VariableType {
    match annotation {
        TypeAnnotation::Named(named) => {
            VariableType::Nullable(NonNullVariableType::Named(named.0.item.0))
        }
        TypeAnnotation::List(list) => VariableType::Nullable(NonNullVariableType::List(
            variable_type_of_graphql_type(list.0.item.reference()).boxed(),
        )),
        TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
            NullableTypeAnnotation::Named(named) => {
                VariableType::NonNull(NonNullVariableType::Named(named.0.item.0))
            }
            NullableTypeAnnotation::List(list) => VariableType::NonNull(NonNullVariableType::List(
                variable_type_of_graphql_type(list.0.item.reference()).boxed(),
            )),
        },
    }
}

/// Upstream's `validate_use_of_arguments_impl` and `value_satisfies_type` for a selection of a
/// schema field, refinement, or `__typename`.
pub(crate) fn selection_argument_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selectable: SelectableKey,
    selection_name: Span,
    arguments: Option<&WithSpan<ArgumentList>>,
    variables: Option<&WithSpan<VariableDeclarationList>>,
) -> Vec<WithSpan<GraphQLArgumentError>> {
    let Ok(field) = selectable_named(db, client, selectable.entity, selectable.selectable) else {
        return Vec::new();
    };
    let expected: Vec<Expected> = field
        .iter()
        .flat_map(|field| field.item.item.arguments.iter())
        .map(|argument| Expected {
            name: argument.item.name.item.0.lookup(),
            type_: variable_type_of_graphql_type(argument.item.type_.item.reference()),
            default: match argument.item.default_value {
                Some(_) => DefaultValue::Present,
                None => DefaultValue::Absent,
            },
        })
        .collect();
    let scope = Scope::of(variables);
    let passed: Vec<_> = passed_arguments(arguments).collect();
    let missing = expected
        .iter()
        .filter(|expected| {
            expected.is_required()
                && !passed
                    .iter()
                    .any(|argument| argument.item.name.item.0.lookup() == expected.name)
        })
        .map(|expected| {
            GraphQLArgumentError::MissingArgument(ArgumentOf {
                field: selectable,
                argument: expected.name.intern().to(),
            })
            .with_span(selection_name)
        });
    let checked = passed.iter().flat_map(|argument| {
        match expected
            .iter()
            .find(|expected| argument.item.name.item.0.lookup() == expected.name)
        {
            None => GraphQLArgumentError::UnknownArgument(ArgumentOf {
                field: selectable,
                argument: argument.item.name.item.0,
            })
            .with_span(argument.item.name.location)
            .wrap_vec(),
            Some(expected) => value_errors(
                db,
                client,
                scope.reference(),
                argument.item.value.reference(),
                expected.type_.reference(),
            ),
        }
    });
    missing.chain(checked).collect()
}

/// The type errors of the values a selection passes to a `field` declaration whose variables
/// are `field_variables`.
pub(crate) fn field_argument_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field_variables: Option<&WithSpan<VariableDeclarationList>>,
    arguments: Option<&WithSpan<ArgumentList>>,
    variables: Option<&WithSpan<VariableDeclarationList>>,
) -> Vec<WithSpan<GraphQLArgumentError>> {
    let declared = Scope::of(field_variables);
    let scope = Scope::of(variables);
    passed_arguments(arguments)
        .flat_map(|argument| {
            let expected = declared.0.iter().find_map(|(name, type_)| {
                (name.lookup() == argument.item.name.item.0.lookup())
                    .then_some(type_.as_ref())
                    .flatten()
            });
            match expected {
                None => Vec::new(),
                Some(expected) => value_errors(
                    db,
                    client,
                    scope.reference(),
                    argument.item.value.reference(),
                    expected,
                ),
            }
        })
        .collect()
}

fn passed_arguments(
    arguments: Option<&WithSpan<ArgumentList>>,
) -> impl Iterator<Item = &WithSpan<isograph_parser::Argument>> {
    arguments
        .into_iter()
        .flat_map(|arguments| arguments.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
}

/// GraphQL's rule for a variable of `supplied` type in a position of `expected` type
/// (upstream's `variable_type_satisfies_argument_type`).
fn variable_type_satisfies(supplied: &VariableType, expected: &VariableType) -> bool {
    match (supplied, expected) {
        (VariableType::Nullable(_), VariableType::NonNull(_)) => false,
        (
            VariableType::NonNull(supplied) | VariableType::Nullable(supplied),
            VariableType::NonNull(expected) | VariableType::Nullable(expected),
        ) => match (supplied, expected) {
            (NonNullVariableType::Named(supplied), NonNullVariableType::Named(expected)) => {
                supplied == expected
            }
            (NonNullVariableType::List(supplied), NonNullVariableType::List(expected)) => {
                variable_type_satisfies(supplied, expected)
            }
            (NonNullVariableType::Named(_), NonNullVariableType::List(_))
            | (NonNullVariableType::List(_), NonNullVariableType::Named(_)) => false,
        },
    }
}

/// The errors of `value` in a position of `expected` type.
fn value_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    scope: &Scope,
    value: &WithSpan<NonConstantValue>,
    expected: &VariableType,
) -> Vec<WithSpan<GraphQLArgumentError>> {
    let inner = match expected {
        VariableType::NonNull(inner) | VariableType::Nullable(inner) => inner,
    };
    let kind = match value.item.reference() {
        NonConstantValue::Variable(variable) => {
            let name = variable.0.item.0.item.0;
            return match scope.0.iter().find(|(declared, _)| *declared == name) {
                None => GraphQLArgumentError::UndefinedVariable(name)
                    .with_span(value.location)
                    .wrap_vec(),
                Some((_, None)) => Vec::new(),
                Some((_, Some(variable_type))) => {
                    if variable_type_satisfies(variable_type, expected) {
                        Vec::new()
                    } else {
                        GraphQLArgumentError::VariableTypeMismatch(
                            VariableTypeMismatch {
                                variable: name,
                                variable_type: variable_type.clone(),
                                expected: expected.clone(),
                            }
                            .boxed(),
                        )
                        .with_span(value.location)
                        .wrap_vec()
                    }
                }
            };
        }
        NonConstantValue::Null(_) => {
            return match expected {
                VariableType::Nullable(_) => Vec::new(),
                VariableType::NonNull(_) => {
                    GraphQLArgumentError::NullForNonNull(expected.clone().boxed())
                        .with_span(value.location)
                        .wrap_vec()
                }
            };
        }
        NonConstantValue::List(list) => {
            if let NonNullVariableType::List(item_type) = inner {
                return list
                    .0
                    .iter()
                    .filter_map(|slot| slot.item.item.as_ref())
                    .flat_map(|item| {
                        value_errors(db, client, scope, item.item.value.reference(), item_type)
                    })
                    .collect();
            }
            LiteralKind::List
        }
        NonConstantValue::String(_) => LiteralKind::String,
        NonConstantValue::Integer(_) => LiteralKind::Integer,
        NonConstantValue::Boolean(_) => LiteralKind::Boolean,
        NonConstantValue::Object(_) => LiteralKind::Object,
    };
    match inner {
        NonNullVariableType::List(item_type) => value_errors(db, client, scope, value, item_type),
        NonNullVariableType::Named(entity) => named_value_errors(
            db,
            client,
            scope,
            value,
            kind,
            entity.dereference(),
            expected,
        ),
    }
}

/// The errors of the literal `value` of `kind` in a position whose named type is `entity`.
fn named_value_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    scope: &Scope,
    value: &WithSpan<NonConstantValue>,
    kind: LiteralKind,
    entity: EntityName,
    expected: &VariableType,
) -> Vec<WithSpan<GraphQLArgumentError>> {
    let mismatch = || {
        GraphQLArgumentError::LiteralTypeMismatch(
            LiteralTypeMismatch {
                expected: expected.clone(),
                literal: kind,
            }
            .boxed(),
        )
        .with_span(value.location)
        .wrap_vec()
    };
    let Ok(Some(ProtocolEntity { data, .. })) = graphql_entity(db, client, entity) else {
        return Vec::new();
    };
    match (data, value.item.reference()) {
        (GraphQLEntityData::InputObject, NonConstantValue::Object(object)) => {
            object_errors(db, client, scope, value.location, object, entity)
        }
        (GraphQLEntityData::InputObject | GraphQLEntityData::Enum, _) => mismatch(),
        (GraphQLEntityData::Scalar(_), NonConstantValue::Integer(integer))
            if entity.lookup() == "Int" && i32::try_from(integer.0).is_err() =>
        {
            GraphQLArgumentError::IntegerOutOfRange(integer.0)
                .with_span(value.location)
                .wrap_vec()
        }
        (GraphQLEntityData::Scalar(_), _) => match scalar_accepts(entity.lookup(), kind) {
            ScalarAcceptance::Accepted => Vec::new(),
            ScalarAcceptance::Rejected => mismatch(),
        },
        (
            GraphQLEntityData::Object(_)
            | GraphQLEntityData::Interface(_)
            | GraphQLEntityData::Union,
            _,
        ) => Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarAcceptance {
    Accepted,
    Rejected,
}

/// Whether the scalar `name` accepts a literal of `kind`. A custom scalar's literal coercion is
/// defined by the server, so it accepts every literal.
fn scalar_accepts(name: &str, kind: LiteralKind) -> ScalarAcceptance {
    match (name, kind) {
        ("String", LiteralKind::String)
        | ("ID", LiteralKind::String | LiteralKind::Integer)
        | ("Int" | "Float", LiteralKind::Integer)
        | ("Boolean", LiteralKind::Boolean) => ScalarAcceptance::Accepted,
        ("String" | "ID" | "Int" | "Float" | "Boolean", _) => ScalarAcceptance::Rejected,
        (_, _) => ScalarAcceptance::Accepted,
    }
}

/// The errors of the object literal `object`, at `location`, of the input object `entity`.
fn object_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    scope: &Scope,
    location: Span,
    object: &isograph_parser::ObjectLiteral,
    entity: EntityName,
) -> Vec<WithSpan<GraphQLArgumentError>> {
    let Ok(Some(schema_entity)) = entity_named(db, client, entity) else {
        return Vec::new();
    };
    let fields: Vec<Expected> = input_fields(schema_entity)
        .map(|field| Expected {
            name: field.item.item.name.item.0.lookup(),
            type_: variable_type_of_graphql_type(field.item.item.type_.item.reference()),
            default: match field.item.item.default_value {
                Some(_) => DefaultValue::Present,
                None => DefaultValue::Absent,
            },
        })
        .collect();
    let entries: Vec<_> = object
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .collect();
    let missing = fields
        .iter()
        .filter(|field| {
            field.is_required()
                && !entries
                    .iter()
                    .any(|entry| entry.item.name.item.0.lookup() == field.name)
        })
        .map(|field| {
            GraphQLArgumentError::MissingInputField(InputFieldOf {
                entity,
                field: field.name.intern().to(),
            })
            .with_span(location)
        });
    let checked = entries.iter().flat_map(|entry| {
        match fields
            .iter()
            .find(|field| entry.item.name.item.0.lookup() == field.name)
        {
            None => GraphQLArgumentError::UnknownInputField(InputFieldOf {
                entity,
                field: entry.item.name.item.0,
            })
            .with_span(entry.item.name.location)
            .wrap_vec(),
            Some(field) => value_errors(
                db,
                client,
                scope,
                entry.item.value.reference(),
                field.type_.reference(),
            ),
        }
    });
    missing.chain(checked).collect()
}
