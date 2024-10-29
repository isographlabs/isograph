pub use string_key_newtype::StringKeyNewtype;
use string_key_newtype::{string_key_conversion, string_key_newtype};

string_key_newtype!(DirectiveName);
string_key_newtype!(DirectiveArgumentName);

// This is an object in the namespace of selectable fields, meaning it can be:
// - a server-defined field or a client-defined field
// - a scalar field or a linked field
// (client-defined linked fields do not exist, but will.)
string_key_newtype!(SelectableFieldName);

string_key_newtype!(InputValueName);
string_key_conversion!(from: InputValueName, to: VariableName);
string_key_conversion!(from: InputValueName, to: FieldArgumentName);

string_key_newtype!(EnumLiteralValue);
string_key_newtype!(StringLiteralValue);
string_key_newtype!(DescriptionValue);
string_key_newtype!(VariableName);
string_key_newtype!(ValueKeyName);

// OutputTypeName and InputTypeName should **only** exist on the schema parsing
// side! Later, they should be converted to some sort of enums. These represent
// unvalidated strings.
string_key_newtype!(OutputTypeName);
string_key_newtype!(InputTypeName);

// A string that is supposed to be a typename (of some sort), but we haven't
// confirmed that the type exists and is the proper type yet (e.g. input, object,
// output, type with fields, etc.)
//
// Client field parent types and field types are the only places where this should
// be used.
//
// It can also be used in error messages a sort of top type name type, i.e. any type name
// can be converted into this type name.
string_key_newtype!(UnvalidatedTypeName);
string_key_conversion!(from: IsographObjectTypeName, to: UnvalidatedTypeName);

string_key_newtype!(GraphQLObjectTypeName);
string_key_newtype!(GraphQLScalarTypeName);
string_key_newtype!(GraphQLInterfaceTypeName);
string_key_newtype!(GraphQLEnumTypeName);
string_key_newtype!(GraphQLUnionTypeName);
string_key_newtype!(GraphQLInputObjectTypeName);

// TODO this doesn't belong in common
// This type represents any type with fields *in the Isograph world*.
// TypeWithFields is a GraphQL concept, this is an Isograph concept.
string_key_newtype!(IsographObjectTypeName);
string_key_conversion!(from: GraphQLObjectTypeName, to: IsographObjectTypeName);
string_key_conversion!(from: GraphQLInterfaceTypeName, to: IsographObjectTypeName);
string_key_conversion!(from: GraphQLUnionTypeName, to: IsographObjectTypeName);

string_key_conversion!(from: GraphQLObjectTypeName, to: OutputTypeName);
string_key_conversion!(from: GraphQLScalarTypeName, to: OutputTypeName);
string_key_conversion!(from: GraphQLInterfaceTypeName, to: OutputTypeName);
string_key_conversion!(from: GraphQLEnumTypeName, to: OutputTypeName);
string_key_conversion!(from: GraphQLUnionTypeName, to: OutputTypeName);

string_key_conversion!(from: GraphQLScalarTypeName, to: InputTypeName);
string_key_conversion!(from: GraphQLEnumTypeName, to: InputTypeName);
string_key_conversion!(from: GraphQLInputObjectTypeName, to: InputTypeName);

string_key_conversion!(from: GraphQLObjectTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: GraphQLScalarTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: GraphQLInterfaceTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: GraphQLEnumTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: GraphQLUnionTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: GraphQLInputObjectTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: OutputTypeName, to: UnvalidatedTypeName);
string_key_conversion!(from: InputTypeName, to: UnvalidatedTypeName);

// The name in the schema of the field
string_key_newtype!(ScalarFieldName);
string_key_conversion!(from: ScalarFieldName, to: SelectableFieldName);

string_key_newtype!(LinkedFieldName);
string_key_conversion!(from: LinkedFieldName, to: SelectableFieldName);

string_key_newtype!(ScalarFieldAlias);
string_key_newtype!(LinkedFieldAlias);

string_key_newtype!(FieldNameOrAlias);
string_key_conversion!(from: ScalarFieldName, to: FieldNameOrAlias);
string_key_conversion!(from: LinkedFieldName, to: FieldNameOrAlias);
string_key_conversion!(from: ScalarFieldAlias, to: FieldNameOrAlias);
string_key_conversion!(from: LinkedFieldAlias, to: FieldNameOrAlias);
string_key_conversion!(from: SelectableFieldName, to: FieldNameOrAlias);

string_key_newtype!(FilePath);
string_key_newtype!(ConstExportName);

// Operations

string_key_newtype!(QueryOperationName);
// Explanation: any client field that is on the Query object is eligible
// to be a query.
string_key_conversion!(from: SelectableFieldName, to: QueryOperationName);
// The reverse is safe as well.
string_key_conversion!(from: QueryOperationName, to: SelectableFieldName);

// For scalars
string_key_newtype!(JavascriptName);

// *Not* a GraphQL directive, @component or @eager or whatnot
// This is really poorly named.
// TODO we should have different types for field directives and fragment directives
string_key_newtype!(IsographDirectiveName);

string_key_newtype!(FieldArgumentName);

string_key_newtype!(SourceFileName);

string_key_newtype!(ArtifactFileType);

string_key_newtype!(JavascriptVariableName);
