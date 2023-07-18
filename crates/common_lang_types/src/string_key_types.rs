pub use string_key_newtype::StringKeyNewtype;

use super::ValidTypeAnnotationInnerType;
use string_key_newtype::{string_key_conversion, string_key_newtype};

string_key_newtype!(DirectiveName);
string_key_newtype!(DirectiveArgumentName);

// TODO this should be "SelectionName" or something because it's not just server fields,
// it's also resolver fields.
string_key_newtype!(ServerFieldDefinitionName);

string_key_newtype!(InputValueName);
string_key_newtype!(EnumLiteralValue);
string_key_newtype!(StringLiteralValue);
string_key_newtype!(DescriptionValue);
string_key_newtype!(VariableName);
string_key_newtype!(ValueKeyName);

// OutputTypeName and InputTypeName should **only** exist on the schema parsing
// side! Later, they should be converted to some sort of enums. These represent
// unvalidated strings.
string_key_newtype!(OutputTypeName);
impl ValidTypeAnnotationInnerType for OutputTypeName {}
string_key_newtype!(InputTypeName);
impl ValidTypeAnnotationInnerType for InputTypeName {}

// A string that is supposed to be a typename (of some sort), but we haven't
// confirmed that the type exists and is the proper type yet (e.g. input, object,
// output, type with fields, etc.)
//
// Resolver parent types and field types are the only places where this should
// be used.
//
// It can also be used in error messages a sort of top type name type, i.e. any type name
// can be converted into this type name.
string_key_newtype!(UnvalidatedTypeName);
impl ValidTypeAnnotationInnerType for UnvalidatedTypeName {}
string_key_conversion!(IsographObjectTypeName, UnvalidatedTypeName);

string_key_newtype!(ObjectTypeName);
impl ValidTypeAnnotationInnerType for ObjectTypeName {}
string_key_newtype!(ScalarTypeName);
impl ValidTypeAnnotationInnerType for ScalarTypeName {}
string_key_newtype!(InterfaceTypeName);
impl ValidTypeAnnotationInnerType for InterfaceTypeName {}
string_key_newtype!(EnumTypeName);
impl ValidTypeAnnotationInnerType for EnumTypeName {}
string_key_newtype!(UnionTypeName);
impl ValidTypeAnnotationInnerType for UnionTypeName {}
string_key_newtype!(InputObjectTypeName);
impl ValidTypeAnnotationInnerType for InputObjectTypeName {}

// TODO this doesn't belong in common
// This type represents any type with fields *in the Isograph world*.
// TypeWithFields is a GraphQL concept, this is an Isograph concept.
string_key_newtype!(IsographObjectTypeName);
string_key_conversion!(ObjectTypeName, IsographObjectTypeName);
string_key_conversion!(InterfaceTypeName, IsographObjectTypeName);
string_key_conversion!(UnionTypeName, IsographObjectTypeName);

string_key_conversion!(ObjectTypeName, OutputTypeName);
string_key_conversion!(ScalarTypeName, OutputTypeName);
string_key_conversion!(InterfaceTypeName, OutputTypeName);
string_key_conversion!(EnumTypeName, OutputTypeName);
string_key_conversion!(UnionTypeName, OutputTypeName);

string_key_conversion!(ScalarTypeName, InputTypeName);
string_key_conversion!(EnumTypeName, InputTypeName);
string_key_conversion!(InputObjectTypeName, InputTypeName);

string_key_conversion!(ObjectTypeName, UnvalidatedTypeName);
string_key_conversion!(ScalarTypeName, UnvalidatedTypeName);
string_key_conversion!(InterfaceTypeName, UnvalidatedTypeName);
string_key_conversion!(EnumTypeName, UnvalidatedTypeName);
string_key_conversion!(UnionTypeName, UnvalidatedTypeName);
string_key_conversion!(InputObjectTypeName, UnvalidatedTypeName);
string_key_conversion!(OutputTypeName, UnvalidatedTypeName);
string_key_conversion!(InputTypeName, UnvalidatedTypeName);

// The name in the schema of the field
string_key_newtype!(ScalarFieldName);
string_key_conversion!(ScalarFieldName, ServerFieldDefinitionName);

string_key_newtype!(LinkedFieldName);
string_key_conversion!(LinkedFieldName, ServerFieldDefinitionName);

string_key_newtype!(ScalarFieldAlias);
string_key_newtype!(LinkedFieldAlias);

string_key_newtype!(FieldNameOrAlias);
string_key_conversion!(ScalarFieldName, FieldNameOrAlias);
string_key_conversion!(LinkedFieldName, FieldNameOrAlias);
string_key_conversion!(ScalarFieldAlias, FieldNameOrAlias);
string_key_conversion!(LinkedFieldAlias, FieldNameOrAlias);
string_key_conversion!(ServerFieldDefinitionName, FieldNameOrAlias);

string_key_newtype!(ResolverDefinitionPath);

// Operations

string_key_newtype!(QueryOperationName);
// Explanation: any resolver field that is on the Query object is eligible
// to be a query.
string_key_conversion!(ServerFieldDefinitionName, QueryOperationName);
// The reverse is safe as well.
string_key_conversion!(QueryOperationName, ServerFieldDefinitionName);

// For scalars
string_key_newtype!(JavascriptName);
// This is getting ridiculous...
impl ValidTypeAnnotationInnerType for JavascriptName {}

string_key_newtype!(IsographDirectiveName);

string_key_newtype!(FieldArgumentName);

// e.g. Query__foo
string_key_newtype!(TypeAndField);

string_key_newtype!(NormalizationKey);
string_key_conversion!(ServerFieldDefinitionName, NormalizationKey);
