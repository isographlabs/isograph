use std::marker::PhantomData;

use common_lang_types::{
    DirectiveArgumentName, DirectiveName, IsographObjectTypeName, Location, SelectableFieldName,
    Span, StringLiteralValue, ValueKeyName, WithEmbeddedLocation, WithLocation, WithSpan,
};
use graphql_lang_types::{
    ConstantValue, GraphQLDirective, GraphQLInputValueDefinition, NameValuePair, ValueType,
};
use intern::{string_key::Intern, Lookup};
use isograph_config::ConfigOptions;
use isograph_lang_types::{
    ClientFieldId, DefinedTypeId, ObjectId, ScalarFieldSelection, Selection, ServerFieldId,
    ServerFieldSelection,
};
use serde::de::value::SeqDeserializer;
use serde::de::MapAccess;
use serde::de::{self, IntoDeserializer};
use serde::Deserialize;
use serde::{self, Deserializer};

use crate::{
    ArgumentMap, DefinedField, FieldMapItem, MutationFieldResolverActionKindInfo,
    MutationFieldResolverVariant, ProcessTypeDefinitionError, ProcessTypeDefinitionResult,
    ProcessedFieldMapItem, ResolverActionKind, ResolverTypeAndField, ResolverVariant,
    SchemaResolver, UnvalidatedSchema,
};
use lazy_static::lazy_static;

lazy_static! {
    static ref EXPOSE_FIELD_DIRECTIVE: DirectiveName = "exposeField".intern().into();
    static ref PATH_DIRECTIVE_ARGUMENT: DirectiveArgumentName = "path".intern().into();
    static ref FIELD_MAP_DIRECTIVE_ARGUMENT: DirectiveArgumentName = "field_map".intern().into();
    static ref FIELD_DIRECTIVE_ARGUMENT: DirectiveArgumentName = "field".intern().into();
    static ref FROM_VALUE_KEY_NAME: ValueKeyName = "from".intern().into();
    static ref TO_VALUE_KEY_NAME: ValueKeyName = "to".intern().into();
}
#[derive(Deserialize, Eq, PartialEq, Debug)]
pub struct MagicMutationFieldInfo {
    #[serde(deserialize_with = "deserialize_stringliteral")]
    path: StringLiteralValue,
    field_map: Vec<FieldMapItem>,
    #[serde(deserialize_with = "deserialize_stringliteral")]
    field: StringLiteralValue,
}

fn deserialize_stringliteral<'de, D>(deserializer: D) -> Result<StringLiteralValue, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(StringLiteralValue::from(value.intern()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiresRefinement {
    Yes(IsographObjectTypeName),
    No,
}

impl UnvalidatedSchema {
    /// Add magical mutation fields.
    ///
    /// Using the MagicMutationFieldInfo (derived from @exposeField directives),
    /// add a magical field to TargetType whose name is the mutation_name, which:
    /// - executes the mutation
    /// - has the mutation's arguments (except those from field_map)
    /// - then acts as a __refetch field on that TargetType, i.e. refetches all the fields
    ///   selected in the merged selection set.
    ///
    /// There is lots of cloning going on here! Not ideal.
    pub fn create_magic_mutation_fields(
        &mut self,
        mutation_id: ObjectId,
        options: ConfigOptions,
    ) -> ProcessTypeDefinitionResult<()> {
        // TODO don't clone if possible
        let mutation_object = self.schema_data.object(mutation_id);
        let mutation_object_name = mutation_object.name;

        // TODO this is a bit ridiculous
        let magic_mutation_infos = mutation_object
            .directives
            .iter()
            .map(|d| self.extract_magic_mutation_field_info(d))
            .collect::<Result<Vec<_>, _>>()?;
        let magic_mutation_infos = magic_mutation_infos
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        for magic_mutation_info in magic_mutation_infos.iter() {
            self.create_new_magic_mutation_field(
                magic_mutation_info,
                mutation_object_name,
                mutation_id,
                options,
            )?;
        }

        Ok(())
    }

    fn create_new_magic_mutation_field(
        &mut self,
        magic_mutation_info: &MagicMutationFieldInfo,
        mutation_object_name: IsographObjectTypeName,
        mutation_id: ObjectId,
        options: ConfigOptions,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let MagicMutationFieldInfo {
            path,
            field_map,
            field,
        } = magic_mutation_info;

        let field_id = self.parse_field(*field, mutation_id)?;

        let mutation_field = self.field(field_id);
        let mutation_field_payload_type_name = *mutation_field.associated_data.inner();
        let mutation_field_name = mutation_field.name.item;
        let mutation_field_arguments = mutation_field.arguments.clone();
        let description = mutation_field.description.clone();
        let payload_id = self
            .schema_data
            .defined_types
            .get(&mutation_field_payload_type_name)
            .map(|x| *x);

        if let Some(DefinedTypeId::Object(mutation_field_object_id)) = payload_id {
            let (mutation_field_args_without_id, processed_field_map_items) =
                skip_arguments_contained_in_field_map(
                    self,
                    mutation_field_arguments.clone(),
                    // TODO make this a no-op
                    mutation_field_payload_type_name.lookup().intern().into(),
                    mutation_object_name,
                    mutation_field_name,
                    // TODO don't clone
                    field_map.clone(),
                    options,
                )?;

            // TODO this is dangerous! mutation_field.name is also formattable (with carats).
            // We should find a way to make WithLocation not impl Display, while also making
            // errors containing WithLocation<...> easy to work with.
            // TODO "expose as" optional field
            let magic_mutation_field_name = mutation_field_name;

            // payload object is the object type of the mutation field, e.g. SetBestFriendResponse
            let payload_object = self.schema_data.object(mutation_field_object_id);
            let payload_object_name = payload_object.name;

            // TODO make this zero cost
            // TODO split path on .
            let path_selectable_field_name = path.lookup().intern().into();

            let primary_field = payload_object
                .encountered_fields
                .get(&path_selectable_field_name);

            let (maybe_abstract_parent_object_id, maybe_abstract_parent_type_name) =
                match primary_field {
                    Some(DefinedField::ServerField(server_field)) => {
                        // This is the parent type name (Pet)
                        let inner = server_field.inner();

                        // TODO validate that the payload object has no plural fields in between

                        let primary_type = self.schema_data.defined_types.get(inner).clone();

                        if let Some(DefinedTypeId::Object(resolver_parent_object_id)) = primary_type
                        {
                            Ok((*resolver_parent_object_id, *inner))
                        } else {
                            Err(WithLocation::new(
                                ProcessTypeDefinitionError::InvalidMutationField,
                                Location::generated(),
                            ))
                        }
                    }
                    _ => Err(WithLocation::new(
                        ProcessTypeDefinitionError::InvalidMutationField,
                        Location::generated(),
                    )),
                }?;

            let fields = processed_field_map_items
                .iter()
                .map(|field_map_item| {
                    let scalar_field_selection = ScalarFieldSelection {
                        name: WithLocation::new(
                            // TODO make this no-op
                            // TODO split on . here; we should be able to have from: "best_friend.id" or whatnot.
                            field_map_item.0.from.lookup().intern().into(),
                            Location::generated(),
                        ),
                        reader_alias: None,
                        normalization_alias: None,
                        associated_data: (),
                        unwraps: vec![],
                        // TODO what about arguments? How would we handle them?
                        arguments: vec![],
                    };

                    WithSpan::new(
                        Selection::ServerField(ServerFieldSelection::ScalarField(
                            scalar_field_selection,
                        )),
                        Span::todo_generated(),
                    )
                })
                .collect::<Vec<_>>();

            let magic_mutation_field_resolver_id = self.resolvers.len().into();
            let magic_mutation_field_resolver = SchemaResolver {
                description,
                // set_pet_best_friend
                name: magic_mutation_field_name,
                id: magic_mutation_field_resolver_id,
                selection_set_and_unwraps: Some((fields.to_vec(), vec![])),
                variant: ResolverVariant::MutationField(MutationFieldResolverVariant {
                    mutation_field_name: magic_mutation_field_name,
                    mutation_primary_field_name: path_selectable_field_name,
                    mutation_field_arguments: mutation_field_arguments.to_vec(),
                    filtered_mutation_field_arguments: mutation_field_args_without_id.to_vec(),
                    mutation_primary_field_return_type_object_id: maybe_abstract_parent_object_id,
                }),
                variable_definitions: vec![],
                type_and_field: ResolverTypeAndField {
                    // TODO make this zero cost?
                    type_name: maybe_abstract_parent_type_name.lookup().intern().into(), // e.g. Pet
                    field_name: magic_mutation_field_name, // set_pet_best_friend
                },
                parent_object_id: maybe_abstract_parent_object_id,
                action_kind: ResolverActionKind::MutationField(
                    MutationFieldResolverActionKindInfo {
                        // TODO don't clone
                        field_map: field_map.to_vec(),
                    },
                ),
            };
            self.resolvers.push(magic_mutation_field_resolver);

            self.insert_resolver_field_on_object(
                magic_mutation_field_name,
                maybe_abstract_parent_object_id,
                magic_mutation_field_resolver_id,
                payload_object_name,
            )?;
        }
        Ok(())
    }

    // TODO this should be defined elsewhere, probably
    pub fn insert_resolver_field_on_object(
        &mut self,
        magic_mutation_field_name: SelectableFieldName,
        resolver_parent_object_id: ObjectId,
        resolver_id: ClientFieldId,
        payload_object_name: IsographObjectTypeName,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let resolver_parent = self.schema_data.object_mut(resolver_parent_object_id);
        if resolver_parent
            .encountered_fields
            .insert(
                magic_mutation_field_name,
                DefinedField::ResolverField(resolver_id),
            )
            .is_some()
        {
            return Err(WithLocation::new(
                // TODO use a more generic error message when making this
                ProcessTypeDefinitionError::FieldExistsOnSubtype {
                    field_name: magic_mutation_field_name,
                    parent_type: payload_object_name,
                },
                // TODO this is blatantly incorrect
                Location::generated(),
            ));
        }
        resolver_parent.resolvers.push(resolver_id);

        Ok(())
    }

    fn extract_magic_mutation_field_info(
        &self,
        d: &GraphQLDirective<ConstantValue>,
    ) -> ProcessTypeDefinitionResult<Option<MagicMutationFieldInfo>> {
        if d.name.item == *EXPOSE_FIELD_DIRECTIVE {
            Ok(Some(self.validate_magic_mutation_directive(d)?))
        } else {
            Ok(None)
        }
    }

    fn validate_magic_mutation_directive(
        &self,
        d: &GraphQLDirective<ConstantValue>,
    ) -> ProcessTypeDefinitionResult<MagicMutationFieldInfo> {
        if d.arguments.len() != 3 {
            return Err(WithEmbeddedLocation::new(
                ProcessTypeDefinitionError::InvalidPrimaryDirectiveArgumentCount,
                // This is wrong, the arguments should have a span, or the whole thing should have a span
                d.name.location,
            )
            .into_with_location());
        }

        let path = d
            .arguments
            .iter()
            .find(|d| d.name.item == *PATH_DIRECTIVE_ARGUMENT)
            .ok_or_else(|| {
                WithEmbeddedLocation::new(
                    ProcessTypeDefinitionError::MissingPathArg,
                    // This is wrong, the arguments should have a span, or the whole thing should have a span
                    d.name.location,
                )
                .into_with_location()
            })?;
        let path_val = match path.value.item {
            ConstantValue::String(s) => Ok(s),
            _ => Err(WithEmbeddedLocation::new(
                ProcessTypeDefinitionError::PathValueShouldBeString,
                // This is wrong, the arguments should have a span, or the whole thing should have a span
                d.name.location,
            )
            .into_with_location()),
        }?;

        let field_map = d
            .arguments
            .iter()
            .find(|d| d.name.item == *FIELD_MAP_DIRECTIVE_ARGUMENT)
            .ok_or_else(|| {
                WithEmbeddedLocation::new(
                    ProcessTypeDefinitionError::MissingFieldMapArg,
                    // This is wrong, the arguments should have a span, or the whole thing should have a span
                    d.name.location,
                )
                .into_with_location()
            })?;

        let field_map = parse_field_map_val(&field_map.value)?;

        let field_argument = d
            .arguments
            .iter()
            .find(|d| d.name.item == *FIELD_DIRECTIVE_ARGUMENT)
            .ok_or_else(|| {
                WithEmbeddedLocation::new(
                    ProcessTypeDefinitionError::MissingFieldMapArg,
                    // This is wrong, the arguments should have a span, or the whole thing should have a span
                    d.name.location,
                )
                .into_with_location()
            })?;

        let field = field_argument.value.item.as_string().ok_or_else(|| {
            WithLocation::new(
                ProcessTypeDefinitionError::InvalidField,
                Location::generated(),
            )
        })?;

        Ok(MagicMutationFieldInfo {
            path: path_val,
            field_map,
            field,
        })
    }

    fn parse_field(
        &self,
        field_arg: StringLiteralValue,
        mutation_id: ObjectId,
    ) -> ProcessTypeDefinitionResult<ServerFieldId> {
        let mutation = self.schema_data.object(mutation_id);

        // TODO make this a no-op
        let field_arg = field_arg.lookup().intern().into();

        // TODO avoid a linear scan?
        let field_id = mutation
            .server_fields
            .iter()
            .find_map(|field_id| {
                let server_field = self.field(*field_id);
                if server_field.name.item == field_arg {
                    Some(*field_id)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::InvalidField,
                    // TODO
                    Location::generated(),
                )
            })?;

        Ok(field_id)
    }
}

fn parse_field_map_val(
    value: &WithLocation<ConstantValue>,
) -> ProcessTypeDefinitionResult<Vec<FieldMapItem>> {
    let list = match &value.item {
        ConstantValue::List(l) => Ok(l),
        _ => Err(WithLocation::new(
            ProcessTypeDefinitionError::InvalidFieldMap,
            // This is wrong, the arguments should have a span, or the whole thing should have a span
            value.location,
        )),
    }?;

    list.iter()
        .map(|argument_value| {
            let object = match &argument_value.item {
                ConstantValue::Object(o) => Ok(o),
                _ => Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                )),
            }?;

            if object.len() != 2 {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                ));
            }

            let from = object
                .iter()
                .find(|d| d.name.item == *FROM_VALUE_KEY_NAME)
                .ok_or_else(|| {
                    WithLocation::new(
                        ProcessTypeDefinitionError::InvalidFieldMap,
                        // This is wrong, the arguments should have a span, or the whole thing should have a span
                        argument_value.location,
                    )
                })?;

            let from_arg = match from.value.item {
                ConstantValue::String(s) => Ok(s),
                _ => Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                )),
            }?;

            let to = object
                .iter()
                .find(|d| d.name.item == *TO_VALUE_KEY_NAME)
                .ok_or_else(|| {
                    WithLocation::new(
                        ProcessTypeDefinitionError::InvalidFieldMap,
                        // This is wrong, the arguments should have a span, or the whole thing should have a span
                        argument_value.location,
                    )
                })?;

            let to_arg = to.value.item.as_string().ok_or_else(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                )
            })?;

            Ok(FieldMapItem {
                from: from_arg,
                to: to_arg,
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

fn skip_arguments_contained_in_field_map(
    // TODO move this to impl Schema
    schema: &mut UnvalidatedSchema,
    arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    primary_type_name: IsographObjectTypeName,
    mutation_object_name: IsographObjectTypeName,
    mutation_field_name: SelectableFieldName,
    field_map_items: Vec<FieldMapItem>,
    options: ConfigOptions,
) -> ProcessTypeDefinitionResult<(
    Vec<WithLocation<GraphQLInputValueDefinition>>,
    Vec<ProcessedFieldMapItem>,
)> {
    let mut processed_field_map_items = Vec::with_capacity(field_map_items.len());
    // TODO
    // We need to create entirely new arguments, which are the existing arguments minus
    // any paths that are in the field map.
    let mut argument_map = ArgumentMap::new(arguments);

    for field_map_item in field_map_items {
        processed_field_map_items.push(argument_map.remove_field_map_item(
            field_map_item,
            primary_type_name,
            mutation_object_name,
            mutation_field_name,
            schema,
        )?);
    }

    Ok((
        argument_map.into_arguments(schema, options),
        processed_field_map_items,
    ))
}

#[derive(Debug)]
struct GraphQLDirectiveDeserializer<'a> {
    directive: &'a GraphQLDirective<ConstantValue>,
}
fn from_graph_ql_directive<'a, T: Deserialize<'a>>(
    directive: &'a GraphQLDirective<ConstantValue>,
) -> ProcessTypeDefinitionResult<T> {
    T::deserialize(GraphQLDirectiveDeserializer { directive })
        .map_err(|err| WithLocation::new(err, Location::generated()))
}

impl de::Error for ProcessTypeDefinitionError {
    fn custom<T>(msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        ProcessTypeDefinitionError::FailedToDeserialize(msg.to_string())
    }
}

impl<'a, 'de> Deserializer<'de> for GraphQLDirectiveDeserializer<'a> {
    type Error = ProcessTypeDefinitionError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(NameValuePairVec::new(&self.directive.arguments))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct NameValuePairVec<'a, T> {
    arguments: &'a Vec<NameValuePair<T, ConstantValue>>,
    field_idx: usize,
}

impl<'a, T> NameValuePairVec<'a, T> {
    fn new(args: &'a Vec<NameValuePair<T, ConstantValue>>) -> Self {
        NameValuePairVec {
            arguments: args,
            field_idx: 0,
        }
    }
}

impl<'a, 'de, T: ToString> MapAccess<'de> for NameValuePairVec<'a, T> {
    type Error = ProcessTypeDefinitionError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        print!("Deserializing field at idx {}", self.field_idx);
        if let Some(name_value_pair) = self.arguments.get(self.field_idx) {
            return seed
                .deserialize(NameSerializer { name_value_pair })
                .map(Some);
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.arguments.get(self.field_idx) {
            Some(name_value_pair) => {
                self.field_idx += 1;
                seed.deserialize(ValueSerializer { name_value_pair })
            }
            _ => Err(ProcessTypeDefinitionError::FailedToDeserialize(format!(
                "Called deserialization of field value for a field with idx {} that doesn't exist",
                self.field_idx
            ))),
        }
    }
}

struct NameSerializer<'a, TName, TValue: ValueType> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

struct ValueSerializer<'a, TName, TValue: ValueType> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

impl<'a, 'de, TName: ToString, TValue: ValueType> Deserializer<'de>
    for NameSerializer<'a, TName, TValue>
{
    type Error = ProcessTypeDefinitionError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_string(self.name_value_pair.name.item.to_string())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_string(self.name_value_pair.name.item.to_string())
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }
}

pub struct ConstantValueDeserializer<'de> {
    value: ConstantValue,
    marker: PhantomData<&'de ()>,
}

impl<'de> IntoDeserializer<'de, ProcessTypeDefinitionError> for ConstantValue {
    type Deserializer = ConstantValueDeserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        ConstantValueDeserializer {
            value: self,
            marker: PhantomData,
        }
    }
}

impl<'a, 'de> Deserializer<'de> for ConstantValueDeserializer<'de> {
    type Error = ProcessTypeDefinitionError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ConstantValue::Boolean(bool) => visitor.visit_bool(bool),
            ConstantValue::Enum(enum_literal) => visitor.visit_string(enum_literal.to_string()),
            ConstantValue::Float(float_value) => visitor.visit_f64(float_value.as_float()),
            ConstantValue::Int(i_64) => visitor.visit_i64(i_64),
            ConstantValue::String(string) => visitor.visit_string(string.to_string()),
            ConstantValue::Null => visitor.visit_none(),
            ConstantValue::List(seq) => {
                let values: Vec<ConstantValue> = seq.into_iter().map(|entry| entry.item).collect();
                let seq_access = SeqDeserializer::new(values.into_iter());
                visitor.visit_seq(seq_access)
            }
            ConstantValue::Object(obj) => {
                let serializer = NameValuePairVec::new(&obj);
                visitor.visit_map(serializer)
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

impl<'a, 'de, TName> Deserializer<'de> for ValueSerializer<'a, TName, ConstantValue> {
    type Error = ProcessTypeDefinitionError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let deserializer = ConstantValueDeserializer {
            value: self.name_value_pair.value.item.clone(),
            marker: PhantomData,
        };
        deserializer.deserialize_any(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use common_lang_types::TextSource;
    use graphql_lang_types::{
        ConstantValue, GraphQLDirective, GraphQLTypeSystemExtension,
        GraphQLTypeSystemExtensionOrDefinition,
    };
    use graphql_schema_parser::*;
    use intern::string_key::Intern;
    use std::error::Error;

    fn unwrap_directive(
        extension_or_definition: GraphQLTypeSystemExtensionOrDefinition,
    ) -> Result<Vec<GraphQLDirective<ConstantValue>>, Box<dyn Error>> {
        if let GraphQLTypeSystemExtensionOrDefinition::Extension(extension) =
            extension_or_definition
        {
            let GraphQLTypeSystemExtension::ObjectTypeExtension(object_type_extension) = extension;
            return Ok(object_type_extension.directives.clone());
        }
        Err("unexpected structure of directive".into())
    }

    #[test]
    fn test_magic_mutation_deserialization() -> Result<(), Box<dyn Error>> {
        let source = "extend type Mutation
        @exposeField(
          field: \"set_pet_tagline\"
          path: \"pet\"
          field_map: [{ from: \"id\", to: \"input.id\" }]
        )
        @exposeField(
          field: \"set_pet_best_friend\"
          path: \"pet\"
          field_map: [{ from: \"id\", to: \"id\" }]
        )";
        let text_source = TextSource {
            path: "dummy".intern().into(),
            span: None,
        };
        let document = parse_schema_extensions(source, text_source).map_err(|e| e.item)?;
        let directives = document
            .0
            .into_iter()
            .map(|dir| unwrap_directive(dir.item))
            .collect::<Result<Vec<_>, _>>()?;
        let directives: Vec<GraphQLDirective<ConstantValue>> =
            directives.into_iter().flatten().collect();

        let magic_mutations: Result<Vec<MagicMutationFieldInfo>, _> = directives
            .into_iter()
            .map(|directive| from_graph_ql_directive::<MagicMutationFieldInfo>(&directive))
            .collect();
        let magic_mutations = magic_mutations?;
        let set_tagline_mutation = MagicMutationFieldInfo {
            path: StringLiteralValue::from("pet".intern()),
            field_map: vec![FieldMapItem {
                from: StringLiteralValue::from("id".intern()),
                to: StringLiteralValue::from("input.id".intern()),
            }],
            field: StringLiteralValue::from("set_pet_tagline".intern()),
        };
        let set_pet_best_friend = MagicMutationFieldInfo {
            path: StringLiteralValue::from("pet".intern()),
            field_map: vec![FieldMapItem {
                from: StringLiteralValue::from("id".intern()),
                to: StringLiteralValue::from("id".intern()),
            }],
            field: StringLiteralValue::from("set_pet_best_friend".intern()),
        };
        assert_eq!(magic_mutations[0], set_tagline_mutation);
        assert_eq!(magic_mutations[1], set_pet_best_friend);
        Ok(())
    }
}
