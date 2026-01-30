use std::collections::BTreeMap;

use common_lang_types::{
    DiagnosticResult, EntityName, JavascriptName, QueryExtraInfo, QueryOperationName, QueryText,
    SelectableName, WithNonFatalDiagnostics,
};
use intern::string_key::Intern;
use isograph_lang_types::{SelectionType, VariableDeclaration};
use isograph_schema::{
    CompilationProfile, IsographDatabase, TargetPlatform, WrapMergedSelectionMapResult,
    flattened_entity_named, selectable_named,
};
use isograph_schema::{
    DeprecatedParseTypeSystemOutcome, Format, MergedSelectionMap, NetworkProtocol,
    RootOperationName,
};
use pico_macros::memo;
use prelude::*;

use crate::nested_server_schema::parse_nested_schema;
use crate::query_text::generate_query_text;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref UNKNOWN_JAVASCRIPT_TYPE: JavascriptName = "unknown".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
    pub static ref BOOLEAN_JAVASCRIPT_TYPE: JavascriptName = "boolean".intern().into();
    pub static ref NUMBER_JAVASCRIPT_TYPE: JavascriptName = "number".intern().into();
    pub static ref NEVER_JAVASCRIPT_TYPE: JavascriptName = "never".intern().into();
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct SQLAndJavascriptProfile {}

impl CompilationProfile for SQLAndJavascriptProfile {
    type NetworkProtocol = SQLNetworkProtocol;
    type TargetPlatform = JavascriptTargetPlatform;

    #[expect(clippy::type_complexity)]
    #[memo]
    fn deprecated_parse_type_system_documents(
        _db: &IsographDatabase<SQLAndJavascriptProfile>,
    ) -> DiagnosticResult<(
        WithNonFatalDiagnostics<DeprecatedParseTypeSystemOutcome<SQLAndJavascriptProfile>>,
        BTreeMap<EntityName, RootOperationName>,
    )> {
        Ok((
            WithNonFatalDiagnostics {
                non_fatal_diagnostics: vec![],
                item: DeprecatedParseTypeSystemOutcome {
                    selectables: BTreeMap::new(),
                    client_scalar_refetch_strategies: vec![],
                },
            },
            BTreeMap::new(),
        ))
    }

    #[memo]
    fn parse_nested_data_model_schema(
        db: &IsographDatabase<Self>,
    ) -> isograph_schema::NestedDataModelSchema<Self> {
        parse_nested_schema(db)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct JavascriptTargetPlatform {}

impl TargetPlatform for JavascriptTargetPlatform {
    type EntityAssociatedData = SelectionType<JavascriptName, SQLSchemaObjectAssociatedData>;

    type SelectableAssociatedData = ();

    fn format_server_field_scalar_type<
        TCompilationProfile: CompilationProfile<TargetPlatform = Self>,
    >(
        _db: &IsographDatabase<TCompilationProfile>,
        _entity_name: EntityName,
        _indentation_level: u8,
    ) -> String {
        todo!()
    }

    fn get_inner_text_for_selectable<
        TCompilationProfile: CompilationProfile<TargetPlatform = Self>,
    >(
        db: &IsographDatabase<TCompilationProfile>,
        parent_object_entity_name: EntityName,
        selectable_name: SelectableName,
    ) -> JavascriptName {
        let server_scalar_selectable =
            selectable_named(db, parent_object_entity_name, selectable_name)
                .clone_err()
                .expect("Expected parsing to have worked")
                .expect("Expected selectable to exist")
                .as_server()
                .expect("Expected selectable to be server selectable")
                .lookup(db);

        flattened_entity_named(
            db,
            server_scalar_selectable
                .target_entity
                .item
                .as_ref()
                .expect("Expected target entity to be valid.")
                .inner()
                .0,
        )
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .lookup(db)
        .associated_data
        .as_ref()
        .as_server()
        .expect("Expected entity to be server defined.")
        .target_platform
        .as_ref()
        .as_scalar()
        .expect("Expected scalar entity to be scalar")
        .dereference()
    }

    fn generate_link_type<'a, TCompilationProfile: CompilationProfile<TargetPlatform = Self>>(
        _db: &IsographDatabase<TCompilationProfile>,
        _server_object_entity_name: &EntityName,
    ) -> String {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct SQLNetworkProtocol {}

impl NetworkProtocol for SQLNetworkProtocol {
    type EntityAssociatedData = ();
    type SelectableAssociatedData = ();

    fn generate_query_text<'a, TCompilationProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDatabase<TCompilationProfile>,
        _root_entity: EntityName,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
        format: Format,
    ) -> QueryText {
        generate_query_text(query_name, selection_map, query_variables, format)
    }

    fn wrap_merged_selection_map<
        TCompilationProfile: CompilationProfile<NetworkProtocol = Self>,
    >(
        _db: &IsographDatabase<TCompilationProfile>,
        root_entity: EntityName,
        merged_selection_map: MergedSelectionMap,
    ) -> DiagnosticResult<WrapMergedSelectionMapResult> {
        WrapMergedSelectionMapResult {
            root_entity,
            merged_selection_map,
        }
        .wrap_ok()
    }

    fn generate_query_extra_info(
        _query_name: QueryOperationName,
        _operation_name: EntityName,
        _indentation_level: u8,
    ) -> QueryExtraInfo {
        QueryExtraInfo("".to_string())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Ord, PartialOrd)]
pub struct SQLSchemaObjectAssociatedData {}
