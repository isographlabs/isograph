use string_key_newtype::{string_key_newtype, string_key_one_way_conversion};

string_key_newtype!(SelectableName);

/**
 * # Aliases
 *
 * In addition, we have aliases! This would add a whole bunch of complexity (as in,
 * it's another axis, and we should have ServerObjectSelectableAlias, etc).
 *
 * However, we never convert from an alias to anything, so instead its sufficient to
 * define SelectableAlias and SelectableNameOrAlias, and have everything be convertible
 * to SelectableNameOrAlias.
 */
#[expect(dead_code)]
struct AllowDocComment2;

string_key_newtype!(SelectableAlias);
string_key_newtype!(SelectableNameOrAlias);

string_key_one_way_conversion!(from: SelectableAlias, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: SelectableName, to: SelectableNameOrAlias);
