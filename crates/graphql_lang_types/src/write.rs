use std::fmt;

use super::GraphQLDirective;

// This should probably be its own crate?

pub fn write_directives<T: fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    directives: &[GraphQLDirective<T>],
) -> fmt::Result {
    if directives.is_empty() {
        return Ok(());
    }

    write!(f, " ")?;
    write_list(f, directives, " ")
}

pub fn write_fields(f: &mut fmt::Formatter<'_>, fields: &[impl fmt::Display]) -> fmt::Result {
    if fields.is_empty() {
        return Ok(());
    }

    write!(f, " {{\n  ")?;
    write_list(f, fields, "\n  ")?;
    write!(f, "\n}}")
}

pub fn write_list(
    f: &mut fmt::Formatter<'_>,
    list: &[impl fmt::Display],
    separator: &str,
) -> fmt::Result {
    let v = list
        .iter()
        .map(|elem| elem.to_string())
        .collect::<Vec<String>>()
        .join(separator);
    write!(f, "{}", v)
}

pub fn write_arguments(f: &mut fmt::Formatter<'_>, arguments: &[impl fmt::Display]) -> fmt::Result {
    if arguments.is_empty() {
        return Ok(());
    }

    write!(f, "(")?;
    write_list(f, arguments, ", ")?;
    write!(f, ")")
}
