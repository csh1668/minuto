use minuto_compiler::pipeline::{Lex, Parse, Pipeline, Resolve};

fn main() {
    let source = r#"
struct Foo {
    x: int,
    x: int,
}

fn main() {
    var x = 10 + 4;
}
    "#
    .trim();

    let result = Pipeline::start(Lex)
        .then(Parse)
        .then(Resolve)
        .run(source.to_string());

    match result {
        Ok((program, _symbols)) => {
            println!("Resolved program: {program:#?}");
        }
        Err(diagnostics) => {
            for d in &diagnostics {
                d.eprint("<source>", source);
            }
        }
    }
}
