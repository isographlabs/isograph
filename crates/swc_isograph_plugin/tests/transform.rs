use std::path::{Path, PathBuf};

use prelude::Postfix;
use swc_core::{
    common::{FileName, SyntaxContext, errors::HANDLER},
    ecma::{
        ast::{EsVersion, Program},
        parser::{EsSyntax, Syntax, parse_file_as_module},
        transforms::testing::test_transform,
    },
    testing::Tester,
};
use swc_isograph_plugin::{ProjectConfig, compile_iso_literal_visitor};

const ROOT_DIR: &str = "/project";
const FILE: &str = "/project/src/components/HomeRoute.tsx";

fn syntax() -> Syntax {
    Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
    })
}

fn no_output_directory() -> ProjectConfig {
    ProjectConfig {
        output_directory: None,
    }
}

enum SourceType {
    Module,
    Script,
}

fn assert_transform(
    config: &ProjectConfig,
    file: &str,
    source_type: SourceType,
    input: &str,
    expected: &str,
) {
    let is_module = match source_type {
        SourceType::Module => true,
        SourceType::Script => false,
    };
    test_transform(
        syntax(),
        is_module.wrap_some(),
        |_| {
            compile_iso_literal_visitor(
                config,
                Path::new(file),
                Path::new(ROOT_DIR),
                SyntaxContext::empty(),
            )
        },
        input,
        expected,
    );
}

fn transform_errors(input: &str) -> Vec<String> {
    transform_errors_in(FILE, input)
}

/// Transforms `input` as the module `file` and returns the messages of the errors it emits.
/// Asserts that the transform left the program unchanged.
fn transform_errors_in(file: &str, input: &str) -> Vec<String> {
    let config = no_output_directory();
    let diagnostics = Tester::new()
        .errors(|cm, handler| {
            HANDLER.set(handler.reference(), || {
                let source_file = cm.new_source_file(
                    FileName::Real(PathBuf::from("input.js")).to(),
                    input.to_string(),
                );
                let module = parse_file_as_module(
                    source_file.reference(),
                    syntax(),
                    EsVersion::latest(),
                    None,
                    &mut vec![],
                )
                .expect("the test input is valid JavaScript");
                let program = Program::Module(module);
                let transformed = program.clone().apply(compile_iso_literal_visitor(
                    config.reference(),
                    Path::new(file),
                    Path::new(ROOT_DIR),
                    SyntaxContext::empty(),
                ));
                assert_eq!(transformed, program);
            });
            ().wrap_err::<()>()
        })
        .expect_err("returning Err from the closure yields the emitted diagnostics");
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message())
        .collect()
}

#[test]
fn an_uncalled_field_literal_becomes_the_identity_function() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        r#"
export const HomeRoute = iso(`
    field Query.HomeRoute @component {
      pets {
        id
      }
    }
  `);
"#,
        "export const HomeRoute = (x) => x;",
    );
}

#[test]
fn a_called_field_literal_becomes_its_argument() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        r#"
export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
    }
  }
`)(function HomeRouteComponent({ data }) {
  return "Render";
});
"#,
        r#"
export const HomeRoute = function HomeRouteComponent({ data }) {
  return "Render";
};
"#,
    );
}

#[test]
fn a_pointer_literal_is_a_field_literal() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        "export const p = iso(`pointer Query.p to Pet { id }`)(f);",
        "export const p = f;",
    );
}

#[test]
fn an_entrypoint_in_a_module_becomes_a_hoisted_import() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        r#"
function test() {
  const a = iso(`entrypoint Query.HomeRoute`);
}
"#,
        r#"
import _Query__HomeRoute from "../../__isograph/Query/HomeRoute/entrypoint.ts";

function test() {
  const a = _Query__HomeRoute;
}
"#,
    );
}

#[test]
fn an_entrypoint_in_a_script_becomes_a_require_call() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Script,
        r#"
const { fragmentReference } = useLazyReference(
  iso(`entrypoint Query.HomeRoute`),
  {},
);
"#,
        r#"
const { fragmentReference } = useLazyReference(require("../../__isograph/Query/HomeRoute/entrypoint.ts").default, {});
"#,
    );
}

#[test]
fn an_entrypoint_inside_a_field_function_is_transformed() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        r#"
export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
    }
  }
`)(function HomeRouteComponent({ data }) {
  const { fragmentReference } = useImperativeReference(
    iso(`entrypoint Query.PetFavoritePhrase`),
  );
  return "Render";
});
"#,
        r#"
import _Query__PetFavoritePhrase from "../../__isograph/Query/PetFavoritePhrase/entrypoint.ts";

export const HomeRoute = function HomeRouteComponent({ data }) {
  const { fragmentReference } = useImperativeReference(_Query__PetFavoritePhrase);
  return "Render";
};
"#,
    );
}

#[test]
fn two_literals_of_one_entrypoint_produce_one_import() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        r#"
const a = iso(`entrypoint Query.HomeRoute`);
const b = iso(`entrypoint Query.HomeRoute`);
"#,
        r#"
import _Query__HomeRoute from "../../__isograph/Query/HomeRoute/entrypoint.ts";

const a = _Query__HomeRoute;
const b = _Query__HomeRoute;
"#,
    );
}

#[test]
fn the_artifact_is_under_the_output_directory() {
    assert_transform(
        ProjectConfig {
            output_directory: PathBuf::from("src/generated").wrap_some(),
        }
        .reference(),
        "/project/src/pages/Home.tsx",
        SourceType::Module,
        "const a = iso(`entrypoint Query.HomeRoute`);",
        r#"
import _Query__HomeRoute from "../generated/__isograph/Query/HomeRoute/entrypoint.ts";

const a = _Query__HomeRoute;
"#,
    );
}

#[test]
fn an_artifact_in_the_files_directory_gets_a_dot_slash_prefix() {
    assert_transform(
        no_output_directory().reference(),
        "/project/File.tsx",
        SourceType::Module,
        "const a = iso(`entrypoint Query.HomeRoute`);",
        r#"
import _Query__HomeRoute from "./__isograph/Query/HomeRoute/entrypoint.ts";

const a = _Query__HomeRoute;
"#,
    );
}

#[test]
fn an_artifact_below_the_file_gets_a_dot_slash_prefix() {
    assert_transform(
        ProjectConfig {
            output_directory: PathBuf::from("src/generated").wrap_some(),
        }
        .reference(),
        "/project/File.tsx",
        SourceType::Module,
        "const a = iso(`entrypoint Query.HomeRoute`);",
        r#"
import _Query__HomeRoute from "./src/generated/__isograph/Query/HomeRoute/entrypoint.ts";

const a = _Query__HomeRoute;
"#,
    );
}

#[test]
fn an_entity_literal_becomes_undefined() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Module,
        r#"
iso(`entity Owner`);
const a = iso(`
  entity Owner
`);
"#,
        r#"
void 0;
const a = void 0;
"#,
    );
}

#[test]
fn an_entity_literal_in_a_script_becomes_undefined() {
    assert_transform(
        no_output_directory().reference(),
        FILE,
        SourceType::Script,
        "iso(`entity Owner`);",
        "void 0;",
    );
}

#[test]
fn a_called_entity_literal_is_an_error() {
    let errors = transform_errors("const a = iso(`entity Owner`)(f);");
    assert_eq!(
        errors,
        ["Invalid iso tag usage. An entity literal cannot be called."]
    );
}

#[test]
fn an_unknown_keyword_is_an_error() {
    let errors = transform_errors(
        r#"
export const HomeRoute = iso(`
  unknown Query.HomeRoute @component {
    pets {
      id
    }
  }
`)(function Test() {
  return 'Render';
});
"#,
    );
    assert_eq!(
        errors,
        ["Invalid iso tag usage. Expected 'entity', 'entrypoint', 'field' or 'pointer'."]
    );
}

#[test]
fn a_field_call_without_arguments_is_an_error() {
    let errors = transform_errors(
        r#"
export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
    }
  }
`)();
"#,
    );
    assert_eq!(
        errors,
        ["Invalid iso tag usage. The iso function should be passed exactly one argument."]
    );
}

#[test]
fn a_substitution_is_an_error() {
    let errors = transform_errors("const a = iso(`field Query.${x} { id }`)(f);");
    assert_eq!(errors, ["Substitutions are not allowed in iso fragments."]);
}

#[test]
fn a_non_template_argument_is_an_error() {
    let errors = transform_errors("const a = iso('entrypoint Query.HomeRoute');");
    assert_eq!(
        errors,
        ["Only template literals are allowed in iso fragments."]
    );
}

#[test]
fn an_iso_call_without_arguments_is_an_error() {
    let errors = transform_errors("const a = iso();");
    assert_eq!(errors, ["Iso invocation require one parameter."]);
}

#[test]
fn a_relative_file_name_cannot_reach_the_artifact() {
    let errors = transform_errors_in(
        "src/File.tsx",
        "const a = iso(`entrypoint Query.HomeRoute`);",
    );
    assert_eq!(
        errors,
        [
            "Cannot compute the path from this file to the entrypoint artifact. root_dir and the file name must both be absolute."
        ]
    );
}
