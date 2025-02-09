use std::sync::atomic::{AtomicUsize, Ordering};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::{Database, SourceId};
use pico_macros::{memo, Source};

mod calc;

static VALUE: AtomicUsize = AtomicUsize::new(0);
static EXPECTED: AtomicUsize = AtomicUsize::new(0);

#[test]
fn arg_reference() {
    let mut db = Database::default();

    let input = db.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });

    let value = evaluate_input(&db, input);
    // assert that the value was not cloned
    assert_eq!(VALUE.load(Ordering::SeqCst), 0);

    let expected = Expected(6);
    assert_result(&db, value, expected);
    // assert that the argument of type `&Value` was cloned only once, when it was
    // inserted into the params store, and then internally used by reference
    assert_eq!(VALUE.load(Ordering::SeqCst), 1);
    // compare with the argument of type `Expected` which is cloned twice
    assert_eq!(EXPECTED.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Value(pub i64);

impl Clone for Value {
    fn clone(&self) -> Self {
        VALUE.fetch_add(1, Ordering::SeqCst);
        Self(self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Expected(pub i64);

impl Clone for Expected {
    fn clone(&self) -> Self {
        EXPECTED.fetch_add(1, Ordering::SeqCst);
        Self(self.0)
    }
}

#[memo]
fn parse_ast(db: &Database, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

// #[memo(reference)]
// fn evaluate_input(db: &Database, id: SourceId<Input>) -> Value {
//     let ast = parse_ast(db, id).expect("ast must be correct");
//     let result = eval(ast.expression).expect("value must be evaluated");
//     Value(result)
// }
fn evaluate_input<'db>(db: &'db Database, id: SourceId<Input>) -> &'db Value {
    // {
    //     ::std::io::_eprint(format_args!("about to intern {0:?}\n", (id.clone(),)));
    // };
    let param_id = ::pico::macro_fns::intern_param(db, (id.clone(),));
    let derived_node_id = ::pico::DerivedNodeId::new(9351471516276861273u64.into(), param_id);
    // {
    //     ::std::io::_eprint(format_args!("about to call memo\n"));
    // };
    ::pico::memo(
        db,
        derived_node_id,
        ::pico::InnerFn::new(|db, param_id| {
            // {
            //     ::std::io::_eprint(format_args!("inner function called\n"));
            // };
            let param_ref = ::pico::macro_fns::get_param(db, param_id)
                .expect("param should exist. This is indicative of a bug in Pico.");
            let (id,) = {
                let (id,) = param_ref
                    .downcast_ref::<(SourceId<Input>,)>()
                    .expect("param type must to be correct. This is indicative of a bug in Pico.");
                (id.clone(),)
            };
            // {
            //     ::std::io::_eprint(format_args!("now calling actual inner\n"));
            // };
            let value: Value = (|| {
                let ast = parse_ast(db, id).expect("ast must be correct");
                let result = eval(ast.expression).expect("value must be evaluated");
                Value(result)
            })();
            // {
            //     ::std::io::_eprint(format_args!("called\n"));
            // };
            Box::new(value)
        }),
    );
    ::pico::macro_fns::get_derived_node(db, derived_node_id)
        .expect("derived node must exist. This is indicative of a bug in Pico.")
        .value
        .as_any()
        .downcast_ref::<Value>()
        .expect("unexpected return type. This is indicative of a bug in Pico.")
}

#[memo]
fn assert_result(_db: &Database, result: &Value, expected: Expected) -> bool {
    result.0 == expected.0
}
