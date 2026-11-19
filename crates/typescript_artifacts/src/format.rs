//! Formats the text `swc::print` writes with dprint.

use std::path::Path;

use dprint_plugin_typescript::configuration::{
    Configuration, ConfigurationBuilder, OperatorPosition, QuoteProps, QuoteStyle, Semicolons,
    SortOrder, TrailingCommas, UseParentheses,
};
use dprint_plugin_typescript::{FormatError, FormatTextOptions, format_text};
use isograph_compiler::ArtifactPath;
use prelude::Postfix;

/// The repo's `.prettierrc.json` as dprint settings.
pub(crate) fn configuration() -> Configuration {
    ConfigurationBuilder::new()
        .line_width(80)
        .indent_width(2)
        .use_tabs(false)
        .quote_style(QuoteStyle::PreferSingle)
        .quote_props(QuoteProps::AsNeeded)
        .semicolons(Semicolons::Always)
        .trailing_commas(TrailingCommas::OnlyMultiLine)
        .arrow_function_use_parentheses(UseParentheses::Force)
        .binary_expression_operator_position(OperatorPosition::SameLine)
        .module_sort_import_declarations(SortOrder::Maintain)
        .module_blank_lines_after_imports(1)
        .build()
}

/// dprint could not format the text swc printed for the artifact at `path`.
#[derive(Debug, thiserror::Error)]
#[error("The generated artifact `{}` could not be formatted: {error}", .path.0)]
pub struct ArtifactFormatError {
    pub path: ArtifactPath,
    pub error: FormatError,
}

/// `text`, the module at `path`, formatted with `configuration`.
pub(crate) fn formatted(
    configuration: &Configuration,
    path: &ArtifactPath,
    text: String,
) -> Result<String, Box<ArtifactFormatError>> {
    format_text(FormatTextOptions {
        path: Path::new(path.0.as_str()),
        extension: None,
        text: text.as_str().into(),
        config: configuration,
        external_formatter: None,
        arena_pool: None,
    })
    .map_err(|error| {
        ArtifactFormatError {
            path: path.clone(),
            error,
        }
        .boxed()
    })
    .map(|formatted| formatted.unwrap_or(text))
}
