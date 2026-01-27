use std::collections::BTreeMap;
use std::fmt;

use common_lang_types::{
    Diagnostic, DiagnosticResult, EntityName, JavascriptName, QueryExtraInfo, QueryOperationName,
    QueryText, SelectableName, WithNonFatalDiagnostics,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, NonConstantValue, SelectionType, TypeAnnotationDeclaration, UnionVariant,
    VariableDeclaration,
};
use isograph_schema::{
    CompilationProfile, IsographDatabase, MemoRefServerSelectable, TargetPlatform,
    flattened_selectables_for_entity, selectable_named,
};
use isograph_schema::{
    ConcreteTargetEntityName, DeprecatedParseTypeSystemOutcome, Format, ID_FIELD_NAME,
    ID_VARIABLE_NAME, MergedSelectionMap, NODE_FIELD_NAME, NetworkProtocol, NormalizationKey,
    RootOperationName, WrapMergedSelectionMapResult, WrappedSelectionMapSelection,
    flattened_entity_named, selection_map_wrapped,
};
use lazy_static::lazy_static;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::nested_server_schema::parse_nested_schema;
use crate::parse_type_system_document::parse_type_system_document;
use crate::query_text::generate_query_text;

lazy_static! {
    pub static ref UNKNOWN_JAVASCRIPT_TYPE: JavascriptName = "unknown".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
    pub static ref BOOLEAN_JAVASCRIPT_TYPE: JavascriptName = "boolean".intern().into();
    pub static ref NUMBER_JAVASCRIPT_TYPE: JavascriptName = "number".intern().into();
    pub static ref NEVER_JAVASCRIPT_TYPE: JavascriptName = "never".intern().into();
}

pub(crate) struct GraphQLRootTypes {
    pub query: EntityName,
    pub mutation: EntityName,
    pub subscription: EntityName,
}

impl Default for GraphQLRootTypes {
    fn default() -> Self {
        Self {
            query: "Query".intern().into(),
            mutation: "Mutation".intern().into(),
            subscription: "Subscription".intern().into(),
        }
    }
}

impl From<GraphQLRootTypes> for BTreeMap<EntityName, RootOperationName> {
    fn from(val: GraphQLRootTypes) -> Self {
        let mut map = BTreeMap::new();
        map.insert(val.query, RootOperationName("query"));
        map.insert(val.mutation, RootOperationName("mutation"));
        map.insert(val.subscription, RootOperationName("subscription"));
        map
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum GraphQLOperationKind {
    Query,
    Mutation,
    Subscription,
}

impl fmt::Display for GraphQLOperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLOperationKind::Query => write!(f, "query"),
            GraphQLOperationKind::Mutation => write!(f, "mutation"),
            GraphQLOperationKind::Subscription => write!(f, "subscription"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GraphQLWrapStrategy {
    None,
    Node { query_root: EntityName },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GraphQLFetchableInfo {
    pub operation_kind: GraphQLOperationKind,
    pub wrap_strategy: GraphQLWrapStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct GraphQLNetworkProtocolEntityAssociatedData {
    pub fetchable: Option<GraphQLFetchableInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct GraphQLAndJavascriptProfile {}

impl CompilationProfile for GraphQLAndJavascriptProfile {
    type NetworkProtocol = GraphQLNetworkProtocol;
    type TargetPlatform = JavascriptTargetPlatform;

    #[expect(clippy::type_complexity)]
    #[memo]
    fn deprecated_parse_type_system_documents(
        db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    ) -> DiagnosticResult<(
        WithNonFatalDiagnostics<DeprecatedParseTypeSystemOutcome<GraphQLAndJavascriptProfile>>,
        // fetchable types
        BTreeMap<EntityName, RootOperationName>,
    )> {
        parse_type_system_document(db)
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
    type EntityAssociatedData = SelectionType<JavascriptName, GraphQLSchemaObjectAssociatedData>;

    type SelectableAssociatedData = ();

    fn format_server_field_scalar_type<
        TCompilationProfile: CompilationProfile<TargetPlatform = Self>,
    >(
        db: &IsographDatabase<TCompilationProfile>,
        entity_name: EntityName,
        indentation_level: u8,
    ) -> String {
        let entity = flattened_entity_named(db, entity_name).expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

        match entity
            .lookup(db)
            .associated_data
            .as_ref()
            .as_server()
            .expect("Expected entity to be server defined.")
            .target_platform
            .as_ref()
        {
            SelectionType::Object(_) => {
                // TODO this is bad; we should never create a type containing all of the fields
                // on a given object. This is currently used for input objects, and we should
                // consider how to do this is a not obviously broken manner.
                let mut s = "{\n".to_string();

                for (name, server_selectable) in flattened_selectables_for_entity(db, entity_name)
                    .as_ref()
                    .expect("Expected entity to be defined")
                {
                    let field_type = format_field_definition(
                        db,
                        name,
                        server_selectable.dereference(),
                        indentation_level + 1,
                    );
                    s.push_str(&field_type)
                }

                s.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));
                s
            }
            SelectionType::Scalar(s) => s.to_string(),
        }
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
        db: &IsographDatabase<TCompilationProfile>,
        server_object_entity_name: &EntityName,
    ) -> String {
        let server_object_entity = &flattened_entity_named(db, *server_object_entity_name)
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db);

        if server_object_entity
            .selection_info
            .as_object()
            .expect("Expected server object entity to be object")
            .is_concrete
            .0
        {
            let name = server_object_entity.name;
            return format!("\n  | Link<\"{name}\">");
        }

        let subtypes = server_object_entity
            .associated_data
            .as_ref()
            .as_server()
            .expect("Expected entity to be server defined.")
            .target_platform
            .as_ref()
            .as_object()
            .expect("Expected server object entity to have object associated data")
            .subtypes
            .reference();

        if subtypes.is_empty() {
            return (*NEVER_JAVASCRIPT_TYPE).to_string();
        }

        let subtypes = subtypes
            .iter()
            .map(|name| format!("\n  | Link<\"{name}\">"))
            .collect::<Vec<_>>();

        subtypes.join("")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct GraphQLNetworkProtocol {}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type EntityAssociatedData = GraphQLNetworkProtocolEntityAssociatedData;
    type SelectableAssociatedData = ();

    fn generate_query_text<'a, TCompilationProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDatabase<TCompilationProfile>,
        root_entity: EntityName,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
        format: Format,
    ) -> QueryText {
        let operation_kind = flattened_entity_named(db, root_entity)
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("Type `{root_entity}` not found in schema."),
                    None,
                )
            })
            .expect("Expected schema to contain root entity.")
            .lookup(db)
            .associated_data
            .as_ref()
            .as_server()
            .expect("Expected entity to be server defined.")
            .network_protocol
            .fetchable
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "Expected `{root_entity}` to be fetchable (missing network protocol entity associated data)."
                )
            })
            .operation_kind;

        generate_query_text(
            operation_kind,
            query_name,
            selection_map,
            query_variables,
            format,
        )
    }

    fn wrap_merged_selection_map<
        TCompilationProfile: CompilationProfile<NetworkProtocol = Self>,
    >(
        db: &IsographDatabase<TCompilationProfile>,
        root_entity: EntityName,
        merged_selection_map: MergedSelectionMap,
    ) -> DiagnosticResult<WrapMergedSelectionMapResult> {
        let fetchable_info = flattened_entity_named(db, root_entity)
            .ok_or_else(|| {
                Diagnostic::new(format!("Type `{root_entity}` not found in schema."), None)
            })?
            .lookup(db)
            .associated_data
            .as_ref()
            .as_server()
            .expect("Expected entity to be server defined.")
            .network_protocol
            .fetchable
            .as_ref()
            .ok_or_else(|| {
                Diagnostic::new(format!("Type `{root_entity}` is not fetchable."), None)
            })?;

        match fetchable_info.wrap_strategy {
            GraphQLWrapStrategy::None => Ok(WrapMergedSelectionMapResult {
                root_entity,
                merged_selection_map,
            }),
            GraphQLWrapStrategy::Node { query_root } => {
                let already_wrapped_with_node_field = merged_selection_map.keys().any(|key| {
                    matches!(
                        key,
                        NormalizationKey::ServerField(name_and_arguments)
                            if name_and_arguments.name == *NODE_FIELD_NAME
                    )
                });

                let merged_selection_map = if already_wrapped_with_node_field {
                    merged_selection_map
                } else {
                    selection_map_wrapped(
                        merged_selection_map,
                        vec![
                            WrappedSelectionMapSelection::InlineFragment(root_entity),
                            WrappedSelectionMapSelection::LinkedField {
                                server_object_selectable_name: *NODE_FIELD_NAME,
                                arguments: vec![ArgumentKeyAndValue {
                                    key: (*ID_FIELD_NAME).unchecked_conversion(),
                                    value: NonConstantValue::Variable((*ID_VARIABLE_NAME).into()),
                                }],
                                concrete_target_entity_name: ConcreteTargetEntityName::Abstract,
                                is_fallible: true,
                            },
                        ],
                    )
                };

                Ok(WrapMergedSelectionMapResult {
                    root_entity: query_root,
                    merged_selection_map,
                })
            }
        }
    }

    fn generate_query_extra_info(
        query_name: QueryOperationName,
        operation_name: EntityName,
        indentation_level: u8,
    ) -> QueryExtraInfo {
        let indent = "  ".repeat((indentation_level + 1) as usize);
        QueryExtraInfo(format!(
            "{{\n\
            {indent}  kind: \"PersistedOperationExtraInfo\",\n\
            {indent}  operationName: \"{query_name}\",\n\
            {indent}  operationKind: \"{operation_name}\",\n\
            {indent}}}"
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Ord, PartialOrd)]
pub struct GraphQLSchemaObjectAssociatedData {
    pub subtypes: Vec<EntityName>,
}

fn format_field_definition<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    name: &SelectableName,
    server_selectable: MemoRefServerSelectable<TCompilationProfile>,
    indentation_level: u8,
) -> String {
    let server_selectable = server_selectable.lookup(db);
    let is_optional = is_nullable(
        server_selectable
            .target_entity
            .item
            .as_ref()
            .expect("Expected target entity to be valid.")
            .reference(),
    );
    let target_type_annotation = server_selectable.target_entity.clone();

    format!(
        "{}readonly {}{}: {},\n",
        "  ".repeat(indentation_level as usize),
        name,
        if is_optional { "?" } else { "" },
        format_type_annotation(
            db,
            target_type_annotation
                .item
                .as_ref()
                .expect("Expected target entity to be valid.")
                .reference(),
            indentation_level + 1
        ),
    )
}

fn is_nullable(type_annotation: &TypeAnnotationDeclaration) -> bool {
    match type_annotation {
        TypeAnnotationDeclaration::Union(union) => union.nullable,
        TypeAnnotationDeclaration::Plural(_) => false,
        TypeAnnotationDeclaration::Scalar(_) => false,
    }
}

fn format_type_annotation<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    type_annotation: &TypeAnnotationDeclaration,
    indentation_level: u8,
) -> String {
    match type_annotation.reference() {
        TypeAnnotationDeclaration::Scalar(scalar) => {
            TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                db,
                scalar.0,
                indentation_level + 1,
            )
        }
        TypeAnnotationDeclaration::Union(union_type_annotation) => {
            if union_type_annotation.variants.is_empty() {
                panic!("Unexpected union with not enough variants.");
            }

            let mut s = String::new();
            if union_type_annotation.variants.len() > 1 || union_type_annotation.nullable {
                s.push('(');
                for (index, variant) in union_type_annotation.variants.iter().enumerate() {
                    if index != 0 {
                        s.push_str(" | ");
                    }

                    match variant {
                        UnionVariant::Scalar(scalar) => {
                            s.push_str(&TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                                db,
                                scalar.0,
                                indentation_level + 1,
                            ));
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            s.push_str(&format_type_annotation(
                                db,
                                type_annotation.item.reference(),
                                indentation_level + 1,
                            ));
                            s.push('>');
                        }
                    }
                }
                if union_type_annotation.nullable {
                    s.push_str(" | null");
                }
                s.push(')');
                s
            } else {
                let variant = union_type_annotation
                    .variants
                    .first()
                    .expect("Expected variant to exist");
                match variant {
                    UnionVariant::Scalar(scalar) => {
                        TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                            db,
                            scalar.0,
                            indentation_level + 1,
                        )
                    }
                    UnionVariant::Plural(type_annotation) => {
                        format!(
                            "ReadonlyArray<{}>",
                            TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                                db,
                                type_annotation.item.inner().0,
                                indentation_level + 1
                            )
                        )
                    }
                }
            }
        }
        TypeAnnotationDeclaration::Plural(type_annotation) => {
            format!(
                "ReadonlyArray<{}>",
                TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                    db,
                    type_annotation.item.inner().0,
                    indentation_level + 1
                )
            )
        }
    }
}
