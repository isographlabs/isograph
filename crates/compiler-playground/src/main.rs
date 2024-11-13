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
        dependency_stack: vec![],
    };

    let path = PathBuf::from("demos/github-demo/schema.graphql");

    database.set("file_text", "foo.txt", Box::new("hello"));

    let _ = database.calculate("read_and_parse_schema", path, |db, param| {
        database.get("file_text", "foo.txt");
        database.get_or_set_default("file_text", "foo.txt");
        database.get_or_set("file_text", "foo.txt", Box::new("ASFASDF"));

        read_parsed_derived(db, param)
    });
}

fn read_parsed_derived(db: &mut Database, param: PathBuf) -> _ {
    let schema = db.calculate("read_schema", param, |_, param2| {
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
}
