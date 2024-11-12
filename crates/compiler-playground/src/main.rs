use std::{collections::HashMap, path::PathBuf};

use common_lang_types::TextSource;
use compiler_playground::incremental_compiler::database::Database;
use graphql_schema_parser::parse_schema;
use intern::string_key::Intern;
use isograph_compiler::read_schema_file;

fn main() {
    let mut database = Database {
        current_epoch: 0,
        nodes: HashMap::new(),
    };
    let mut view = database.create_view();

    let path = PathBuf::from("demos/github-demo/schema.graphql");

    let _ = view.calculate("read_and_parse_schema", path, |nested_view, param| {
        let schema = nested_view.calculate("read_schema", param, |_, param2| {
            let content = read_schema_file(&param2).unwrap();
            let schema_text_source = TextSource {
                path: param2
                    .to_str()
                    .expect("Expected schema to be valid string")
                    .intern()
                    .into(),
                span: None,
            };
            parse_schema(&content, schema_text_source).unwrap()
        });
        schema
    });
    println!("{:?}", database)
}
