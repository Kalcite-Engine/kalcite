//! Semantic checks performed after HIR lowering and before MIR/code generation.

use std::collections::BTreeMap;

use kalcite_hir::{AssignOp, BinaryOp, Expr, Function, Program, Stmt, Type, UnaryOp};
use kalcite_syntax::{Diagnostic, Span};

type Result<T> = std::result::Result<T, Diagnostic>;

pub fn check(program: &Program) -> Result<()> {
    let mut globals = BTreeMap::new();
    for field in &program.constants {
        if let Some(value) = &field.init {
            expect(
                &field.ty,
                expr_type(value, &globals, None)?,
                "constant initializer",
            )?;
        }
        globals.insert(field.name.clone(), field.ty.clone());
    }

    let mut functions = BTreeMap::new();
    for function in &program.functions {
        functions.insert(function.name.clone(), function);
    }
    for class in &program.classes {
        for function in &class.functions {
            functions.insert(function.name.clone(), function);
        }
    }

    for class in &program.classes {
        let mut fields = globals.clone();
        for field in &class.fields {
            if let Some(value) = &field.init {
                expect(
                    &field.ty,
                    expr_type(value, &fields, Some(&functions))?,
                    "field initializer",
                )?;
            }
            fields.insert(field.name.clone(), field.ty.clone());
        }
        for function in &class.functions {
            check_function(function, &fields, &functions)?;
        }
    }
    for function in &program.functions {
        check_function(function, &globals, &functions)?;
    }
    Ok(())
}

fn check_function(
    function: &Function,
    base: &BTreeMap<String, Type>,
    functions: &BTreeMap<String, &Function>,
) -> Result<()> {
    let mut symbols = base.clone();
    for parameter in &function.params {
        symbols.insert(parameter.name.clone(), parameter.ty.clone());
    }
    check_body(&function.body, &mut symbols, &function.ret, functions, 0)
}

fn check_body(
    body: &[Stmt],
    symbols: &mut BTreeMap<String, Type>,
    return_type: &Type,
    functions: &BTreeMap<String, &Function>,
    loop_depth: usize,
) -> Result<()> {
    for statement in body {
        match statement {
            Stmt::Expr(expr) => {
                expr_type(expr, symbols, Some(functions))?;
            }
            Stmt::Defer(expr) => {
                // A deferred expression has the same type rules as an ordinary
                // expression statement; its value, if any, is discarded.
                expr_type(expr, symbols, Some(functions))?;
            }
            Stmt::Local {
                name, ty, value, ..
            } => {
                let value_type = value
                    .as_ref()
                    .map(|value| expr_type(value, symbols, Some(functions)))
                    .transpose()?;
                let declared = ty
                    .clone()
                    .or(value_type.clone())
                    .ok_or_else(|| error("local declaration needs a type or initializer"))?;
                if let Some(value_type) = value_type {
                    expect(&declared, value_type, "local initializer")?;
                }
                symbols.insert(name.clone(), declared);
            }
            Stmt::Assign { target, op, value } => {
                let target_type = expr_type(target, symbols, Some(functions))?;
                let value_type = expr_type(value, symbols, Some(functions))?;
                if *op == AssignOp::Set {
                    expect(&target_type, value_type, "assignment")?;
                } else {
                    numeric(&target_type, "compound assignment target")?;
                    numeric(&value_type, "compound assignment value")?;
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                expect(
                    &Type::Bool,
                    expr_type(condition, symbols, Some(functions))?,
                    "if condition",
                )?;
                check_body(
                    then_body,
                    &mut symbols.clone(),
                    return_type,
                    functions,
                    loop_depth,
                )?;
                check_body(
                    else_body,
                    &mut symbols.clone(),
                    return_type,
                    functions,
                    loop_depth,
                )?;
            }
            Stmt::While { condition, body } => {
                expect(
                    &Type::Bool,
                    expr_type(condition, symbols, Some(functions))?,
                    "while condition",
                )?;
                check_body(
                    body,
                    &mut symbols.clone(),
                    return_type,
                    functions,
                    loop_depth + 1,
                )?;
            }
            Stmt::Break if loop_depth == 0 => {
                return Err(error("`break` is only valid inside a while loop"));
            }
            Stmt::Break => {}
            Stmt::Continue if loop_depth == 0 => {
                return Err(error("`continue` is only valid inside a while loop"));
            }
            Stmt::Continue => {}
            Stmt::Return(value) => match value {
                Some(value) => expect(
                    return_type,
                    expr_type(value, symbols, Some(functions))?,
                    "return value",
                )?,
                None if *return_type != Type::Void => {
                    return Err(error("non-void function must return a value"));
                }
                None => {}
            },
            Stmt::Native { .. } => {}
        }
    }
    Ok(())
}

fn expr_type(
    expr: &Expr,
    symbols: &BTreeMap<String, Type>,
    functions: Option<&BTreeMap<String, &Function>>,
) -> Result<Type> {
    match expr {
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::String(value) => Ok(Type::BoundedString(value.len())),
        Expr::Number(value) => Ok(if value.contains('.') {
            Type::Fx8
        } else {
            Type::I32
        }),
        Expr::Path(path) if path.len() == 1 => symbols
            .get(&path[0])
            .cloned()
            .ok_or_else(|| error(&format!("unknown name `{}`", path[0]))),
        Expr::Path(_) => Ok(Type::Named("qualified value".into())),
        Expr::Array(values) => {
            let Some(first) = values.first() else {
                return Ok(Type::FixedArray(Box::new(Type::Void), 0));
            };
            let item = expr_type(first, symbols, functions)?;
            for value in &values[1..] {
                expect(
                    &item,
                    expr_type(value, symbols, functions)?,
                    "array element",
                )?;
            }
            Ok(Type::FixedArray(Box::new(item), values.len()))
        }
        Expr::Call { callee, args } => {
            if let Expr::Path(path) = callee.as_ref()
                && path.len() == 1
                && let Some(function) = functions.and_then(|functions| functions.get(&path[0]))
            {
                if args.len() != function.params.len() {
                    return Err(error(&format!(
                        "`{}` expects {} arguments, got {}",
                        path[0],
                        function.params.len(),
                        args.len()
                    )));
                }
                for (argument, parameter) in args.iter().zip(&function.params) {
                    expect(
                        &parameter.ty,
                        expr_type(argument, symbols, functions)?,
                        "function argument",
                    )?;
                }
                return Ok(function.ret.clone());
            }
            for argument in args {
                expr_type(argument, symbols, functions)?;
            }
            Ok(Type::Named("call result".into()))
        }
        Expr::Index { base, index } => {
            let base = expr_type(base, symbols, functions)?;
            let index = expr_type(index, symbols, functions)?;
            numeric(&index, "array index")?;
            match base {
                Type::FixedArray(item, _) => Ok(*item),
                other => Err(error(&format!(
                    "index target must be a fixed array, got {other:?}"
                ))),
            }
        }
        Expr::Unary { op, value } => {
            let value = expr_type(value, symbols, functions)?;
            match op {
                UnaryOp::Not => {
                    expect(&Type::Bool, value, "! operand")?;
                    Ok(Type::Bool)
                }
                UnaryOp::Neg => {
                    numeric(&value, "- operand")?;
                    Ok(value)
                }
            }
        }
        Expr::Binary { left, op, right } => {
            let left = expr_type(left, symbols, functions)?;
            let right = expr_type(right, symbols, functions)?;
            match op {
                BinaryOp::And | BinaryOp::Or => {
                    expect(&Type::Bool, left, "logical operand")?;
                    expect(&Type::Bool, right, "logical operand")?;
                    Ok(Type::Bool)
                }
                BinaryOp::Eq | BinaryOp::Ne => {
                    expect(&left, right, "comparison")?;
                    Ok(Type::Bool)
                }
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                    numeric(&left, "comparison operand")?;
                    numeric(&right, "comparison operand")?;
                    Ok(Type::Bool)
                }
                _ => {
                    numeric(&left, "binary operand")?;
                    numeric(&right, "binary operand")?;
                    Ok(left)
                }
            }
        }
    }
}

fn expect(expected: &Type, actual: Type, context: &str) -> Result<()> {
    if compatible(expected, &actual) {
        Ok(())
    } else {
        Err(error(&format!(
            "{context} has type {actual:?}, expected {expected:?}"
        )))
    }
}

fn compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual
        || matches!(actual, Type::Named(name) if name == "call result" || name == "qualified value")
        || matches!(
            (expected, actual),
            (
                Type::U8 | Type::I8 | Type::U16 | Type::I16 | Type::U32 | Type::I32 | Type::Fx8,
                Type::I32
            ) | (Type::BoundedString(_), Type::BoundedString(_))
                | (Type::Named(_), Type::Named(_))
        )
}

fn numeric(ty: &Type, context: &str) -> Result<()> {
    if matches!(
        ty,
        Type::U8 | Type::I8 | Type::U16 | Type::I16 | Type::U32 | Type::I32 | Type::Fx8
    ) {
        Ok(())
    } else {
        Err(error(&format!("{context} must be numeric, got {ty:?}")))
    }
}

fn error(message: &str) -> Diagnostic {
    Diagnostic {
        message: message.into(),
        span: Span { start: 0, end: 0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kalcite_hir::lower;
    use kalcite_syntax::parse;

    #[test]
    fn rejects_a_boolean_u8_initializer() {
        let program = lower(&parse("public class Main { u8 value = true; }").unwrap()).unwrap();
        assert!(check(&program).is_err());
    }

    #[test]
    fn rejects_break_outside_a_while_loop() {
        let program = lower(&parse("fn update() -> void { break; }").unwrap()).unwrap();
        let error = check(&program).unwrap_err();
        assert_eq!(error.message, "`break` is only valid inside a while loop");
    }

    #[test]
    fn accepts_break_inside_a_while_loop() {
        let program =
            lower(&parse("fn update() -> void { while true { break; } }").unwrap()).unwrap();
        check(&program).unwrap();
    }

    #[test]
    fn accepts_fixed_array_indexing() {
        let program = lower(
            &parse("i16 read([i16; 2] values, i16 index) { return values[index]; }").unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
    }

    #[test]
    fn rejects_non_numeric_array_indices() {
        let program =
            lower(&parse("i16 read([i16; 2] values) { return values[true]; }").unwrap()).unwrap();
        let error = check(&program).unwrap_err();
        assert_eq!(error.message, "array index must be numeric, got Bool");
    }

    #[test]
    fn rejects_continue_outside_a_while_loop() {
        let program = lower(&parse("fn update() -> void { continue; }").unwrap()).unwrap();
        let error = check(&program).unwrap_err();
        assert_eq!(
            error.message,
            "`continue` is only valid inside a while loop"
        );
    }

    #[test]
    fn accepts_continue_inside_a_while_loop() {
        let program =
            lower(&parse("fn update() -> void { while true { continue; } }").unwrap()).unwrap();
        check(&program).unwrap();
    }
}
