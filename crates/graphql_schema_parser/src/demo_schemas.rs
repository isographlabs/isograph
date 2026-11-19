use crate::parse_schema;

const TOP_LEVEL_KEYWORDS: [&str; 9] = [
    "schema",
    "scalar",
    "type",
    "interface",
    "union",
    "enum",
    "input",
    "directive",
    "extend",
];

/// The lines of `text` that start, unindented, with a definition keyword or `extend`.
fn count_top_level_keywords(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            line.split_whitespace()
                .next()
                .is_some_and(|word| TOP_LEVEL_KEYWORDS.contains(&word))
                && !line.starts_with(char::is_whitespace)
        })
        .count()
}

fn assert_every_definition_parses(text: &str) {
    let parsed = parse_schema(text);
    let errors: Vec<_> = parsed
        .definitions
        .iter()
        .filter_map(|definition| definition.as_ref().err())
        .collect();
    assert!(errors.is_empty(), "failed definitions: {errors:?}");
    assert_eq!(parsed.definitions.len(), count_top_level_keywords(text));
}

#[test]
fn the_github_demo_schema_subset_parses() {
    let text = include_str!("../tests/fixtures/github-demo-subset.graphql");
    assert_eq!(count_top_level_keywords(text), 75);
    assert_every_definition_parses(text);
}

#[test]
fn the_pet_demo_schema_parses() {
    assert_every_definition_parses(include_str!("../tests/fixtures/pet-demo-schema.graphql"));
}

#[test]
fn the_pet_demo_schema_extension_parses() {
    let text = include_str!("../tests/fixtures/pet-demo-schema-extension.graphql");
    assert_eq!(count_top_level_keywords(text), 2);
    assert_every_definition_parses(text);
}
