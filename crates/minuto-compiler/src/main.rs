use minuto_compiler::{Lexer, Parser};

fn main() {
    let source = r#"
x = a + b * c
    "#.trim();

    let mut lexer = Lexer::new(source);
    let (tokens, errors) = lexer.tokenize();

    if !errors.is_empty() {
        eprintln!("Lexer errors:");
        for error in errors {
            eprintln!("{error}");
        }
        return;
    }

    let mut parser = Parser::new(tokens);
    match parser.parse_expr() {
        Ok(expr) => {
            println!("Parsed expr: {expr:#?}");
        }
        Err(error) => {
            eprintln!("Parser error:");
            eprintln!("{error}");
        }
    }
}
