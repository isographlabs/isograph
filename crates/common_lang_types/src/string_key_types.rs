use crate::ValidScalarFieldType;

use super::ValidTypeAnnotationInnerType;
use string_key_newtype::{string_key_conversion, string_key_newtype};

string_key_newtype!(DirectiveName);
string_key_newtype!(DirectiveArgumentName);

string_key_newtype!(FieldDefinitionName);

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
string_key_newtype!(UnvalidatedTypeName);
impl ValidTypeAnnotationInnerType for UnvalidatedTypeName {}

string_key_newtype!(TypeWithFieldsName);
string_key_newtype!(TypeWithoutFieldsName);

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

string_key_conversion!(ObjectTypeName, OutputTypeName);
string_key_conversion!(ScalarTypeName, OutputTypeName);
string_key_conversion!(InterfaceTypeName, OutputTypeName);
string_key_conversion!(EnumTypeName, OutputTypeName);
string_key_conversion!(UnionTypeName, OutputTypeName);
string_key_conversion!(TypeWithFieldsName, OutputTypeName);
impl ValidTypeAnnotationInnerType for TypeWithFieldsName {}

string_key_conversion!(ScalarTypeName, InputTypeName);
string_key_conversion!(EnumTypeName, InputTypeName);
string_key_conversion!(InputObjectTypeName, InputTypeName);

string_key_conversion!(ObjectTypeName, TypeWithFieldsName);
string_key_conversion!(InterfaceTypeName, TypeWithFieldsName);

string_key_conversion!(ObjectTypeName, UnvalidatedTypeName);
string_key_conversion!(ScalarTypeName, UnvalidatedTypeName);
string_key_conversion!(InterfaceTypeName, UnvalidatedTypeName);
string_key_conversion!(EnumTypeName, UnvalidatedTypeName);
string_key_conversion!(UnionTypeName, UnvalidatedTypeName);
string_key_conversion!(InputObjectTypeName, UnvalidatedTypeName);
string_key_conversion!(OutputTypeName, UnvalidatedTypeName);
string_key_conversion!(InputTypeName, UnvalidatedTypeName);
string_key_conversion!(TypeWithFieldsName, UnvalidatedTypeName);

string_key_conversion!(ScalarTypeName, TypeWithoutFieldsName);
string_key_conversion!(EnumTypeName, TypeWithoutFieldsName);

// The name in the schema of the field
string_key_newtype!(ScalarFieldName);
impl ValidScalarFieldType for ScalarFieldName {}
string_key_conversion!(ScalarFieldName, FieldDefinitionName);

string_key_newtype!(LinkedFieldName);
string_key_conversion!(LinkedFieldName, FieldDefinitionName);

string_key_newtype!(ScalarFieldAlias);
string_key_newtype!(LinkedFieldAlias);

string_key_newtype!(FieldNameOrAlias);
string_key_conversion!(ScalarFieldName, FieldNameOrAlias);
string_key_conversion!(LinkedFieldName, FieldNameOrAlias);
string_key_conversion!(ScalarFieldAlias, FieldNameOrAlias);
string_key_conversion!(LinkedFieldAlias, FieldNameOrAlias);

string_key_newtype!(ResolverDefinitionPath);

// Operations

string_key_newtype!(QueryOperationName);
// Explanation: any resolver field that is on the Query object is eligible
// to be a query.
string_key_conversion!(FieldDefinitionName, QueryOperationName);
