use common_lang_types::InMemorySourceReader;
use common_lang_types::Span;
use common_lang_types::TextSource;
use common_lang_types::set_source_reader;
use intern::string_key::Intern;
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    ClientFieldDeclaration, ClientPointerDeclaration, ClientScalarSelectableDirectiveSet,
    EntrypointDeclaration,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[derive(Serialize, Deserialize)]
pub struct ParseResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<ParsedData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ParsedData {
    ClientFieldDeclaration(ClientFieldInfo),
    ClientPointerDeclaration(ClientPointerInfo),
    EntrypointDeclaration(EntrypointInfo),
}

#[derive(Serialize, Deserialize)]
pub struct ClientFieldInfo {
    pub parent_type: String,
    pub field_name: String,
    pub export_name: String,
    pub description: Option<String>,
    pub has_component_directive: bool,
    pub variable_count: usize,
    pub selection_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ClientPointerInfo {
    pub parent_type: String,
    pub pointer_name: String,
    pub export_name: String,
    pub target_type: String,
    pub description: Option<String>,
    pub directives: Vec<String>,
    pub variable_count: usize,
    pub selection_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EntrypointInfo {
    pub parent_type: String,
    pub field_name: String,
}

fn extract_field_info(decl: &ClientFieldDeclaration) -> ClientFieldInfo {
    ClientFieldInfo {
        parent_type: decl.parent_type.item.to_string(),
        field_name: decl.client_field_name.item.to_string(),
        export_name: decl.const_export_name.to_string(),
        description: decl.description.as_ref().map(|d| d.item.to_string()),
        has_component_directive: decl
            .client_scalar_selectable_directive_set
            .as_ref()
            .map(|d| matches!(d, ClientScalarSelectableDirectiveSet::Component(_)))
            .unwrap_or(false),
        variable_count: decl.variable_definitions.len(),
        selection_count: decl.selection_set.item.selections.len(),
    }
}

fn extract_pointer_info(decl: &ClientPointerDeclaration) -> ClientPointerInfo {
    ClientPointerInfo {
        parent_type: decl.parent_type.item.to_string(),
        pointer_name: decl.client_pointer_name.item.to_string(),
        export_name: decl.const_export_name.to_string(),
        target_type: format!("{:?}", decl.target_type), // Simplified for now
        description: decl.description.as_ref().map(|d| d.item.to_string()),
        directives: decl
            .directives
            .iter()
            .map(|d| d.item.name.to_string())
            .collect(),
        variable_count: decl.variable_definitions.len(),
        selection_count: decl.selection_set.item.selections.len(),
    }
}

fn extract_entrypoint_info(decl: &EntrypointDeclaration) -> EntrypointInfo {
    EntrypointInfo {
        parent_type: decl.parent_type.item.to_string(),
        field_name: decl.client_field_name.item.to_string(),
    }
}

#[wasm_bindgen]
pub fn parse_iso_literal(
    source_text: &str,
    file_path: &str,
    export_name: Option<String>,
) -> JsValue {
    let reader = InMemorySourceReader::new();
    reader.add_source(file_path.to_string(), source_text.to_string());
    set_source_reader(Box::new(reader));

    let text_source = TextSource {
        current_working_directory: "".intern().into(),
        relative_path_to_source_file: file_path.intern().into(),
        span: Some(Span::new(0, source_text.len() as u32)),
    };

    let result = isograph_lang_parser::parse_iso_literal(
        source_text.to_string(),
        text_source.relative_path_to_source_file,
        export_name,
        text_source,
    );

    match result {
        Ok(result) => {
            let data = match result {
                IsoLiteralExtractionResult::ClientFieldDeclaration(decl) => {
                    ParsedData::ClientFieldDeclaration(extract_field_info(&decl.item))
                }
                IsoLiteralExtractionResult::ClientPointerDeclaration(decl) => {
                    ParsedData::ClientPointerDeclaration(extract_pointer_info(&decl.item))
                }
                IsoLiteralExtractionResult::EntrypointDeclaration(decl) => {
                    ParsedData::EntrypointDeclaration(extract_entrypoint_info(&decl.item))
                }
            };

            serde_json::to_string(&ParseResult {
                success: true,
                data: Some(data),
                errors: None,
            })
            .unwrap()
            .into()
        }
        Err(diagnostic) => serde_json::to_string(&ParseResult {
            success: false,
            data: None,
            errors: Some(vec![diagnostic.to_string()]),
        })
        .unwrap()
        .into(),
    }
}
