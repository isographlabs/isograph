//! # The Isograph object model.
//!
//! In Isograph, there are two main "axes" on which things can vary:
//! - whether they are defined locally or come from a schema, and
//! - whether they are a scalar or an object
//!
//! For example, selectable names can be client fields, client pointers, server scalar
//! fields and server object fields. (This naming is inconsistent and can be cleaned up.)
//!
//! In addition, ids and the underlying objects can be divided into these four categories.
//!
//! # Selectable names
//!
//! There are four types of selectable names:
//!        Scalar                     Object
//! Client ClientScalarSelectableName ClientObjectSelectableName
//! Server ServerScalarSelectableName ServerObjectSelectableName
//!
//! the rows are:    ClientSelectableName and ServerSelectableName
//! the columns are: ScalarSelectableName and ObjectSelectableName
//!
//! You can convert from a narrow type to a broad type using .into(), e.g.
//! ClientScalarSelectableName -> ClientSelectableName
//! ClientScalarSelectableName -> SelectableName
//!
//! The broadest object (SelectableName) corresponds to "we know nothing about this", e.g.
//! we parse SelectableNames when parsing iso literals. In addition, namespaces of available
//! fields use this as the key, since you cannot define both e.g. a scalar client field
//! and a server object field with the same name on the same type.
//!
//! As we learn about the object, or if it is parsed in a particular context (e.g. as the
//! name of a client field), we can instead define it as the more narrow type.
//!
//! Note: client selectable names include "magic" fields like link, etc. that are not
//! executed by the server. A rough guide is that a client selectable name corresponds to
//! an object that has a selection set. But confusingly, asTypename fields are server fields
//! because they are executed by the server.
//!
//! Note: for rows and columns (etc) we only define SelectionType and DefinitionLocation enums
//! for ids, but not for names. We can revisit this if we need this! But generally, the only
//! thing we need to do with names is:
//! - keep them distinguished to avoid type silly errors
//! - print them as part of artifacts or errors (in which case we don't care what their narrow
//!   type is), and
//! - use them as keys into a HashMap<SelectableName, Something>, in which case we *want* the
//!   key to not encode the narrow type, so as to prevent clashes.

use string_key_newtype::{string_key_newtype, string_key_one_way_conversion};

string_key_newtype!(SelectableName);

// Columns
string_key_newtype!(ClientSelectableName);
string_key_one_way_conversion!(from: ClientSelectableName, to: SelectableName);

string_key_newtype!(ServerSelectableName);
string_key_one_way_conversion!(from: ServerSelectableName, to: SelectableName);

// Rows
string_key_newtype!(ScalarSelectableName);
string_key_one_way_conversion!(from: ScalarSelectableName, to: SelectableName);

string_key_newtype!(ObjectSelectableName);
string_key_one_way_conversion!(from: ObjectSelectableName, to: SelectableName);

// Individual cells
string_key_newtype!(ClientScalarSelectableName);
string_key_one_way_conversion!(from: ClientScalarSelectableName, to: SelectableName);
string_key_one_way_conversion!(from: ClientScalarSelectableName, to: ClientSelectableName);
string_key_one_way_conversion!(from: ClientScalarSelectableName, to: ScalarSelectableName);

string_key_newtype!(ClientObjectSelectableName);
string_key_one_way_conversion!(from: ClientObjectSelectableName, to: SelectableName);
string_key_one_way_conversion!(from: ClientObjectSelectableName, to: ClientSelectableName);
string_key_one_way_conversion!(from: ClientObjectSelectableName, to: ObjectSelectableName);

string_key_newtype!(ServerScalarSelectableName);
string_key_one_way_conversion!(from: ServerScalarSelectableName, to: SelectableName);
string_key_one_way_conversion!(from: ServerScalarSelectableName, to: ServerSelectableName);
string_key_one_way_conversion!(from: ServerScalarSelectableName, to: ScalarSelectableName);

string_key_newtype!(ServerScalarIdSelectableName);
string_key_one_way_conversion!(from: ServerScalarIdSelectableName, to: ServerScalarSelectableName);
string_key_one_way_conversion!(from: ServerScalarIdSelectableName, to: ScalarSelectableName);

string_key_newtype!(ServerObjectSelectableName);
string_key_one_way_conversion!(from: ServerObjectSelectableName, to: SelectableName);
string_key_one_way_conversion!(from: ServerObjectSelectableName, to: ServerSelectableName);
string_key_one_way_conversion!(from: ServerObjectSelectableName, to: ObjectSelectableName);

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
#[allow(dead_code)]
struct AllowDocComment2;

string_key_newtype!(SelectableAlias);
string_key_newtype!(SelectableNameOrAlias);

string_key_one_way_conversion!(from: SelectableAlias, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: SelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ClientSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ServerSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ScalarSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ObjectSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ClientScalarSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ClientObjectSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ServerScalarSelectableName, to: SelectableNameOrAlias);
string_key_one_way_conversion!(from: ServerObjectSelectableName, to: SelectableNameOrAlias);
