use super::TypeTrait;
use string_key_newtype::string_key_newtype;

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
impl TypeTrait for OutputTypeName {}
string_key_newtype!(InputTypeName);
impl TypeTrait for InputTypeName {}

string_key_newtype!(ObjectTypeName);
string_key_newtype!(InterfaceTypeName);

string_key_newtype!(ScalarFieldName);
string_key_newtype!(ScalarFieldAlias);
string_key_newtype!(LinkedFieldName);
string_key_newtype!(LinkedFieldAlias);
