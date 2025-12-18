use std::str::FromStr;

use common_lang_types::{EntityName, IsographCodeAction, SelectableName};
use intern::Lookup;
use isograph_lang_types::SelectionType;
use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::{
    CodeAction, CodeActionOrCommand, CreateFile, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, ResourceOp, TextDocumentEdit,
    TextEdit, Uri, WorkspaceEdit,
    request::{CodeActionRequest, Request},
};
use prelude::Postfix;

use crate::{commands::OpenFileIsographLspCommand, lsp_state::LspState};
use crate::{
    commands::{IsographLspCommand, OpenFileIsographLspCommandParams},
    lsp_runtime_error::LSPRuntimeResult,
};

pub fn on_code_action<TNetworkProtocol: NetworkProtocol>(
    lsp_state: &LspState<TNetworkProtocol>,
    params: <CodeActionRequest as Request>::Params,
) -> LSPRuntimeResult<<CodeActionRequest as Request>::Result> {
    for diagnostic in params.context.diagnostics {
        if let Some(data) = diagnostic.data {
            let code_actions = serde_json::from_value::<Vec<IsographCodeAction>>(data).expect(
                "Expected deserialization to work. \
                This is indicative of a bug in Isograph.",
            );

            return code_actions
                .into_iter()
                .flat_map(|code_action| {
                    isograph_code_action_to_lsp_code_actions(
                        &lsp_state.compiler_state.db,
                        code_action,
                    )
                })
                .collect::<Vec<_>>()
                .wrap_some()
                .wrap_ok();
        }
    }

    // TODO support code actions that don't come from diagnostics

    Ok(None)
}

fn isograph_code_action_to_lsp_code_actions<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    action: IsographCodeAction,
) -> Vec<CodeActionOrCommand> {
    let config = db.get_isograph_config();

    match action {
        IsographCodeAction::CreateNewScalarSelectable(
            parent_object_entity_name_and_selectable_name,
        ) => {
            let parent_entity_name =
                parent_object_entity_name_and_selectable_name.parent_object_entity_name;
            let selectable_name = parent_object_entity_name_and_selectable_name.selectable_name;
            let new_file_path_string = format!(
                "{}/{}/{}.ts",
                config.project_root.to_str().expect(
                    "Expected project root to be able to be turned into a string. \
                    This is indicative of a bug in Isograph."
                ),
                parent_entity_name,
                selectable_name
            );
            let new_file_path_string_tsx = format!(
                "{}/{}/{}.tsx",
                config.project_root.to_str().expect(
                    "Expected project root to be able to be turned into a string. \
                    This is indicative of a bug in Isograph."
                ),
                parent_entity_name,
                selectable_name
            );
            let new_file_path = Uri::from_str(&new_file_path_string).expect(
                "Expected uri to be valid. \
                This is indicative of a bug in Isograph.",
            );
            let new_file_path_tsx = Uri::from_str(&new_file_path_string_tsx).expect(
                "Expected uri to be valid. \
                This is indicative of a bug in Isograph.",
            );

            vec![
                CodeActionOrCommand::CodeAction(create_new_selectable_code_action(
                    parent_entity_name,
                    selectable_name,
                    new_file_path_string,
                    new_file_path,
                    false,
                    SelectionType::Scalar(()),
                )),
                CodeActionOrCommand::CodeAction(create_new_selectable_code_action(
                    parent_entity_name,
                    selectable_name,
                    new_file_path_string_tsx,
                    new_file_path_tsx,
                    true,
                    SelectionType::Scalar(()),
                )),
            ]
        }
        IsographCodeAction::CreateNewObjectSelectable(
            parent_object_entity_name_and_selectable_name,
        ) => {
            let parent_entity_name =
                parent_object_entity_name_and_selectable_name.parent_object_entity_name;
            let selectable_name = parent_object_entity_name_and_selectable_name.selectable_name;
            let new_file_path_string = format!(
                "{}/{}/{}.ts",
                config.project_root.to_str().expect(
                    "Expected project root to be able to be turned into a string. \
                    This is indicative of a bug in Isograph."
                ),
                parent_entity_name,
                selectable_name
            );
            let new_file_path = Uri::from_str(&new_file_path_string).expect(
                "Expected uri to be valid. \
                This is indicative of a bug in Isograph.",
            );

            vec![CodeActionOrCommand::CodeAction(
                create_new_selectable_code_action(
                    parent_entity_name,
                    selectable_name,
                    new_file_path_string.clone(),
                    new_file_path.clone(),
                    false,
                    SelectionType::Object(()),
                ),
            )]
        }
    }
}

fn create_new_selectable_code_action(
    parent_entity_name: EntityName,
    selectable_name: SelectableName,
    new_file_path_string: String,
    new_file_path: Uri,
    // TODO it would be more elegant to make should_add_component_annotation
    // in the SelectionType::Scalar variant of selectable_type
    // And to make the tsx ending part of that...
    should_add_component_annotation: bool,
    selectable_type: SelectionType<(), ()>,
) -> CodeAction {
    let indent = "  ";

    let component_annotation = if should_add_component_annotation {
        "@component "
    } else {
        ""
    };

    let (keyword, to_section, target_range) = match selectable_type {
        SelectionType::Scalar(_) => ("field", "", None),
        SelectionType::Object(_) => {
            let left_char =
                (15 + parent_entity_name.lookup().len() + selectable_name.lookup().len()) as u32;
            (
                "pointer",
                " to TYPE",
                // Corresponds to TYPE in the generated output...
                Range::new(
                    Position {
                        line: 3,
                        character: left_char,
                    },
                    Position {
                        line: 3,
                        character: left_char + 4,
                    },
                )
                .wrap_some(),
            )
        }
    };

    CodeAction {
        title: format!(
            "Create new {component_annotation}{keyword} named `{}.{}`",
            parent_entity_name, selectable_name
        ),
        edit: WorkspaceEdit {
            document_changes: DocumentChanges::Operations(vec![
                DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                    uri: new_file_path.clone(),
                    options: None,
                    annotation_id: None,
                })),
                DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: new_file_path,
                        version: None,
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        range: Range {
                            start: Position {
                                line: 0,
                                character: 0,
                            },
                            end: Position {
                                line: 0,
                                character: 0,
                            },
                        },
                        new_text: format!(
                            "import {{ iso }} from '@iso';\n\
                            \n\
                            export const {parent_entity_name}__{selectable_name} = iso(`\n\
                            {indent}{keyword} {parent_entity_name}.{selectable_name}{to_section} {component_annotation}{{\n\
                            {indent}}}\n\
                            `)(({{ data }}) => {{\n\
                            {indent}return null;\n\
                            }})\n",
                        ),
                    })],
                }),
            ])
            .wrap_some(),
            ..Default::default()
        }
        .wrap_some(),
        command: OpenFileIsographLspCommand::command(OpenFileIsographLspCommandParams {
            uri_string: new_file_path_string,
            target_range,
        }).wrap_some(),
        ..Default::default()
    }
}
