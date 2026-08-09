use kalcite_syntax::{
    Attribute, Class as AstClass, Diagnostic, Item, Member, Module, Span, Token, TokenKind, lex,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub classes: Vec<Class>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    pub path: Vec<String>,
    pub name: String,
    pub attrs: Vec<Attribute>,
    pub base: Option<String>,
    pub fields: Vec<Field>,
    pub functions: Vec<Function>,
}

impl Class {
    pub fn is_scene(&self) -> bool {
        self.attrs.iter().any(|a| a.name == "scene")
    }
    pub fn rust_name(&self) -> String {
        self.path.join("_")
    }
    pub fn pool_capacity(&self) -> Option<usize> {
        self.attrs
            .iter()
            .find(|a| a.name == "pool")
            .and_then(|a| a.args.first())
            .and_then(|n| n.parse().ok())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub init: Option<Expr>,
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub owner: Option<Vec<String>>,
    pub name: String,
    pub params: Vec<Field>,
    pub ret: Type,
    pub body: Vec<Stmt>,
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Void,
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    Fx8,
    Vec2fx,
    FixedArray(Box<Type>, usize),
    Handle(Box<Type>),
    Pool(Box<Type>, usize),
    Named(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Local {
        name: String,
        ty: Option<Type>,
        mutable: bool,
        value: Option<Expr>,
    },
    Native {
        language: NativeLanguage,
        target: Option<String>,
        body: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeLanguage {
    Rust,
    Asm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(String),
    Bool(bool),
    String(String),
    Path(Vec<String>),
    Array(Vec<Expr>),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

pub fn lower(module: &Module) -> Result<Program, Diagnostic> {
    let mut p = Program {
        classes: Vec::new(),
        functions: Vec::new(),
    };
    for item in &module.items {
        match item {
            Item::Class(c) => lower_class(c, &[], &mut p)?,
            Item::Function(f) => p.functions.push(lower_function(None, f)?),
            Item::Struct(_) | Item::Use(_) => {}
        }
    }
    Ok(p)
}

fn lower_class(c: &AstClass, parent: &[String], p: &mut Program) -> Result<(), Diagnostic> {
    let mut path = parent.to_vec();
    path.push(c.name.clone());
    let mut fields = Vec::new();
    let mut functions = Vec::new();
    for m in &c.members {
        match m {
            Member::Field(f) => fields.push(Field {
                name: f.name.clone(),
                ty: parse_type(&f.ty),
                mutable: f.mutable,
                init: f.init.as_deref().map(parse_expr_text).transpose()?,
                attrs: f.attrs.clone(),
            }),
            Member::Function(f) => functions.push(lower_function(Some(path.clone()), f)?),
            Member::Class(nested) => lower_class(nested, &path, p)?,
            Member::Signal(_) => {}
        }
    }
    p.classes.push(Class {
        path,
        name: c.name.clone(),
        attrs: c.attrs.clone(),
        base: c.base.clone(),
        fields,
        functions,
    });
    Ok(())
}

fn lower_function(
    owner: Option<Vec<String>>,
    f: &kalcite_syntax::Function,
) -> Result<Function, Diagnostic> {
    let params = f
        .params
        .iter()
        .map(|p| Field {
            name: p.name.clone(),
            ty: parse_type(&p.ty),
            mutable: false,
            init: None,
            attrs: Vec::new(),
        })
        .collect();
    Ok(Function {
        owner,
        name: f.name.clone(),
        params,
        ret: f.ret.as_deref().map(parse_type).unwrap_or(Type::Void),
        body: parse_body(&f.body)?,
        attrs: f.attrs.clone(),
    })
}

pub fn parse_type(text: &str) -> Type {
    let t = text.trim();
    match t {
        "void" => Type::Void,
        "bool" => Type::Bool,
        "u8" => Type::U8,
        "i8" => Type::I8,
        "u16" => Type::U16,
        "i16" => Type::I16,
        "u32" => Type::U32,
        "i32" => Type::I32,
        "fx8" => Type::Fx8,
        "Vec2fx" => Type::Vec2fx,
        _ if t.starts_with('[') && t.ends_with(']') => {
            let inner = &t[1..t.len() - 1];
            if let Some((ty, n)) = inner.rsplit_once(';') {
                if let Ok(n) = n.trim().parse() {
                    return Type::FixedArray(Box::new(parse_type(ty)), n);
                }
            }
            Type::Named(t.into())
        }
        _ if t.starts_with("Handle[") && t.ends_with(']') => {
            Type::Handle(Box::new(parse_type(&t[7..t.len() - 1])))
        }
        _ if t.starts_with("Pool[") && t.ends_with(']') => {
            let inner = &t[5..t.len() - 1];
            if let Some((ty, n)) = inner.rsplit_once(';') {
                if let Ok(n) = n.trim().parse() {
                    return Type::Pool(Box::new(parse_type(ty)), n);
                }
            }
            Type::Named(t.into())
        }
        _ => Type::Named(t.into()),
    }
}

pub fn parse_expr_text(text: &str) -> Result<Expr, Diagnostic> {
    let mut p = BodyParser::new(text)?;
    let e = p.expr(0)?;
    if !matches!(p.peek(), TokenKind::Eof) {
        return p.err("unexpected token after expression");
    };
    Ok(e)
}
pub fn parse_body(text: &str) -> Result<Vec<Stmt>, Diagnostic> {
    BodyParser::new(text)?.body_until_eof()
}

struct BodyParser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    at: usize,
}
impl<'a> BodyParser<'a> {
    fn new(src: &'a str) -> Result<Self, Diagnostic> {
        Ok(Self {
            src,
            tokens: lex(src)?,
            at: 0,
        })
    }
    fn body_until_eof(mut self) -> Result<Vec<Stmt>, Diagnostic> {
        let mut out = Vec::new();
        while !matches!(self.peek(), TokenKind::Eof) {
            out.push(self.stmt()?)
        }
        Ok(out)
    }
    fn block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect(TokenKind::LBrace)?;
        let mut out = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            out.push(self.stmt()?)
        }
        self.expect(TokenKind::RBrace)?;
        Ok(out)
    }
    fn stmt(&mut self) -> Result<Stmt, Diagnostic> {
        match self.peek() {
            TokenKind::NativeBlock { .. } => {
                let TokenKind::NativeBlock {
                    language,
                    target,
                    body,
                } = self.peek().clone()
                else {
                    unreachable!()
                };
                self.bump();
                let language = match language.as_str() {
                    "rust" => NativeLanguage::Rust,
                    "asm" => NativeLanguage::Asm,
                    _ => return self.err("unknown native language"),
                };
                if language == NativeLanguage::Asm && target.is_none() {
                    return self.err("unsafe asm requires an explicit target, for example unsafe asm[numworks] { ... }");
                }
                if let Some(t) = target.as_deref() {
                    if !matches!(
                        t,
                        "numworks" | "desktop" | "linux" | "windows" | "macos" | "web" | "wasm"
                    ) {
                        return self.err("unknown native target; expected numworks, desktop, linux, windows, macos, web, or wasm");
                    }
                }
                Ok(Stmt::Native {
                    language,
                    target,
                    body,
                })
            }
            TokenKind::If => {
                self.bump();
                let c = self.condition()?;
                let t = self.block()?;
                let e = if matches!(self.peek(), TokenKind::Else) {
                    self.bump();
                    if matches!(self.peek(), TokenKind::If) {
                        vec![self.stmt()?]
                    } else {
                        self.block()?
                    }
                } else {
                    Vec::new()
                };
                Ok(Stmt::If {
                    condition: c,
                    then_body: t,
                    else_body: e,
                })
            }
            TokenKind::While => {
                self.bump();
                let c = self.condition()?;
                Ok(Stmt::While {
                    condition: c,
                    body: self.block()?,
                })
            }
            TokenKind::Return => {
                self.bump();
                if matches!(self.peek(), TokenKind::Semicolon) {
                    self.bump();
                    Ok(Stmt::Return(None))
                } else {
                    let e = self.expr(0)?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Return(Some(e)))
                }
            }
            TokenKind::Var | TokenKind::Const => self.local(),
            _ => {
                if let Some(local) = self.try_typed_local(true)? {
                    return Ok(local);
                }
                // Parse only a primary/postfix expression first so `+=`, `-=`, `*=`, `/=`
                // are not swallowed as binary operators by the Pratt parser. If this is
                // not an assignment, rewind and parse the full expression statement.
                let checkpoint = self.at;
                let lhs = self.expr(10)?;
                let op = match self.peek() {
                    TokenKind::Assign => Some(AssignOp::Set),
                    TokenKind::Plus if self.next_is_assign() => Some(AssignOp::Add),
                    TokenKind::Minus if self.next_is_assign() => Some(AssignOp::Sub),
                    TokenKind::Star if self.next_is_assign() => Some(AssignOp::Mul),
                    TokenKind::Slash if self.next_is_assign() => Some(AssignOp::Div),
                    _ => None,
                };
                if let Some(op) = op {
                    self.bump();
                    if op != AssignOp::Set {
                        self.expect(TokenKind::Assign)?;
                    }
                    let value = self.expr(0)?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Assign {
                        target: lhs,
                        op,
                        value,
                    })
                } else {
                    self.at = checkpoint;
                    let expr = self.expr(0)?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }
    fn condition(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek(), TokenKind::LParen) {
            self.bump();
            let value = self.expr(0)?;
            self.expect(TokenKind::RParen)?;
            Ok(value)
        } else {
            self.expr(0)
        }
    }
    fn local(&mut self) -> Result<Stmt, Diagnostic> {
        let mutable = matches!(self.peek(), TokenKind::Var);
        self.bump();

        // `const u32 score = 0;` is accepted in addition to the legacy
        // `const score: u32 = 0;`. `var` intentionally keeps its inference-first
        // spelling; explicit C#-style locals do not need a `var` keyword.
        if !mutable {
            if let Some(local) = self.try_typed_local(false)? {
                return Ok(local);
            }
        }

        let name = self.ident()?;
        let ty = if matches!(self.peek(), TokenKind::Colon) {
            self.bump();
            Some(parse_type(&self.type_text_until_body(&[
                TokenKind::Assign,
                TokenKind::Semicolon,
            ])?))
        } else {
            None
        };
        let value = if matches!(self.peek(), TokenKind::Assign) {
            self.bump();
            Some(self.expr(0)?)
        } else {
            None
        };
        if ty.is_none() && value.is_none() {
            return self.err("local variable needs a type or initializer");
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Local {
            name,
            ty,
            mutable,
            value,
        })
    }
    fn try_typed_local(&mut self, mutable: bool) -> Result<Option<Stmt>, Diagnostic> {
        let checkpoint = self.at;
        let start = self.tokens[self.at].span.start;

        match self.peek() {
            TokenKind::Ident(_) => {
                self.bump();
                if matches!(self.peek(), TokenKind::LBracket) {
                    self.consume_balanced_square()?;
                }
            }
            TokenKind::LBracket => self.consume_balanced_square()?,
            _ => return Ok(None),
        }

        if !matches!(self.peek(), TokenKind::Ident(_)) {
            self.at = checkpoint;
            return Ok(None);
        }

        let type_end = self.tokens[self.at].span.start;
        let ty = parse_type(self.src[start..type_end].trim());
        let name = self.ident()?;

        // A typed local must end like a declaration. Otherwise this was an
        // expression that merely happened to begin with a type-looking token.
        if !matches!(self.peek(), TokenKind::Assign | TokenKind::Semicolon) {
            self.at = checkpoint;
            return Ok(None);
        }

        let value = if matches!(self.peek(), TokenKind::Assign) {
            self.bump();
            Some(self.expr(0)?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Some(Stmt::Local {
            name,
            ty: Some(ty),
            mutable,
            value,
        }))
    }
    fn consume_balanced_square(&mut self) -> Result<(), Diagnostic> {
        self.expect(TokenKind::LBracket)?;
        let mut depth = 1usize;
        while depth > 0 {
            match self.peek() {
                TokenKind::LBracket => {
                    depth += 1;
                    self.bump();
                }
                TokenKind::RBracket => {
                    depth -= 1;
                    self.bump();
                }
                TokenKind::Eof => return self.err("unterminated local type"),
                _ => self.bump(),
            }
        }
        Ok(())
    }
    fn type_text_until_body(&mut self, stop: &[TokenKind]) -> Result<String, Diagnostic> {
        let st = self.tokens[self.at].span.start;
        let (mut brackets, mut parens, mut angles) = (0usize, 0usize, 0usize);
        loop {
            let top = brackets == 0 && parens == 0 && angles == 0;
            if top && stop.iter().any(|k| same(k, self.peek())) {
                break;
            }
            match self.peek() {
                TokenKind::LBracket => brackets += 1,
                TokenKind::RBracket if brackets > 0 => brackets -= 1,
                TokenKind::LParen => parens += 1,
                TokenKind::RParen if parens > 0 => parens -= 1,
                TokenKind::Less => angles += 1,
                TokenKind::Greater if angles > 0 => angles -= 1,
                TokenKind::Eof => return self.err("unterminated local type"),
                _ => {}
            }
            self.bump();
        }
        let en = self.tokens[self.at].span.start;
        Ok(self.src[st..en].trim().into())
    }
    fn next_is_assign(&self) -> bool {
        self.tokens
            .get(self.at + 1)
            .is_some_and(|t| matches!(t.kind, TokenKind::Assign))
    }
    fn expr(&mut self, min_prec: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.prefix()?;
        loop {
            let Some((prec, op)) = binop(self.peek()) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.bump();
            let right = self.expr(prec + 1)?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }
    fn prefix(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = match self.peek().clone() {
            TokenKind::Minus => {
                self.bump();
                Expr::Unary {
                    op: UnaryOp::Neg,
                    value: Box::new(self.prefix()?),
                }
            }
            TokenKind::Bang => {
                self.bump();
                Expr::Unary {
                    op: UnaryOp::Not,
                    value: Box::new(self.prefix()?),
                }
            }
            TokenKind::Number(n) => {
                self.bump();
                Expr::Number(n)
            }
            TokenKind::String(s) => {
                self.bump();
                Expr::String(s)
            }
            TokenKind::Ident(n) => {
                self.bump();
                if n == "true" {
                    Expr::Bool(true)
                } else if n == "false" {
                    Expr::Bool(false)
                } else {
                    Expr::Path(vec![n])
                }
            }
            TokenKind::LParen => {
                self.bump();
                let e = self.expr(0)?;
                self.expect(TokenKind::RParen)?;
                e
            }
            TokenKind::LBracket => {
                self.bump();
                let mut xs = Vec::new();
                while !matches!(self.peek(), TokenKind::RBracket) {
                    xs.push(self.expr(0)?);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Expr::Array(xs)
            }
            _ => return self.err("expected expression"),
        };
        loop {
            match self.peek() {
                TokenKind::Dot => {
                    self.bump();
                    let n = self.ident()?;
                    match &mut e {
                        Expr::Path(parts) => parts.push(n),
                        _ => e = Expr::Path(vec![render_pathish(&e), n]),
                    }
                }
                TokenKind::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), TokenKind::RParen) {
                        args.push(self.expr(0)?);
                        if matches!(self.peek(), TokenKind::Comma) {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    e = Expr::Call {
                        callee: Box::new(e),
                        args,
                    };
                }
                _ => break,
            }
        }
        Ok(e)
    }
    fn ident(&mut self) -> Result<String, Diagnostic> {
        if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump();
            Ok(s)
        } else {
            self.err("expected identifier")
        }
    }
    fn expect(&mut self, k: TokenKind) -> Result<(), Diagnostic> {
        if same(&k, self.peek()) {
            self.bump();
            Ok(())
        } else {
            self.err(&format!("expected {k:?}"))
        }
    }
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.at].kind
    }
    fn bump(&mut self) {
        self.at += 1;
    }
    fn err<T>(&self, msg: &str) -> Result<T, Diagnostic> {
        Err(Diagnostic {
            message: msg.into(),
            span: self.tokens.get(self.at).map(|t| t.span).unwrap_or(Span {
                start: self.src.len(),
                end: self.src.len(),
            }),
        })
    }
}
fn render_pathish(e: &Expr) -> String {
    match e {
        Expr::Path(p) => p.join("."),
        _ => "value".into(),
    }
}
fn same(a: &TokenKind, b: &TokenKind) -> bool {
    core::mem::discriminant(a) == core::mem::discriminant(b)
}
fn binop(k: &TokenKind) -> Option<(u8, BinaryOp)> {
    Some(match k {
        TokenKind::OrOr => (1, BinaryOp::Or),
        TokenKind::AndAnd => (2, BinaryOp::And),
        TokenKind::Or => (3, BinaryOp::BitOr),
        TokenKind::Caret => (4, BinaryOp::BitXor),
        TokenKind::And => (5, BinaryOp::BitAnd),
        TokenKind::EqEq => (6, BinaryOp::Eq),
        TokenKind::BangEq => (6, BinaryOp::Ne),
        TokenKind::Less => (7, BinaryOp::Lt),
        TokenKind::LessEq => (7, BinaryOp::Le),
        TokenKind::Greater => (7, BinaryOp::Gt),
        TokenKind::GreaterEq => (7, BinaryOp::Ge),
        TokenKind::Plus => (8, BinaryOp::Add),
        TokenKind::Minus => (8, BinaryOp::Sub),
        TokenKind::Star => (9, BinaryOp::Mul),
        TokenKind::Slash => (9, BinaryOp::Div),
        TokenKind::Percent => (9, BinaryOp::Rem),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_game_body() {
        let body = r#"position += velocity; if (position.y <= 0 || position.y >= 232) { velocity.y = -velocity.y; }"#;
        let s = parse_body(body).unwrap();
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn accepts_conditions_with_or_without_parentheses() {
        let with_parens =
            parse_body("if (Input.held(Key.Left)) { x += 1; } while (x < 4) { x += 1; }").unwrap();
        let without_parens =
            parse_body("if Input.held(Key.Left) { x += 1; } while x < 4 { x += 1; }").unwrap();
        assert_eq!(with_parens.len(), 2);
        assert_eq!(without_parens.len(), 2);
    }
}

#[cfg(test)]
mod local_tests {
    use super::*;
    #[test]
    fn local_variables_are_lowered() {
        let body = parse_body(
            "var inferred = 1; i16 x = 2; Vec2fx pos = Vec2fx(1, 2); Handle[Bullet] h; [u8; 16] data; const u32 limit = 3; x += limit;"
        ).unwrap();
        assert!(
            matches!(&body[0], Stmt::Local { name, ty: None, mutable: true, .. } if name == "inferred")
        );
        assert!(
            matches!(&body[1], Stmt::Local { name, ty: Some(Type::I16), mutable: true, .. } if name == "x")
        );
        assert!(
            matches!(&body[2], Stmt::Local { name, ty: Some(Type::Vec2fx), mutable: true, .. } if name == "pos")
        );
        assert!(
            matches!(&body[3], Stmt::Local { name, ty: Some(Type::Handle(_)), mutable: true, value: None } if name == "h")
        );
        assert!(
            matches!(&body[4], Stmt::Local { name, ty: Some(Type::FixedArray(_, 16)), mutable: true, value: None } if name == "data")
        );
        assert!(
            matches!(&body[5], Stmt::Local { name, ty: Some(Type::U32), mutable: false, .. } if name == "limit")
        );
    }
}

#[cfg(test)]
mod bounded_type_tests {
    use super::*;
    #[test]
    fn parses_pool_and_handle_types() {
        assert_eq!(
            parse_type("Handle[Bullet]"),
            Type::Handle(Box::new(Type::Named("Bullet".into())))
        );
        assert_eq!(
            parse_type("Pool[Bullet; 32]"),
            Type::Pool(Box::new(Type::Named("Bullet".into())), 32)
        );
    }
}

#[cfg(test)]
mod assignment_regression_tests {
    use super::*;

    #[test]
    fn compound_assignments_are_not_parsed_as_binary_expressions() {
        let body = parse_body("x = 1; x += 2; x -= 3; x *= 4; x /= 5; foo(x + 1);")
            .expect("assignment statements should parse");

        assert!(matches!(
            body[0],
            Stmt::Assign {
                op: AssignOp::Set,
                ..
            }
        ));
        assert!(matches!(
            body[1],
            Stmt::Assign {
                op: AssignOp::Add,
                ..
            }
        ));
        assert!(matches!(
            body[2],
            Stmt::Assign {
                op: AssignOp::Sub,
                ..
            }
        ));
        assert!(matches!(
            body[3],
            Stmt::Assign {
                op: AssignOp::Mul,
                ..
            }
        ));
        assert!(matches!(
            body[4],
            Stmt::Assign {
                op: AssignOp::Div,
                ..
            }
        ));
        assert!(matches!(body[5], Stmt::Expr(_)));
    }
}

#[cfg(test)]
mod native_escape_tests {
    use super::*;
    #[test]
    fn lowers_native_rust_and_asm_statements() {
        let body = parse_body(
            r#"
            unsafe rust { let x: u32 = 1; core::hint::black_box(x); }
            unsafe asm[numworks] { "nop", options(nomem, nostack) }
        "#,
        )
        .unwrap();
        assert!(matches!(
            &body[0],
            Stmt::Native {
                language: NativeLanguage::Rust,
                target: None,
                ..
            }
        ));
        assert!(
            matches!(&body[1], Stmt::Native{language:NativeLanguage::Asm,target:Some(t),..} if t=="numworks")
        );
    }
    #[test]
    fn asm_without_target_is_rejected() {
        assert!(parse_body(r#"unsafe asm { "nop" }"#).is_err());
    }
}
