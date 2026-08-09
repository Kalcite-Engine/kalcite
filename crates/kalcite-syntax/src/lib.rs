use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    At,
    Ident(String),
    Number(String),
    String(String),
    Class,
    Struct,
    Fn,
    Var,
    Const,
    Signal,
    Use,
    Extends,
    Return,
    If,
    Else,
    While,
    For,
    In,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Arrow,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    Bang,
    BangEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    And,
    AndAnd,
    Or,
    OrOr,
    Percent,
    Caret,
    NativeBlock {
        language: String,
        target: Option<String>,
        body: String,
    },
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, Diagnostic> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        if let Some((kind, end)) = try_lex_native_block(src, i)? {
            out.push(Token {
                kind,
                span: Span { start: i, end },
            });
            i = end;
            continue;
        }
        match b[i] {
            c if c.is_ascii_whitespace() => i += 1,
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                i += 2;
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' => {
                out.push(tok(TokenKind::Slash, i, 1));
                i += 1
            }
            b'@' => {
                out.push(tok(TokenKind::At, i, 1));
                i += 1
            }
            b'{' => {
                out.push(tok(TokenKind::LBrace, i, 1));
                i += 1
            }
            b'}' => {
                out.push(tok(TokenKind::RBrace, i, 1));
                i += 1
            }
            b'(' => {
                out.push(tok(TokenKind::LParen, i, 1));
                i += 1
            }
            b')' => {
                out.push(tok(TokenKind::RParen, i, 1));
                i += 1
            }
            b'[' => {
                out.push(tok(TokenKind::LBracket, i, 1));
                i += 1
            }
            b']' => {
                out.push(tok(TokenKind::RBracket, i, 1));
                i += 1
            }
            b':' => {
                out.push(tok(TokenKind::Colon, i, 1));
                i += 1
            }
            b';' => {
                out.push(tok(TokenKind::Semicolon, i, 1));
                i += 1
            }
            b',' => {
                out.push(tok(TokenKind::Comma, i, 1));
                i += 1
            }
            b'.' => {
                out.push(tok(TokenKind::Dot, i, 1));
                i += 1
            }
            b'+' => {
                out.push(tok(TokenKind::Plus, i, 1));
                i += 1
            }
            b'*' => {
                out.push(tok(TokenKind::Star, i, 1));
                i += 1
            }
            b'-' if i + 1 < b.len() && b[i + 1] == b'>' => {
                out.push(tok(TokenKind::Arrow, i, 2));
                i += 2
            }
            b'-' => {
                out.push(tok(TokenKind::Minus, i, 1));
                i += 1
            }
            b'=' if i + 1 < b.len() && b[i + 1] == b'=' => {
                out.push(tok(TokenKind::EqEq, i, 2));
                i += 2
            }
            b'=' => {
                out.push(tok(TokenKind::Assign, i, 1));
                i += 1
            }
            b'!' if i + 1 < b.len() && b[i + 1] == b'=' => {
                out.push(tok(TokenKind::BangEq, i, 2));
                i += 2
            }
            b'!' => {
                out.push(tok(TokenKind::Bang, i, 1));
                i += 1
            }
            b'<' if i + 1 < b.len() && b[i + 1] == b'=' => {
                out.push(tok(TokenKind::LessEq, i, 2));
                i += 2
            }
            b'<' => {
                out.push(tok(TokenKind::Less, i, 1));
                i += 1
            }
            b'>' if i + 1 < b.len() && b[i + 1] == b'=' => {
                out.push(tok(TokenKind::GreaterEq, i, 2));
                i += 2
            }
            b'>' => {
                out.push(tok(TokenKind::Greater, i, 1));
                i += 1
            }
            b'&' if i + 1 < b.len() && b[i + 1] == b'&' => {
                out.push(tok(TokenKind::AndAnd, i, 2));
                i += 2
            }
            b'&' => {
                out.push(tok(TokenKind::And, i, 1));
                i += 1
            }
            b'|' if i + 1 < b.len() && b[i + 1] == b'|' => {
                out.push(tok(TokenKind::OrOr, i, 2));
                i += 2
            }
            b'|' => {
                out.push(tok(TokenKind::Or, i, 1));
                i += 1
            }
            b'%' => {
                out.push(tok(TokenKind::Percent, i, 1));
                i += 1
            }
            b'^' => {
                out.push(tok(TokenKind::Caret, i, 1));
                i += 1
            }
            b'"' => {
                let st = i;
                i += 1;
                let begin = i;
                while i < b.len() && b[i] != b'"' {
                    i += 1;
                }
                if i == b.len() {
                    return Err(Diagnostic {
                        message: "unterminated string".into(),
                        span: Span { start: st, end: i },
                    });
                }
                let s = &src[begin..i];
                i += 1;
                out.push(Token {
                    kind: TokenKind::String(s.into()),
                    span: Span { start: st, end: i },
                });
            }
            c if c.is_ascii_digit() => {
                let st = i;
                i += 1;
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                out.push(Token {
                    kind: TokenKind::Number(src[st..i].into()),
                    span: Span { start: st, end: i },
                })
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let st = i;
                i += 1;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let s = &src[st..i];
                let k = match s {
                    "class" => TokenKind::Class,
                    "struct" => TokenKind::Struct,
                    "fn" => TokenKind::Fn,
                    "var" => TokenKind::Var,
                    "const" => TokenKind::Const,
                    "signal" => TokenKind::Signal,
                    "use" => TokenKind::Use,
                    "extends" => TokenKind::Extends,
                    "return" => TokenKind::Return,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "while" => TokenKind::While,
                    "for" => TokenKind::For,
                    "in" => TokenKind::In,
                    _ => TokenKind::Ident(s.into()),
                };
                out.push(Token {
                    kind: k,
                    span: Span { start: st, end: i },
                })
            }
            _ => {
                return Err(Diagnostic {
                    message: format!("unexpected byte {:?}", b[i] as char),
                    span: Span {
                        start: i,
                        end: i + 1,
                    },
                });
            }
        }
    }
    out.push(Token {
        kind: TokenKind::Eof,
        span: Span { start: i, end: i },
    });
    Ok(out)
}

fn try_lex_native_block(src: &str, start: usize) -> Result<Option<(TokenKind, usize)>, Diagnostic> {
    let bytes = src.as_bytes();
    if !src[start..].starts_with("unsafe") {
        return Ok(None);
    }
    let boundary = start + "unsafe".len();
    if boundary < bytes.len()
        && (bytes[boundary].is_ascii_alphanumeric() || bytes[boundary] == b'_')
    {
        return Ok(None);
    }
    let mut i = boundary;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let (language, after_lang) = if src[i..].starts_with("rust") {
        ("rust", i + 4)
    } else if src[i..].starts_with("asm") {
        ("asm", i + 3)
    } else {
        return Ok(None);
    };
    i = after_lang;
    if i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        return Ok(None);
    }
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    let mut target = None;
    if i < bytes.len() && bytes[i] == b'[' {
        i += 1;
        let t0 = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'-'))
        {
            i += 1;
        }
        if i == t0 || i >= bytes.len() || bytes[i] != b']' {
            return Err(Diagnostic {
                message: "expected native target like [numworks]".into(),
                span: Span {
                    start,
                    end: i.min(bytes.len()),
                },
            });
        }
        target = Some(src[t0..i].to_string());
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
    }
    if i >= bytes.len() || bytes[i] != b'{' {
        return Ok(None);
    }
    let body_start = i + 1;
    let mut depth = 1usize;
    i += 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                let mut d = 1usize;
                while i + 1 < bytes.len() && d > 0 {
                    if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                        d += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        d -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            b'"' => {
                i = skip_quoted(bytes, i, b'"');
            }
            b'\'' => {
                i = skip_quoted(bytes, i, b'\'');
            }
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if depth != 0 {
        return Err(Diagnostic {
            message: format!("unterminated unsafe {language} block"),
            span: Span {
                start,
                end: bytes.len(),
            },
        });
    }
    let body_end = i - 1;
    Ok(Some((
        TokenKind::NativeBlock {
            language: language.into(),
            target,
            body: src[body_start..body_end].trim().into(),
        },
        i,
    )))
}

fn skip_quoted(bytes: &[u8], mut i: usize, quote: u8) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn tok(kind: TokenKind, start: usize, len: usize) -> Token {
    Token {
        kind,
        span: Span {
            start,
            end: start + len,
        },
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Use(UseDecl),
    Class(Class),
    Struct(Struct),
    Function(Function),
}
#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub path: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub base: Option<String>,
    pub members: Vec<Member>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub fields: Vec<Field>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    Field(Field),
    Function(Function),
    Signal(Signal),
    Class(Class),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub attrs: Vec<Attribute>,
    pub mutable: bool,
    pub name: String,
    pub ty: String,
    pub init: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub params: Vec<Field>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub params: Vec<Field>,
    pub ret: Option<String>,
    pub body: String,
}

pub fn parse(src: &str) -> Result<Module, Diagnostic> {
    Parser {
        src,
        tokens: lex(src)?,
        at: 0,
    }
    .module()
}
struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    at: usize,
}
impl<'a> Parser<'a> {
    fn module(&mut self) -> Result<Module, Diagnostic> {
        let mut items = Vec::new();
        while !matches!(self.peek(), TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Use) {
                items.push(Item::Use(self.use_decl()?));
                continue;
            }
            let a = self.attrs()?;
            items.push(match self.peek() {
                TokenKind::Class => Item::Class(self.class(a)?),
                TokenKind::Struct => Item::Struct(self.strukt(a)?),
                TokenKind::Fn => Item::Function(self.function(a)?),
                _ => return self.err("expected use, class, struct, or fn"),
            });
        }
        Ok(Module { items })
    }
    fn use_decl(&mut self) -> Result<UseDecl, Diagnostic> {
        self.expect(TokenKind::Use)?;
        let mut path = vec![self.ident()?];
        while matches!(self.peek(), TokenKind::Dot) {
            self.bump();
            path.push(self.ident()?);
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(UseDecl { path })
    }
    fn attrs(&mut self) -> Result<Vec<Attribute>, Diagnostic> {
        let mut v = Vec::new();
        while matches!(self.peek(), TokenKind::At) {
            self.bump();
            let n = self.ident()?;
            let mut args = Vec::new();
            if matches!(self.peek(), TokenKind::LParen) {
                self.bump();
                let st = self.tokens[self.at].span.start;
                let mut depth = 1;
                while depth > 0 {
                    match self.peek() {
                        TokenKind::LParen => depth += 1,
                        TokenKind::RParen => depth -= 1,
                        TokenKind::Eof => return self.err("unterminated attribute"),
                        _ => {}
                    }
                    self.bump();
                }
                let en = self.tokens[self.at - 1].span.start;
                let raw = self.src[st..en].trim();
                if !raw.is_empty() {
                    args = raw.split(',').map(|x| x.trim().to_string()).collect();
                }
            }
            v.push(Attribute { name: n, args });
        }
        Ok(v)
    }
    fn class(&mut self, attrs: Vec<Attribute>) -> Result<Class, Diagnostic> {
        self.bump();
        let name = self.ident()?;
        let base = if matches!(self.peek(), TokenKind::Extends) {
            self.bump();
            Some(self.ident()?)
        } else {
            None
        };
        self.expect(TokenKind::LBrace)?;
        let mut members = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            let a = self.attrs()?;
            members.push(match self.peek() {
                TokenKind::Var | TokenKind::Const => Member::Field(self.field(a)?),
                TokenKind::Fn => Member::Function(self.function(a)?),
                TokenKind::Signal => Member::Signal(self.signal(a)?),
                TokenKind::Class => Member::Class(self.class(a)?),
                _ => return self.err("expected field, function, signal, or nested class"),
            });
        }
        self.bump();
        Ok(Class {
            attrs,
            name,
            base,
            members,
        })
    }
    fn strukt(&mut self, attrs: Vec<Attribute>) -> Result<Struct, Diagnostic> {
        self.bump();
        let name = self.ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            let a = self.attrs()?;
            fields.push(self.field(a)?);
        }
        self.bump();
        Ok(Struct {
            attrs,
            name,
            fields,
        })
    }
    fn field(&mut self, attrs: Vec<Attribute>) -> Result<Field, Diagnostic> {
        let mutable = matches!(self.peek(), TokenKind::Var);
        self.bump();
        let name = self.ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.type_text()?;
        let init = if matches!(self.peek(), TokenKind::Assign) {
            self.bump();
            Some(self.until_semicolon()?)
        } else {
            self.expect(TokenKind::Semicolon)?;
            None
        };
        Ok(Field {
            attrs,
            mutable,
            name,
            ty,
            init,
        })
    }
    fn signal(&mut self, attrs: Vec<Attribute>) -> Result<Signal, Diagnostic> {
        self.expect(TokenKind::Signal)?;
        let name = self.ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen) {
            let n = self.ident()?;
            self.expect(TokenKind::Colon)?;
            let t = self.type_text_until(&[TokenKind::Comma, TokenKind::RParen])?;
            params.push(Field {
                attrs: Vec::new(),
                mutable: false,
                name: n,
                ty: t,
                init: None,
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.bump();
            }
        }
        self.bump();
        self.expect(TokenKind::Semicolon)?;
        Ok(Signal {
            attrs,
            name,
            params,
        })
    }
    fn function(&mut self, attrs: Vec<Attribute>) -> Result<Function, Diagnostic> {
        self.expect(TokenKind::Fn)?;
        let name = self.ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen) {
            let n = self.ident()?;
            self.expect(TokenKind::Colon)?;
            let t = self.type_text_until(&[TokenKind::Comma, TokenKind::RParen])?;
            params.push(Field {
                attrs: Vec::new(),
                mutable: false,
                name: n,
                ty: t,
                init: None,
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.bump();
            }
        }
        self.bump();
        let ret = if matches!(self.peek(), TokenKind::Arrow) {
            self.bump();
            Some(self.type_text_until(&[TokenKind::LBrace])?)
        } else {
            None
        };
        let body = self.block_text()?;
        Ok(Function {
            attrs,
            name,
            params,
            ret,
            body,
        })
    }
    fn block_text(&mut self) -> Result<String, Diagnostic> {
        self.expect(TokenKind::LBrace)?;
        let st = self.tokens[self.at].span.start;
        let mut d = 1;
        while d > 0 {
            match self.peek() {
                TokenKind::LBrace => d += 1,
                TokenKind::RBrace => d -= 1,
                TokenKind::Eof => return self.err("unterminated block"),
                _ => {}
            }
            self.bump();
        }
        let en = self.tokens[self.at - 1].span.start;
        Ok(self.src[st..en].trim().into())
    }
    fn type_text(&mut self) -> Result<String, Diagnostic> {
        self.type_text_until(&[TokenKind::Assign, TokenKind::Semicolon])
    }
    fn type_text_until(&mut self, stop: &[TokenKind]) -> Result<String, Diagnostic> {
        let st = self.tokens[self.at].span.start;
        let (mut brackets, mut parens, mut angles) = (0usize, 0usize, 0usize);
        loop {
            let at_top = brackets == 0 && parens == 0 && angles == 0;
            if at_top && stop.iter().any(|k| same(k, self.peek())) {
                break;
            }
            match self.peek() {
                TokenKind::LBracket => brackets += 1,
                TokenKind::RBracket if brackets > 0 => brackets -= 1,
                TokenKind::LParen => parens += 1,
                TokenKind::RParen if parens > 0 => parens -= 1,
                TokenKind::Less => angles += 1,
                TokenKind::Greater if angles > 0 => angles -= 1,
                TokenKind::Eof => return self.err("unterminated type"),
                _ => {}
            }
            self.bump();
        }
        let en = self.tokens[self.at].span.start;
        Ok(self.src[st..en].trim().into())
    }
    fn until_semicolon(&mut self) -> Result<String, Diagnostic> {
        let st = self.tokens[self.at].span.start;
        while !matches!(self.peek(), TokenKind::Semicolon | TokenKind::Eof) {
            self.bump();
        }
        let en = self.tokens[self.at].span.start;
        self.expect(TokenKind::Semicolon)?;
        Ok(self.src[st..en].trim().into())
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
    fn err<T>(&self, m: &str) -> Result<T, Diagnostic> {
        Err(Diagnostic {
            message: m.into(),
            span: self.tokens[self.at].span,
        })
    }
}
fn same(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_class() {
        let m=parse("@pool(4) class Ball extends Entity { var x: i16 = 0; fn tick(dt: u16) -> void { x += 1; } }").unwrap();
        assert_eq!(m.items.len(), 1);
    }
}

#[cfg(test)]
mod lexer_operator_tests {
    use super::{TokenKind, lex};

    #[test]
    fn lexes_logical_and_comparison_operators_in_function_bodies() {
        let source = r#"
            class Test {
                fn update() -> void {
                    if (x <= 0 || x >= 10 && flags & 1 == 1) {
                        x = (x + 1) % 10;
                    }
                }
            }
        "#;

        let tokens = lex(source).expect("operators should lex");
        assert!(tokens.iter().any(|token| token.kind == TokenKind::LessEq));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::GreaterEq)
        );
        assert!(tokens.iter().any(|token| token.kind == TokenKind::OrOr));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::AndAnd));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::And));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Percent));
    }
}

#[cfg(test)]
mod parser_regression_tests {
    use super::{Item, Member, parse};

    #[test]
    fn parses_fixed_array_and_nested_classes() {
        let source = r#"
            @scene
            class Pong extends Game {
                const SCREEN: [u16; 2] = [320, 240];

                @pool(1)
                class Ball extends Entity {
                    var position: Vec2fx;
                }
            }
        "#;

        let module = parse(source).expect("fixed arrays and nested classes should parse");
        let Item::Class(pong) = &module.items[0] else {
            panic!("expected Pong class")
        };
        assert_eq!(pong.members.len(), 2);
        assert!(matches!(pong.members[1], Member::Class(_)));
    }
}

#[cfg(test)]
mod use_tests {
    use super::*;
    #[test]
    fn parses_use_declaration() {
        let m = parse("use std.msgpack; @scene class G {}").unwrap();
        assert!(matches!(&m.items[0],Item::Use(u) if u.path==vec!["std","msgpack"]));
    }
}

#[cfg(test)]
mod native_block_tests {
    use super::*;
    #[test]
    fn lexes_native_rust_without_lexing_inner_rust_tokens() {
        let src = r#"class G { fn tick() -> void { unsafe rust[numworks] { let p: *mut u16 = 0x2000_0000 as *mut u16; core::ptr::write_volatile(p, 1); } } }"#;
        let tokens = lex(src).unwrap();
        assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::NativeBlock{language,target,body} if language=="rust" && target.as_deref()==Some("numworks") && body.contains("write_volatile"))));
    }
    #[test]
    fn lexes_native_asm_as_raw_asm_macro_body() {
        let src = r#"class G { fn tick() -> void { unsafe asm[numworks] { "nop", options(nomem, nostack) } } }"#;
        let tokens = lex(src).unwrap();
        assert!(
            tokens
                .iter()
                .any(|t| matches!(&t.kind, TokenKind::NativeBlock{language,..} if language=="asm"))
        );
    }
    #[test]
    fn native_asm_allows_register_placeholders_and_braces_in_strings() {
        let src = r#"class G { fn tick() -> void { unsafe asm[numworks] { "mov {0}, {1}", out(reg) dst, in(reg) src, options(nomem, nostack) } } }"#;
        let tokens = lex(src).unwrap();
        assert!(tokens.iter().any(|t| matches!(
            &t.kind,
            TokenKind::NativeBlock { language, target, body }
                if language == "asm"
                    && target.as_deref() == Some("numworks")
                    && body.contains("mov {0}, {1}")
        )));
    }
}
