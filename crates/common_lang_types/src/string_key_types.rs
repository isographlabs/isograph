use string_key_newtype::{
    string_key_equality, string_key_newtype, string_key_newtype_no_display,
    string_key_one_way_conversion,
};

use crate::{ClientScalarSelectableName, SelectableName};

string_key_newtype!(DirectiveName);
string_key_newtype!(DirectiveArgumentName);

string_key_newtype!(InputValueName);
string_key_one_way_conversion!(from: InputValueName, to: VariableName);
string_key_one_way_conversion!(from: InputValueName, to: FieldArgumentName);

string_key_newtype!(EnumLiteralValue);
string_key_newtype!(StringLiteralValue);
string_key_equality!(StringLiteralValue, VariableName);
string_key_equality!(StringLiteralValue, SelectableName);
string_key_newtype!(DescriptionValue);
string_key_newtype!(VariableName);
string_key_newtype!(ValueKeyName);
string_key_equality!(ValueKeyName, SelectableName);

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
string_key_one_way_conversion!(from: ServerObjectEntityName, to: UnvalidatedTypeName);

string_key_newtype!(GraphQLObjectTypeName);
// TODO there should also be a GraphQL scalar name that can be converted to this
string_key_newtype!(ServerScalarEntityName);
string_key_newtype!(GraphQLInterfaceTypeName);
string_key_newtype!(GraphQLEnumTypeName);
string_key_newtype!(GraphQLUnionTypeName);
string_key_newtype!(GraphQLInputObjectTypeName);

// TODO this doesn't belong in common
// This type represents any type with fields *in the Isograph world*.
// TypeWithFields is a GraphQL concept, this is an Isograph concept.
string_key_newtype!(ServerObjectEntityName);
string_key_one_way_conversion!(from: GraphQLObjectTypeName, to: ServerObjectEntityName);
string_key_one_way_conversion!(from: GraphQLInterfaceTypeName, to: ServerObjectEntityName);
string_key_one_way_conversion!(from: GraphQLUnionTypeName, to: ServerObjectEntityName);

string_key_one_way_conversion!(from: GraphQLObjectTypeName, to: OutputTypeName);
string_key_one_way_conversion!(from: ServerScalarEntityName, to: OutputTypeName);
string_key_one_way_conversion!(from: GraphQLInterfaceTypeName, to: OutputTypeName);
string_key_one_way_conversion!(from: GraphQLEnumTypeName, to: OutputTypeName);
string_key_one_way_conversion!(from: GraphQLUnionTypeName, to: OutputTypeName);

string_key_one_way_conversion!(from: ServerScalarEntityName, to: InputTypeName);
string_key_one_way_conversion!(from: GraphQLEnumTypeName, to: InputTypeName);
string_key_one_way_conversion!(from: GraphQLInputObjectTypeName, to: InputTypeName);

string_key_one_way_conversion!(from: GraphQLObjectTypeName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: ServerScalarEntityName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: GraphQLInterfaceTypeName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: GraphQLEnumTypeName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: GraphQLUnionTypeName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: GraphQLInputObjectTypeName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: OutputTypeName, to: UnvalidatedTypeName);
string_key_one_way_conversion!(from: InputTypeName, to: UnvalidatedTypeName);

string_key_newtype!(ConstExportName);

// Operations

string_key_newtype!(QueryOperationName);
// Explanation: any client field that is on the Query object is eligible
// to be a query name.
string_key_one_way_conversion!(from: ClientScalarSelectableName, to: QueryOperationName);
string_key_one_way_conversion!(from: QueryOperationName, to: SelectableName);

// For scalars
string_key_newtype!(JavascriptName);

// *Not* a GraphQL directive, @component or @eager or whatnot
// This is really poorly named.
// TODO we should have different types for field directives and fragment directives
string_key_newtype!(IsographDirectiveName);

string_key_newtype!(FieldArgumentName);
string_key_equality!(FieldArgumentName, VariableName);

string_key_newtype!(ArtifactFilePrefix);
string_key_newtype!(ArtifactFileName);

string_key_newtype!(JavascriptVariableName);

// HACK:
// Locations contain two paths:
// - The absolute path to where the compiler is executed (i.e. the current
//   working directory)
// - The relative path from the working directory to the source file
//
// These are separate paths because the CurrentWorkingDirectory has a
// std::fmt::Display impl that prints the hard-coded string
// "CurrentWorkingDirectory", so that generated fixtures (which include
// debug output of the CurrentWorkingDirectory) are consistent when we
// generated from multiple machines (including on arbitrary CI machines).
//
// We print the relative path in error messages, but use the full path
// (i.e. cwd + relative path) for reading the content of the source file
// for printing errors.
string_key_newtype_no_display!(CurrentWorkingDirectory);
impl std::fmt::Display for CurrentWorkingDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CurrentWorkingDirectory")
    }
}
impl std::fmt::Debug for CurrentWorkingDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CurrentWorkingDirectory")
    }
}

string_key_newtype!(RelativePathToSourceFile);

string_key_newtype!(IsoLiteralText);

string_key_newtype!(GeneratedFileHeader);
