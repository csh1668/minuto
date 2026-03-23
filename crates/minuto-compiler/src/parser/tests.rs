use super::Parser;
use crate::ast::*;
use crate::lexer::Lexer;

fn parse_expr_ok(source: &str) -> ParsedExpr {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse_expr().expect("parse_expr failed")
}

fn parse_type_ok(source: &str) -> TypeExpr<Parsed> {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse_type().expect("parse_type failed")
}

fn parse_expr_err(source: &str) -> String {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse_expr().unwrap_err().message
}

fn parse_type_err(source: &str) -> String {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse_type().unwrap_err().message
}

fn lex(source: &str) -> Vec<(crate::lexer::token::Token, crate::common::Span)> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next_token() {
        tokens.push(result.expect("unexpected lexer error"));
    }
    tokens
}

// ══════════════════════════════════════════
//  parse_expr — Literals
// ══════════════════════════════════════════

#[test]
fn int_literal() {
    let e = parse_expr_ok("42");
    assert!(matches!(e.kind, ExprKind::IntLit(42)));
}

#[test]
fn char_literal() {
    let e = parse_expr_ok("'a'");
    assert!(matches!(e.kind, ExprKind::CharLit('a')));
}

#[test]
fn string_literal() {
    let e = parse_expr_ok(r#""hello""#);
    assert!(matches!(e.kind, ExprKind::StrLit(ref s) if s == "hello"));
}

// ══════════════════════════════════════════
//  parse_expr — Identifier
// ══════════════════════════════════════════

#[test]
fn ident() {
    let e = parse_expr_ok("foo");
    assert!(matches!(e.kind, ExprKind::Ident(Ident { ref name, .. }) if name == "foo"));
}

// ══════════════════════════════════════════
//  parse_expr — Parenthesized
// ══════════════════════════════════════════

#[test]
fn paren_expr() {
    let e = parse_expr_ok("(42)");
    assert!(matches!(e.kind, ExprKind::IntLit(42)));
}

#[test]
fn nested_parens() {
    let e = parse_expr_ok("((1 + 2))");
    assert!(matches!(e.kind, ExprKind::Binary(BinOp::Add, _, _)));
}

// ══════════════════════════════════════════
//  parse_expr — Unary operators
// ══════════════════════════════════════════

#[test]
fn unary_neg() {
    let e = parse_expr_ok("-x");
    assert!(matches!(e.kind, ExprKind::Unary(UnaryOp::Neg, _)));
}

#[test]
fn unary_not() {
    let e = parse_expr_ok("!x");
    assert!(matches!(e.kind, ExprKind::Unary(UnaryOp::Not, _)));
}

#[test]
fn unary_bitnot() {
    let e = parse_expr_ok("~x");
    assert!(matches!(e.kind, ExprKind::Unary(UnaryOp::BitNot, _)));
}

#[test]
fn unary_deref() {
    let e = parse_expr_ok("*p");
    assert!(matches!(e.kind, ExprKind::Unary(UnaryOp::Deref, _)));
}

#[test]
fn unary_addr_of() {
    let e = parse_expr_ok("&x");
    assert!(matches!(e.kind, ExprKind::Unary(UnaryOp::AddrOf, _)));
}

#[test]
fn chained_unary() {
    // -(-x)
    let e = parse_expr_ok("--x");
    match &e.kind {
        ExprKind::Unary(UnaryOp::Neg, inner) => {
            assert!(matches!(inner.kind, ExprKind::Unary(UnaryOp::Neg, _)));
        }
        _ => panic!("expected Unary(Neg, Unary(Neg, _))"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Binary operators & precedence
// ══════════════════════════════════════════

#[test]
fn binary_add() {
    let e = parse_expr_ok("1 + 2");
    assert!(matches!(e.kind, ExprKind::Binary(BinOp::Add, _, _)));
}

#[test]
fn binary_mul_higher_than_add() {
    // 1 + 2 * 3 → Add(1, Mul(2, 3))
    let e = parse_expr_ok("1 + 2 * 3");
    match &e.kind {
        ExprKind::Binary(BinOp::Add, lhs, rhs) => {
            assert!(matches!(lhs.kind, ExprKind::IntLit(1)));
            assert!(matches!(rhs.kind, ExprKind::Binary(BinOp::Mul, _, _)));
        }
        _ => panic!("expected Add(1, Mul(2, 3))"),
    }
}

#[test]
fn binary_left_associative() {
    // 1 - 2 - 3 → Sub(Sub(1, 2), 3)
    let e = parse_expr_ok("1 - 2 - 3");
    match &e.kind {
        ExprKind::Binary(BinOp::Sub, lhs, rhs) => {
            assert!(matches!(lhs.kind, ExprKind::Binary(BinOp::Sub, _, _)));
            assert!(matches!(rhs.kind, ExprKind::IntLit(3)));
        }
        _ => panic!("expected Sub(Sub(1, 2), 3)"),
    }
}

#[test]
fn binary_comparison() {
    let e = parse_expr_ok("a < b");
    assert!(matches!(e.kind, ExprKind::Binary(BinOp::Lt, _, _)));
}

#[test]
fn binary_equality() {
    let e = parse_expr_ok("a == b");
    assert!(matches!(e.kind, ExprKind::Binary(BinOp::Eq, _, _)));
}

#[test]
fn binary_logical_and_or() {
    // a || b && c → Or(a, And(b, c))
    let e = parse_expr_ok("a || b && c");
    match &e.kind {
        ExprKind::Binary(BinOp::Or, _, rhs) => {
            assert!(matches!(rhs.kind, ExprKind::Binary(BinOp::And, _, _)));
        }
        _ => panic!("expected Or(a, And(b, c))"),
    }
}

#[test]
fn binary_bitwise_precedence() {
    // a | b ^ c & d → BitOr(a, BitXor(b, BitAnd(c, d)))
    let e = parse_expr_ok("a | b ^ c & d");
    match &e.kind {
        ExprKind::Binary(BinOp::BitOr, _, rhs) => match &rhs.kind {
            ExprKind::Binary(BinOp::BitXor, _, inner_rhs) => {
                assert!(matches!(
                    inner_rhs.kind,
                    ExprKind::Binary(BinOp::BitAnd, _, _)
                ));
            }
            _ => panic!("expected BitXor"),
        },
        _ => panic!("expected BitOr"),
    }
}

#[test]
fn binary_shift() {
    let e = parse_expr_ok("a << 2");
    assert!(matches!(e.kind, ExprKind::Binary(BinOp::Shl, _, _)));
}

#[test]
fn binary_complex_precedence() {
    // 1 + 2 * 3 == 7 → Eq(Add(1, Mul(2, 3)), 7)
    let e = parse_expr_ok("1 + 2 * 3 == 7");
    match &e.kind {
        ExprKind::Binary(BinOp::Eq, lhs, rhs) => {
            assert!(matches!(lhs.kind, ExprKind::Binary(BinOp::Add, _, _)));
            assert!(matches!(rhs.kind, ExprKind::IntLit(7)));
        }
        _ => panic!("expected Eq(Add(...), 7)"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Assignment
// ══════════════════════════════════════════

#[test]
fn assignment() {
    let e = parse_expr_ok("x = 5");
    assert!(matches!(e.kind, ExprKind::Assign { .. }));
}

#[test]
fn assignment_right_assoc() {
    // a = b = 1 → Assign(a, Assign(b, 1))
    let e = parse_expr_ok("a = b = 1");
    match &e.kind {
        ExprKind::Assign { rhs, .. } => {
            assert!(matches!(rhs.kind, ExprKind::Assign { .. }));
        }
        _ => panic!("expected chained assignment"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Postfix: index
// ══════════════════════════════════════════

#[test]
fn index_access() {
    let e = parse_expr_ok("arr[0]");
    match &e.kind {
        ExprKind::Index { base, index } => {
            assert!(matches!(base.kind, ExprKind::Ident(Ident { ref name, .. }) if name == "arr"));
            assert!(matches!(index.kind, ExprKind::IntLit(0)));
        }
        _ => panic!("expected Index"),
    }
}

#[test]
fn index_with_expr() {
    let e = parse_expr_ok("arr[i + 1]");
    match &e.kind {
        ExprKind::Index { index, .. } => {
            assert!(matches!(index.kind, ExprKind::Binary(BinOp::Add, _, _)));
        }
        _ => panic!("expected Index with binary expr"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Postfix: field, arrow field
// ══════════════════════════════════════════

#[test]
fn field_access() {
    let e = parse_expr_ok("s.len");
    match &e.kind {
        ExprKind::Field { field, .. } => assert_eq!(field, "len"),
        _ => panic!("expected Field"),
    }
}

#[test]
fn arrow_field_access() {
    // p->x  →  Field { base: Deref(p), field: "x" }
    let e = parse_expr_ok("p->x");
    match &e.kind {
        ExprKind::Field { base, field } => {
            assert_eq!(field, "x");
            assert!(matches!(base.kind, ExprKind::Unary(UnaryOp::Deref, _)));
        }
        _ => panic!("expected Field(Deref(_))"),
    }
}

#[test]
fn chained_arrow_field() {
    // a->b->c → Field { base: Deref(Field { base: Deref(a), field: "b" }), field: "c" }
    let e = parse_expr_ok("a->b->c");
    match &e.kind {
        ExprKind::Field { base, field } => {
            assert_eq!(field, "c");
            match &base.kind {
                ExprKind::Unary(UnaryOp::Deref, inner) => {
                    assert!(
                        matches!(inner.kind, ExprKind::Field { ref field, .. } if field == "b")
                    );
                }
                _ => panic!("expected Deref(Field)"),
            }
        }
        _ => panic!("expected Field(Deref(Field(Deref(_))))"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Postfix: method call
// ══════════════════════════════════════════

#[test]
fn method_call_no_args() {
    let e = parse_expr_ok("c.increment()");
    match &e.kind {
        ExprKind::MethodCall { method, args, .. } => {
            assert_eq!(method, "increment");
            assert_eq!(args.len(), 0);
        }
        _ => panic!("expected MethodCall"),
    }
}

#[test]
fn method_call_with_args() {
    let e = parse_expr_ok("a.distance(b, c)");
    match &e.kind {
        ExprKind::MethodCall { method, args, .. } => {
            assert_eq!(method, "distance");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected MethodCall"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Postfix: function call
// ══════════════════════════════════════════

#[test]
fn function_call_no_args() {
    let e = parse_expr_ok("foo()");
    match &e.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 0),
        _ => panic!("expected Call"),
    }
}

#[test]
fn function_call_with_args() {
    let e = parse_expr_ok("add(1, 2)");
    match &e.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
        _ => panic!("expected Call"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Postfix chains
// ══════════════════════════════════════════

#[test]
fn chained_postfix() {
    // arr.ptr[0] → Index(Field(arr, "ptr"), 0)
    let e = parse_expr_ok("arr.ptr[0]");
    match &e.kind {
        ExprKind::Index { base, .. } => {
            assert!(matches!(base.kind, ExprKind::Field { ref field, .. } if field == "ptr"));
        }
        _ => panic!("expected Index(Field(...))"),
    }
}

#[test]
fn arrow_then_call() {
    // a->distance_sq(b) → MethodCall { base: Deref(a), method: "distance_sq", args: [b] }
    let e = parse_expr_ok("a->distance_sq(b)");
    match &e.kind {
        ExprKind::MethodCall { base, method, args } => {
            assert!(matches!(base.kind, ExprKind::Unary(UnaryOp::Deref, _)));
            assert_eq!(method, "distance_sq");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected MethodCall"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — self keyword
// ══════════════════════════════════════════

#[test]
fn self_keyword() {
    let e = parse_expr_ok("self");
    assert!(matches!(e.kind, ExprKind::Ident(Ident { ref name, .. }) if name == "self"));
}

#[test]
fn self_arrow_field() {
    // self->count  →  Field { base: Deref(self), field: "count" }
    let e = parse_expr_ok("self->count");
    match &e.kind {
        ExprKind::Field { base, field } => {
            assert_eq!(field, "count");
            match &base.kind {
                ExprKind::Unary(UnaryOp::Deref, inner) => {
                    assert!(
                        matches!(inner.kind, ExprKind::Ident(Ident { ref name, .. }) if name == "self")
                    );
                }
                _ => panic!("expected Deref(self)"),
            }
        }
        _ => panic!("expected Field(Deref(self))"),
    }
}

#[test]
fn self_arrow_method_call() {
    // self->get()  →  MethodCall { base: Deref(self), method: "get", args: [] }
    let e = parse_expr_ok("self->get()");
    match &e.kind {
        ExprKind::MethodCall { base, method, args } => {
            assert!(matches!(base.kind, ExprKind::Unary(UnaryOp::Deref, _)));
            assert_eq!(method, "get");
            assert_eq!(args.len(), 0);
        }
        _ => panic!("expected MethodCall"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — arrow method call
// ══════════════════════════════════════════

#[test]
fn arrow_method_no_args() {
    // p->reset()  →  MethodCall { base: Deref(p), ... }
    let e = parse_expr_ok("p->reset()");
    match &e.kind {
        ExprKind::MethodCall { base, method, args } => {
            assert!(matches!(base.kind, ExprKind::Unary(UnaryOp::Deref, _)));
            assert_eq!(method, "reset");
            assert_eq!(args.len(), 0);
        }
        _ => panic!("expected MethodCall"),
    }
}

#[test]
fn arrow_method_with_args() {
    // p->set(1, 2)  →  MethodCall { base: Deref(p), ... }
    let e = parse_expr_ok("p->set(1, 2)");
    match &e.kind {
        ExprKind::MethodCall { base, method, args } => {
            assert!(matches!(base.kind, ExprKind::Unary(UnaryOp::Deref, _)));
            assert_eq!(method, "set");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected MethodCall"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — alloc / free
// ══════════════════════════════════════════

#[test]
fn alloc_expr() {
    let e = parse_expr_ok("alloc<int>(10)");
    match &e.kind {
        ExprKind::Alloc { ty, count } => {
            assert!(matches!(ty, TypeExpr::Int(_)));
            assert!(matches!(count.kind, ExprKind::IntLit(10)));
        }
        _ => panic!("expected Alloc"),
    }
}

#[test]
fn alloc_nested_type() {
    let e = parse_expr_ok("alloc<ptr<int>>(n)");
    match &e.kind {
        ExprKind::Alloc { ty, .. } => {
            assert!(
                matches!(ty, TypeExpr::Ptr(_, inner) if matches!(inner.as_ref(), TypeExpr::Int(_)))
            );
        }
        _ => panic!("expected Alloc with ptr<int>"),
    }
}

#[test]
fn free_expr() {
    let e = parse_expr_ok("free(p)");
    match &e.kind {
        ExprKind::Free { expr } => {
            assert!(matches!(expr.kind, ExprKind::Ident(_)));
        }
        _ => panic!("expected Free"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Static calls (type keywords)
// ══════════════════════════════════════════

#[test]
fn span_new() {
    let e = parse_expr_ok("span::new(p, 10)");
    match &e.kind {
        ExprKind::StaticCall {
            receiver,
            method,
            args,
        } => {
            assert_eq!(*receiver, StaticReceiver::Span);
            assert_eq!(method, "new");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected StaticCall(Span, new)"),
    }
}

#[test]
fn int_parse() {
    let e = parse_expr_ok("int::parse(line)");
    match &e.kind {
        ExprKind::StaticCall {
            receiver,
            method,
            args,
        } => {
            assert_eq!(*receiver, StaticReceiver::Int);
            assert_eq!(method, "parse");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected StaticCall(Int, parse)"),
    }
}

#[test]
fn char_parse() {
    let e = parse_expr_ok("char::parse(s)");
    match &e.kind {
        ExprKind::StaticCall {
            receiver, method, ..
        } => {
            assert_eq!(*receiver, StaticReceiver::Char);
            assert_eq!(method, "parse");
        }
        _ => panic!("expected StaticCall(Char, parse)"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Static calls (named struct)
// ══════════════════════════════════════════

#[test]
fn named_static_call() {
    let e = parse_expr_ok("Counter::new()");
    match &e.kind {
        ExprKind::StaticCall {
            receiver,
            method,
            args,
        } => {
            assert_eq!(*receiver, StaticReceiver::Named("Counter".to_string()));
            assert_eq!(method, "new");
            assert_eq!(args.len(), 0);
        }
        _ => panic!("expected StaticCall(Named(Counter), new)"),
    }
}

#[test]
fn named_static_call_with_args() {
    let e = parse_expr_ok("Point::new(1, 2)");
    match &e.kind {
        ExprKind::StaticCall {
            receiver,
            method,
            args,
        } => {
            assert_eq!(*receiver, StaticReceiver::Named("Point".to_string()));
            assert_eq!(method, "new");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected StaticCall(Named(Point), new)"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — std calls
// ══════════════════════════════════════════

#[test]
fn std_print() {
    let e = parse_expr_ok(r#"std::print("{}\n", x)"#);
    match &e.kind {
        ExprKind::StdCall { func, args } => {
            assert_eq!(func, "print");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected StdCall(print)"),
    }
}

#[test]
fn std_input() {
    let e = parse_expr_ok("std::input()");
    match &e.kind {
        ExprKind::StdCall { func, args } => {
            assert_eq!(func, "input");
            assert_eq!(args.len(), 0);
        }
        _ => panic!("expected StdCall(input)"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Struct literal
// ══════════════════════════════════════════

#[test]
fn struct_literal() {
    let e = parse_expr_ok("Point { x: 10, y: 20 }");
    match &e.kind {
        ExprKind::StructLit { name, fields } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
        }
        _ => panic!("expected StructLit"),
    }
}

#[test]
fn struct_literal_trailing_comma() {
    let e = parse_expr_ok("Counter { count: 0, }");
    match &e.kind {
        ExprKind::StructLit { name, fields } => {
            assert_eq!(name, "Counter");
            assert_eq!(fields.len(), 1);
        }
        _ => panic!("expected StructLit"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Complex expressions (spec)
// ══════════════════════════════════════════

#[test]
fn deref_assign() {
    // *p = 42 → Assign(Unary(Deref, p), 42)
    let e = parse_expr_ok("*p = 42");
    match &e.kind {
        ExprKind::Assign { lhs, rhs } => {
            assert!(matches!(lhs.kind, ExprKind::Unary(UnaryOp::Deref, _)));
            assert!(matches!(rhs.kind, ExprKind::IntLit(42)));
        }
        _ => panic!("expected Assign(Deref, 42)"),
    }
}

#[test]
fn arrow_assign() {
    // p->x = 10 → Assign(Field { base: Deref(p), field: "x" }, 10)
    let e = parse_expr_ok("p->x = 10");
    match &e.kind {
        ExprKind::Assign { lhs, rhs } => {
            assert!(matches!(lhs.kind, ExprKind::Field { ref field, .. } if field == "x"));
            assert!(matches!(rhs.kind, ExprKind::IntLit(10)));
        }
        _ => panic!("expected Assign(Field(Deref(_)), 10)"),
    }
}

#[test]
fn index_assign() {
    // arr[0] = 42 → Assign(Index(arr, 0), 42)
    let e = parse_expr_ok("arr[0] = 42");
    match &e.kind {
        ExprKind::Assign { lhs, rhs } => {
            assert!(matches!(lhs.kind, ExprKind::Index { .. }));
            assert!(matches!(rhs.kind, ExprKind::IntLit(42)));
        }
        _ => panic!("expected Assign(Index, 42)"),
    }
}

#[test]
fn unary_higher_than_binary() {
    // -a + b → Add(Neg(a), b)
    let e = parse_expr_ok("-a + b");
    match &e.kind {
        ExprKind::Binary(BinOp::Add, lhs, _) => {
            assert!(matches!(lhs.kind, ExprKind::Unary(UnaryOp::Neg, _)));
        }
        _ => panic!("expected Add(Neg(a), b)"),
    }
}

#[test]
fn addr_of_in_call() {
    // swap(&x, &y)
    let e = parse_expr_ok("swap(&x, &y)");
    match &e.kind {
        ExprKind::Call { args, .. } => {
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0].kind, ExprKind::Unary(UnaryOp::AddrOf, _)));
            assert!(matches!(args[1].kind, ExprKind::Unary(UnaryOp::AddrOf, _)));
        }
        _ => panic!("expected Call with addr-of args"),
    }
}

// ══════════════════════════════════════════
//  parse_expr — Errors
// ══════════════════════════════════════════

#[test]
fn error_empty_input() {
    let msg = parse_expr_err("");
    assert!(msg.contains("unexpected end of file"));
}

#[test]
fn error_unexpected_token() {
    let msg = parse_expr_err(";");
    assert!(msg.contains("expected expression"));
}

// ══════════════════════════════════════════
//  parse_type — Primitives
// ══════════════════════════════════════════

#[test]
fn type_int() {
    let t = parse_type_ok("int");
    assert!(matches!(t, TypeExpr::Int(_)));
}

#[test]
fn type_char() {
    let t = parse_type_ok("char");
    assert!(matches!(t, TypeExpr::Char(_)));
}

#[test]
fn type_void() {
    let t = parse_type_ok("void");
    assert!(matches!(t, TypeExpr::Void(_)));
}

// ══════════════════════════════════════════
//  parse_type — Pointer / Span
// ══════════════════════════════════════════

#[test]
fn type_ptr_int() {
    let t = parse_type_ok("ptr<int>");
    match t {
        TypeExpr::Ptr(_, inner) => assert!(matches!(*inner, TypeExpr::Int(_))),
        _ => panic!("expected Ptr<Int>"),
    }
}

#[test]
fn type_span_char() {
    let t = parse_type_ok("span<char>");
    match t {
        TypeExpr::Span(_, inner) => assert!(matches!(*inner, TypeExpr::Char(_))),
        _ => panic!("expected Span<Char>"),
    }
}

#[test]
fn type_nested_ptr() {
    // ptr<ptr<int>>
    let t = parse_type_ok("ptr<ptr<int>>");
    match t {
        TypeExpr::Ptr(_, inner) => match *inner {
            TypeExpr::Ptr(_, inner2) => assert!(matches!(*inner2, TypeExpr::Int(_))),
            _ => panic!("expected Ptr<Ptr<Int>>"),
        },
        _ => panic!("expected Ptr"),
    }
}

// ══════════════════════════════════════════
//  parse_type — Readonly
// ══════════════════════════════════════════

#[test]
fn type_readonly_span() {
    let t = parse_type_ok("readonly span<char>");
    match t {
        TypeExpr::Readonly(_, inner) => {
            assert!(matches!(*inner, TypeExpr::Span(_, _)));
        }
        _ => panic!("expected Readonly(Span<Char>)"),
    }
}

#[test]
fn type_readonly_ptr() {
    let t = parse_type_ok("readonly ptr<int>");
    match t {
        TypeExpr::Readonly(_, inner) => {
            assert!(matches!(*inner, TypeExpr::Ptr(_, _)));
        }
        _ => panic!("expected Readonly(Ptr<Int>)"),
    }
}

// ══════════════════════════════════════════
//  parse_type — Named
// ══════════════════════════════════════════

#[test]
fn type_named() {
    let t = parse_type_ok("Point");
    match t {
        TypeExpr::Named(_, name) => assert_eq!(name, "Point"),
        _ => panic!("expected Named(Point)"),
    }
}

#[test]
fn type_ptr_named() {
    let t = parse_type_ok("ptr<Counter>");
    match t {
        TypeExpr::Ptr(_, inner) => {
            assert!(matches!(*inner, TypeExpr::Named(_, ref n) if n == "Counter"));
        }
        _ => panic!("expected Ptr<Named(Counter)>"),
    }
}

// ══════════════════════════════════════════
//  parse_type — Function type
// ══════════════════════════════════════════

#[test]
fn type_fn_no_params() {
    let t = parse_type_ok("fn() -> int");
    match t {
        TypeExpr::Fn { params, ret, .. } => {
            assert_eq!(params.len(), 0);
            assert!(matches!(*ret, TypeExpr::Int(_)));
        }
        _ => panic!("expected Fn() -> Int"),
    }
}

#[test]
fn type_fn_with_params() {
    let t = parse_type_ok("fn(int, int) -> int");
    match t {
        TypeExpr::Fn { params, ret, .. } => {
            assert_eq!(params.len(), 2);
            assert!(matches!(params[0], TypeExpr::Int(_)));
            assert!(matches!(params[1], TypeExpr::Int(_)));
            assert!(matches!(*ret, TypeExpr::Int(_)));
        }
        _ => panic!("expected Fn(Int, Int) -> Int"),
    }
}

#[test]
fn type_fn_void_return() {
    // fn() without -> should default to void
    let t = parse_type_ok("fn()");
    match t {
        TypeExpr::Fn { ret, .. } => {
            assert!(matches!(*ret, TypeExpr::Void(_)));
        }
        _ => panic!("expected Fn() -> Void"),
    }
}

#[test]
fn type_fn_complex_params() {
    let t = parse_type_ok("fn(ptr<int>, span<char>) -> void");
    match t {
        TypeExpr::Fn { params, .. } => {
            assert_eq!(params.len(), 2);
            assert!(matches!(params[0], TypeExpr::Ptr(_, _)));
            assert!(matches!(params[1], TypeExpr::Span(_, _)));
        }
        _ => panic!("expected Fn with complex params"),
    }
}

#[test]
fn type_fn_as_param() {
    // fn(fn(int) -> int) -> int
    let t = parse_type_ok("fn(fn(int) -> int) -> int");
    match t {
        TypeExpr::Fn { params, .. } => {
            assert_eq!(params.len(), 1);
            assert!(matches!(params[0], TypeExpr::Fn { .. }));
        }
        _ => panic!("expected higher-order Fn type"),
    }
}

// ══════════════════════════════════════════
//  parse_type — Errors
// ══════════════════════════════════════════

#[test]
fn type_error_empty() {
    let msg = parse_type_err("");
    assert!(msg.contains("unexpected end of file"));
}

#[test]
fn type_error_unexpected() {
    let msg = parse_type_err("42");
    assert!(msg.contains("expected type"));
}

// ══════════════════════════════════════════
//  Helpers for statement / block parsing
// ══════════════════════════════════════════

fn parse_stmt_ok(source: &str) -> ParsedStmt {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse_stmt().expect("parse_stmt failed")
}

fn parse_stmt_err(source: &str) -> Vec<String> {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser
        .parse_stmt()
        .unwrap_err()
        .into_iter()
        .map(|d| d.message)
        .collect()
}

fn parse_block_ok(source: &str) -> Block<Parsed> {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse_block().expect("parse_block failed")
}

fn parse_block_err(source: &str) -> Vec<String> {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser
        .parse_block()
        .unwrap_err()
        .into_iter()
        .map(|d| d.message)
        .collect()
}

// ══════════════════════════════════════════
//  parse_stmt — VarDecl
// ══════════════════════════════════════════

#[test]
fn var_decl_with_type() {
    let s = parse_stmt_ok("var x: int = 42;");
    match &s.kind {
        StmtKind::VarDecl { name, ty, init } => {
            assert_eq!(name.name, "x");
            assert!(ty.is_some());
            assert!(matches!(init.kind, ExprKind::IntLit(42)));
        }
        _ => panic!("expected VarDecl"),
    }
}

#[test]
fn var_decl_without_type() {
    let s = parse_stmt_ok("var y = 10;");
    match &s.kind {
        StmtKind::VarDecl { name, ty, init } => {
            assert_eq!(name.name, "y");
            assert!(ty.is_none());
            assert!(matches!(init.kind, ExprKind::IntLit(10)));
        }
        _ => panic!("expected VarDecl"),
    }
}

// ══════════════════════════════════════════
//  parse_stmt — ConstDecl
// ══════════════════════════════════════════

#[test]
fn const_decl() {
    let s = parse_stmt_ok("const MAX: int = 100;");
    match &s.kind {
        StmtKind::ConstDecl { name, init, .. } => {
            assert_eq!(name.name, "MAX");
            assert!(matches!(init.kind, ExprKind::IntLit(100)));
        }
        _ => panic!("expected ConstDecl"),
    }
}

// ══════════════════════════════════════════
//  parse_stmt — Return
// ══════════════════════════════════════════

#[test]
fn return_void() {
    let s = parse_stmt_ok("return;");
    assert!(matches!(s.kind, StmtKind::Return(None)));
}

#[test]
fn return_with_expr() {
    let s = parse_stmt_ok("return 42;");
    match &s.kind {
        StmtKind::Return(Some(expr)) => {
            assert!(matches!(expr.kind, ExprKind::IntLit(42)));
        }
        _ => panic!("expected Return(Some(_))"),
    }
}

// ══════════════════════════════════════════
//  parse_stmt — Break / Continue
// ══════════════════════════════════════════

#[test]
fn break_stmt() {
    let s = parse_stmt_ok("break;");
    assert!(matches!(s.kind, StmtKind::Break));
}

#[test]
fn continue_stmt() {
    let s = parse_stmt_ok("continue;");
    assert!(matches!(s.kind, StmtKind::Continue));
}

// ══════════════════════════════════════════
//  parse_stmt — Expression statement
// ══════════════════════════════════════════

#[test]
fn expr_stmt() {
    let s = parse_stmt_ok("foo();");
    match &s.kind {
        StmtKind::Expr(expr) => {
            assert!(matches!(expr.kind, ExprKind::Call { .. }));
        }
        _ => panic!("expected Expr statement"),
    }
}

// ══════════════════════════════════════════
//  parse_stmt — If / else if / else
// ══════════════════════════════════════════

#[test]
fn if_simple() {
    let s = parse_stmt_ok("if x { return; }");
    match &s.kind {
        StmtKind::If {
            else_ifs,
            else_block,
            ..
        } => {
            assert!(else_ifs.is_empty());
            assert!(else_block.is_none());
        }
        _ => panic!("expected If"),
    }
}

#[test]
fn if_else() {
    let s = parse_stmt_ok("if x { return; } else { break; }");
    match &s.kind {
        StmtKind::If {
            else_ifs,
            else_block,
            ..
        } => {
            assert!(else_ifs.is_empty());
            assert!(else_block.is_some());
        }
        _ => panic!("expected If with else"),
    }
}

#[test]
fn if_else_if_else() {
    let s = parse_stmt_ok("if a { return; } else if b { break; } else { continue; }");
    match &s.kind {
        StmtKind::If {
            else_ifs,
            else_block,
            ..
        } => {
            assert_eq!(else_ifs.len(), 1);
            assert!(else_block.is_some());
        }
        _ => panic!("expected If with else-if and else"),
    }
}

// ══════════════════════════════════════════
//  parse_stmt — While
// ══════════════════════════════════════════

#[test]
fn while_stmt() {
    let s = parse_stmt_ok("while x { break; }");
    match &s.kind {
        StmtKind::While { body, .. } => {
            assert_eq!(body.stmts.len(), 1);
        }
        _ => panic!("expected While"),
    }
}

// ══════════════════════════════════════════
//  parse_block — basic & error recovery
// ══════════════════════════════════════════

#[test]
fn block_multiple_stmts() {
    let b = parse_block_ok("{ var x: int = 1; return x; }");
    assert_eq!(b.stmts.len(), 2);
}

#[test]
fn block_empty() {
    let b = parse_block_ok("{ }");
    assert_eq!(b.stmts.len(), 0);
}

#[test]
fn block_error_recovery() {
    // `var = 1;` is invalid (missing identifier), error recovery should skip to `;`
    // and then parse the valid `return;` statement — but since errors were collected,
    // we get Err with the diagnostics.
    let errs = parse_block_err("{ var = 1; return; }");
    assert!(!errs.is_empty());
}

// ══════════════════════════════════════════
//  Struct literal not parsed in if/while condition
// ══════════════════════════════════════════

#[test]
fn if_condition_no_struct_lit() {
    // `Point` should be parsed as an identifier, `{` starts the block
    let s = parse_stmt_ok("if Point { return; }");
    match &s.kind {
        StmtKind::If { cond, .. } => {
            assert!(
                matches!(cond.kind, ExprKind::Ident(Ident { ref name, .. }) if name == "Point")
            );
        }
        _ => panic!("expected If"),
    }
}

#[test]
fn while_condition_no_struct_lit() {
    let s = parse_stmt_ok("while x { break; }");
    match &s.kind {
        StmtKind::While { cond, .. } => {
            assert!(
                matches!(cond.kind, ExprKind::Ident(Ident { ref name, .. }) if name == "x")
            );
        }
        _ => panic!("expected While"),
    }
}

// ══════════════════════════════════════════
//  parse_stmt — Errors
// ══════════════════════════════════════════

#[test]
fn stmt_error_eof() {
    let errs = parse_stmt_err("");
    assert!(errs[0].contains("unexpected end of file"));
}

#[test]
fn stmt_error_missing_semicolon() {
    let errs = parse_stmt_err("return 42");
    assert!(errs[0].contains("Semicolon"));
}

// ══════════════════════════════════════════
//  Helpers for parse (full program)
// ══════════════════════════════════════════

fn parse_ok(source: &str) -> ParsedProgram {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser.parse().expect("parse failed")
}

fn parse_err(source: &str) -> Vec<String> {
    let tokens = lex(source);
    let mut parser = Parser::new(tokens);
    parser
        .parse()
        .unwrap_err()
        .into_iter()
        .map(|d| d.message)
        .collect()
}

// ══════════════════════════════════════════
//  parse — Function declarations
// ══════════════════════════════════════════

#[test]
fn fn_decl_no_params_void() {
    let prog = parse_ok("fn main() { }");
    assert_eq!(prog.decls.len(), 1);
    match &prog.decls[0] {
        Decl::Fn(f) => {
            assert_eq!(f.name.name, "main");
            assert!(f.params.is_empty());
            assert!(matches!(f.ret_ty, TypeExpr::Void(_)));
            assert!(f.body.stmts.is_empty());
        }
        _ => panic!("expected Fn decl"),
    }
}

#[test]
fn fn_decl_with_params_and_return() {
    let prog = parse_ok("fn add(a: int, b: int) -> int { return a + b; }");
    assert_eq!(prog.decls.len(), 1);
    match &prog.decls[0] {
        Decl::Fn(f) => {
            assert_eq!(f.name.name, "add");
            assert_eq!(f.params.len(), 2);
            assert!(matches!(f.ret_ty, TypeExpr::Int(_)));
            assert_eq!(f.body.stmts.len(), 1);
        }
        _ => panic!("expected Fn decl"),
    }
}

#[test]
fn fn_decl_complex_params() {
    let prog = parse_ok("fn foo(p: ptr<int>, s: span<char>) -> void { }");
    match &prog.decls[0] {
        Decl::Fn(f) => {
            assert_eq!(f.params.len(), 2);
            match &f.params[0].kind {
                ParamKind::Named { name, ty } => {
                    assert_eq!(name.name, "p");
                    assert!(matches!(ty, TypeExpr::Ptr(_, _)));
                }
                _ => panic!("expected Named param"),
            }
        }
        _ => panic!("expected Fn decl"),
    }
}

#[test]
fn fn_decl_fn_type_param() {
    let prog = parse_ok("fn apply(f: fn(int) -> int, x: int) -> int { return f(x); }");
    match &prog.decls[0] {
        Decl::Fn(f) => {
            assert_eq!(f.params.len(), 2);
            match &f.params[0].kind {
                ParamKind::Named { ty, .. } => {
                    assert!(matches!(ty, TypeExpr::Fn { .. }));
                }
                _ => panic!("expected Named param"),
            }
        }
        _ => panic!("expected Fn decl"),
    }
}

// ══════════════════════════════════════════
//  parse — Struct declarations
// ══════════════════════════════════════════

#[test]
fn struct_decl_fields_only() {
    let prog = parse_ok("struct Point { x: int, y: int, }");
    match &prog.decls[0] {
        Decl::Struct(s) => {
            assert_eq!(s.name.name, "Point");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "x");
            assert_eq!(s.fields[1].name, "y");
            assert!(s.methods.is_empty());
        }
        _ => panic!("expected Struct decl"),
    }
}

#[test]
fn struct_decl_with_methods() {
    let prog = parse_ok(
        r#"struct Counter {
            count: int,

            fn new() -> Counter {
                return Counter { count: 0 };
            }

            fn increment(self) {
                self->count = self->count + 1;
            }

            fn get(self) -> int {
                return self->count;
            }
        }"#,
    );
    match &prog.decls[0] {
        Decl::Struct(s) => {
            assert_eq!(s.name.name, "Counter");
            assert_eq!(s.fields.len(), 1);
            assert_eq!(s.methods.len(), 3);
            assert_eq!(s.methods[0].name.name, "new");
            assert_eq!(s.methods[1].name.name, "increment");
            assert_eq!(s.methods[2].name.name, "get");
        }
        _ => panic!("expected Struct decl"),
    }
}

#[test]
fn struct_self_param_no_type() {
    let prog = parse_ok(
        "struct Foo { fn bar(self) { } }",
    );
    match &prog.decls[0] {
        Decl::Struct(s) => {
            match &s.methods[0].params[0].kind {
                ParamKind::SelfParam { ty } => assert!(ty.is_none()),
                _ => panic!("expected SelfParam"),
            }
        }
        _ => panic!("expected Struct decl"),
    }
}

#[test]
fn struct_self_param_with_type() {
    let prog = parse_ok(
        "struct Foo { fn bar(self: ptr<Foo>) { } }",
    );
    match &prog.decls[0] {
        Decl::Struct(s) => {
            match &s.methods[0].params[0].kind {
                ParamKind::SelfParam { ty } => {
                    assert!(ty.is_some());
                    assert!(matches!(ty.as_ref().unwrap(), TypeExpr::Ptr(_, _)));
                }
                _ => panic!("expected SelfParam"),
            }
        }
        _ => panic!("expected Struct decl"),
    }
}

// ══════════════════════════════════════════
//  parse — Multiple declarations
// ══════════════════════════════════════════

#[test]
fn multiple_decls() {
    let prog = parse_ok(
        r#"
        struct Point { x: int, y: int, }
        fn main() { var p: Point = Point { x: 1, y: 2 }; }
        "#,
    );
    assert_eq!(prog.decls.len(), 2);
    assert!(matches!(prog.decls[0], Decl::Struct(_)));
    assert!(matches!(prog.decls[1], Decl::Fn(_)));
}

#[test]
fn full_program() {
    let prog = parse_ok(
        r#"
        struct Counter {
            count: int,

            fn new() -> Counter {
                return Counter { count: 0 };
            }

            fn get(self) -> int {
                return self->count;
            }
        }

        fn main() {
            var c: Counter = Counter::new();
            var n: int = c.get();
            if n == 0 {
                std::print("zero\n");
            } else {
                std::print("nonzero\n");
            }
        }
        "#,
    );
    assert_eq!(prog.decls.len(), 2);
}

// ══════════════════════════════════════════
//  parse — Errors
// ══════════════════════════════════════════

#[test]
fn parse_error_unexpected_top_level() {
    let errs = parse_err("var x = 1;");
    assert!(!errs.is_empty());
}

#[test]
fn parse_error_recovery_continues() {
    // First decl is broken (missing body), second should still parse
    let errs = parse_err("fn bad( fn main() { }");
    // We get errors but parsing attempted recovery
    assert!(!errs.is_empty());
}

#[test]
fn parse_empty_program() {
    let prog = parse_ok("");
    assert!(prog.decls.is_empty());
}
