/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

#![deny(warnings)]
#![deny(rust_2018_idioms)]
#![allow(clippy::all)]

mod relay_console_logger;
mod relay_diagnostic;
mod relay_diagnostic_check;
mod relay_feature_flags;
mod relay_location;
mod relay_named_item;
mod relay_perf_logger;
mod relay_pointer_address;
mod relay_rollout;
mod relay_span;
pub mod relay_sync;
mod relay_text_source;

pub use lsp_types::DiagnosticSeverity;
pub use lsp_types::DiagnosticTag;
pub use relay_console_logger::print_time;
pub use relay_console_logger::ConsoleLogEvent;
pub use relay_console_logger::ConsoleLogger;
pub use relay_diagnostic::diagnostics_result;
pub use relay_diagnostic::get_diagnostics_data;
pub use relay_diagnostic::Diagnostic;
pub use relay_diagnostic::DiagnosticDisplay;
pub use relay_diagnostic::DiagnosticRelatedInformation;
pub(crate) use relay_diagnostic::Diagnostics;
pub use relay_diagnostic::DiagnosticsResult;
pub use relay_diagnostic::WithDiagnosticData;
pub use relay_diagnostic::WithDiagnostics;
pub use relay_diagnostic_check::escalate_and_check;
pub use relay_diagnostic_check::CriticalDiagnostics;
pub use relay_diagnostic_check::DiagnosticCheck;
pub use relay_diagnostic_check::StableDiagnostics;
pub use relay_feature_flags::FeatureFlag;
pub use relay_feature_flags::FeatureFlags;
pub use relay_location::Location;
pub use relay_location::SourceLocationKey;
pub use relay_location::WithLocation;
pub use relay_named_item::ArgumentName;
pub use relay_named_item::DirectiveName;
pub use relay_named_item::EnumName;
pub use relay_named_item::InputObjectName;
pub use relay_named_item::InterfaceName;
pub use relay_named_item::Named;
pub use relay_named_item::NamedItem;
pub use relay_named_item::ObjectName;
pub use relay_named_item::ScalarName;
pub use relay_named_item::UnionName;
pub use relay_perf_logger::NoopPerfLogger;
pub use relay_perf_logger::NoopPerfLoggerEvent;
pub use relay_perf_logger::PerfLogEvent;
pub use relay_perf_logger::PerfLogger;
pub use relay_pointer_address::PointerAddress;
pub use relay_rollout::Rollout;
pub use relay_span::Span;
pub use relay_text_source::TextSource;
