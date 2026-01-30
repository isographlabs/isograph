use pico::{Key, Singleton, Source};
use string_key_newtype::{string_key_equality, string_key_newtype, string_key_one_way_conversion};

use crate::SelectableName;

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
string_key_newtype!(EntityName);

pub trait ExpectEntityToExist<T> {
    fn expect_entity_to_exist(self, entity_name: EntityName) -> T;
}

impl<T> ExpectEntityToExist<T> for Option<T> {
    fn expect_entity_to_exist(self, entity_name: EntityName) -> T {
        self.unwrap_or_else(|| panic!("Expected `{}` to exist.", entity_name))
    }
}

string_key_newtype!(ConstExportName);

// Operations

string_key_newtype!(QueryOperationName);
// Explanation: any client field that is on the Query object is eligible
// to be a query name.
string_key_one_way_conversion!(from: SelectableName, to: QueryOperationName);

// For scalars
string_key_newtype!(JavascriptName);

// *Not* a GraphQL directive, @component or @eager or whatnot
// This is really poorly named.
// TODO we should have different types for field directives and fragment directives
string_key_newtype!(IsographDirectiveName);

string_key_newtype!(FieldArgumentName);
string_key_equality!(FieldArgumentName, VariableName);
string_key_equality!(SelectableName, VariableName);

string_key_newtype!(ArtifactFilePrefix);
string_key_newtype!(ArtifactFileName);

string_key_newtype!(JavascriptVariableName);

string_key_newtype!(CurrentWorkingDirectory);

impl Singleton for CurrentWorkingDirectory {
    fn get_singleton_key() -> Key {
        use ::std::hash::{DefaultHasher, Hash, Hasher};
        let mut s = DefaultHasher::new();
        ::core::any::TypeId::of::<CurrentWorkingDirectory>().hash(&mut s);
        s.finish().into()
    }
}

impl Source for CurrentWorkingDirectory {
    fn get_key(&self) -> Key {
        CurrentWorkingDirectory::get_singleton_key()
    }
}

string_key_newtype!(RelativePathToSourceFile);

string_key_newtype!(IsoLiteralText);

string_key_newtype!(GeneratedFileHeader);

string_key_newtype!(OperationId);
