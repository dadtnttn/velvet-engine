//! JS-like motion script inside `.vcss` (`@script { … }`).
//!
//! `.vcss` = **CSS** (look + `@keyframes`) + **JS-lite** (orchestration):
//! `let`, `fn`, `for`, `if`, `play()`, `animate()`, `wait()`, `on()`.

use indexmap::IndexMap;
use thiserror::Error;

use crate::animation::{plan_from_spec, AnimationSpec, TimelinePlan};
use crate::parse::Stylesheet;
use crate::runtime::StyleRuntime;

/// Script parse/eval error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// Parse failure.
    #[error("vcss script parse at {line}: {msg}")]
    Parse {
        /// 1-based line within the script block (approx).
        line: usize,
        /// Detail.
        msg: String,
    },
    /// Runtime failure.
    #[error("vcss script runtime: {0}")]
    Runtime(String),
}

/// Dynamic value in the JS-lite runtime.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum JsValue {
    /// null / undefined
    #[default]
    Null,
    /// number
    Number(f32),
    /// string
    String(String),
    /// bool
    Bool(bool),
    /// array
    Array(Vec<JsValue>),
    /// object `{ key: value }`
    Object(IndexMap<String, JsValue>),
}

impl JsValue {
    /// Coerce to f32 if possible.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Coerce to bool (JS-ish truthiness).
    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::String(s) => !s.is_empty(),
            Self::Bool(b) => *b,
            Self::Array(a) => !a.is_empty(),
            Self::Object(o) => !o.is_empty(),
        }
    }

    /// Display / string coerce.
    pub fn as_string(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e9 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Self::String(s) => s.clone(),
            Self::Bool(b) => b.to_string(),
            Self::Array(a) => a
                .iter()
                .map(|v| v.as_string())
                .collect::<Vec<_>>()
                .join(","),
            Self::Object(_) => "[object]".into(),
        }
    }

    /// From Rust f32.
    pub fn num(n: f32) -> Self {
        Self::Number(n)
    }

    /// From string.
    pub fn str(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }
}

/// One side-effect / scheduled action produced by running a script fn.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleAction {
    /// Play a named `@keyframes` animation on a target.
    Play {
        /// Keyframe name.
        animation: String,
        /// Target id (no `#`).
        target: Option<String>,
        /// Delay seconds.
        delay: f32,
        /// Override duration (None = use CSS / default).
        duration: Option<f32>,
        /// Override easing.
        ease: Option<String>,
    },
    /// Imperative from→to tween (no named keyframes needed).
    Animate {
        /// Target id or selector string.
        target: String,
        /// (channel, from, to)
        channels: Vec<(String, f32, f32)>,
        /// Duration seconds.
        duration: f32,
        /// Delay seconds.
        delay: f32,
        /// Easing keyword.
        ease: String,
    },
    /// Wait (sequencing hint for hosts).
    Wait {
        /// Seconds.
        secs: f32,
    },
    /// Emit a custom event name (host may listen).
    Emit {
        /// Event name.
        event: String,
        /// Optional payload string.
        payload: Option<String>,
    },
    /// Set a numeric style channel on a target (`set("#id", "opacity", 0.5)`).
    Set {
        /// Target id (no `#`).
        target: String,
        /// Channel / property name.
        prop: String,
        /// Value.
        value: f32,
    },
}

/// Registered event handler reference.
#[derive(Debug, Clone, PartialEq)]
pub struct EventHandler {
    /// Event name (`menu.open`, `hand.deal`, …).
    pub event: String,
    /// Function name to call.
    pub fn_name: String,
}

/// Compiled script module from one or more `@script` blocks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptModule {
    /// Top-level `let` initializers (name → expression source, evaluated on run).
    pub globals: IndexMap<String, Expr>,
    /// Named functions.
    pub functions: IndexMap<String, Function>,
    /// `on(event, fnName)` registrations.
    pub handlers: Vec<EventHandler>,
}

impl ScriptModule {
    /// Merge another module (later functions/handlers win / append).
    pub fn extend(&mut self, other: ScriptModule) {
        for (k, v) in other.globals {
            self.globals.insert(k, v);
        }
        for (k, v) in other.functions {
            self.functions.insert(k, v);
        }
        self.handlers.extend(other.handlers);
    }

    /// Function names.
    pub fn function_names(&self) -> impl Iterator<Item = &str> {
        self.functions.keys().map(|s| s.as_str())
    }
}

/// Function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Parameter names.
    pub params: Vec<String>,
    /// Body statements.
    pub body: Vec<Stmt>,
}

/// Statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let x = expr;`
    Let {
        /// Name.
        name: String,
        /// Init.
        value: Expr,
    },
    /// `x = expr;`
    Assign {
        /// Name.
        name: String,
        /// Value.
        value: Expr,
    },
    /// Expression statement (call side-effects).
    Expr(Expr),
    /// `for (let i = a; i < b; i = i + 1) { … }`  (classic 3-part)
    For {
        /// Loop variable.
        var: String,
        /// Init expression.
        init: Expr,
        /// Continue while truthy.
        cond: Expr,
        /// End-of-iteration update (assign expr RHS evaluated into var if Assign form).
        update: Box<Stmt>,
        /// Body.
        body: Vec<Stmt>,
    },
    /// `if (cond) { … } else { … }`
    If {
        /// Condition.
        cond: Expr,
        /// Then branch.
        then_body: Vec<Stmt>,
        /// Else branch.
        else_body: Vec<Stmt>,
    },
    /// `return expr;`
    Return(Option<Expr>),
}

/// Expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// null
    Null,
    /// number literal
    Number(f32),
    /// string literal
    String(String),
    /// bool
    Bool(bool),
    /// identifier
    Ident(String),
    /// array `[a, b]`
    Array(Vec<Expr>),
    /// object `{ a: 1, b: "x" }`
    Object(Vec<(String, Expr)>),
    /// unary `-` / `!`
    Unary {
        /// op
        op: UnaryOp,
        /// expr
        expr: Box<Expr>,
    },
    /// binary
    Binary {
        /// left
        left: Box<Expr>,
        /// op
        op: BinOp,
        /// right
        right: Box<Expr>,
    },
    /// `fn(a, b)` or method-less call
    Call {
        /// callee name (only idents for now)
        name: String,
        /// args
        args: Vec<Expr>,
    },
    /// `obj.key` or `arr[i]`
    Index {
        /// base
        base: Box<Expr>,
        /// index (string key or number)
        index: Box<Expr>,
        /// true if `.ident` form (index is Ident string)
        dot: bool,
    },
}

/// Unary ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// -
    Neg,
    /// !
    Not,
}

/// Binary ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
    /// %
    Mod,
    /// <
    Lt,
    /// <=
    Le,
    /// >
    Gt,
    /// >=
    Ge,
    /// ==
    Eq,
    /// !=
    Ne,
    /// &&
    And,
    /// ||
    Or,
}

// ─── Lexer ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Number(f32),
    String(String),
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semi,
    Dot,
    // ops
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    // keywords as idents handled in parser
    Eof,
}

struct Lexer<'a> {
    src: &'a [u8],
    i: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            i: 0,
            line: 1,
        }
    }

    fn peek(&self) -> u8 {
        if self.i < self.src.len() {
            self.src[self.i]
        } else {
            0
        }
    }

    fn bump(&mut self) -> u8 {
        let c = self.peek();
        if c == b'\n' {
            self.line += 1;
        }
        if self.i < self.src.len() {
            self.i += 1;
        }
        c
    }

    fn skip_ws_comments(&mut self) {
        loop {
            while self.i < self.src.len() && self.src[self.i].is_ascii_whitespace() {
                self.bump();
            }
            if self.peek() == b'/' && self.i + 1 < self.src.len() && self.src[self.i + 1] == b'/' {
                while self.i < self.src.len() && self.peek() != b'\n' {
                    self.bump();
                }
                continue;
            }
            if self.peek() == b'/' && self.i + 1 < self.src.len() && self.src[self.i + 1] == b'*' {
                self.bump();
                self.bump();
                while self.i + 1 < self.src.len()
                    && !(self.peek() == b'*' && self.src[self.i + 1] == b'/')
                {
                    self.bump();
                }
                self.bump();
                self.bump();
                continue;
            }
            break;
        }
    }

    fn next_tok(&mut self) -> Result<(Tok, usize), ScriptError> {
        self.skip_ws_comments();
        let line = self.line;
        if self.i >= self.src.len() {
            return Ok((Tok::Eof, line));
        }
        let c = self.peek();
        // string
        if c == b'"' || c == b'\'' {
            let quote = self.bump();
            let mut s = String::new();
            while self.i < self.src.len() && self.peek() != quote {
                let ch = self.bump();
                if ch == b'\\' && self.i < self.src.len() {
                    let e = self.bump();
                    s.push(match e {
                        b'n' => '\n',
                        b't' => '\t',
                        b'\\' => '\\',
                        b'"' => '"',
                        b'\'' => '\'',
                        other => other as char,
                    });
                } else {
                    s.push(ch as char);
                }
            }
            if self.peek() != quote {
                return Err(ScriptError::Parse {
                    line,
                    msg: "unterminated string".into(),
                });
            }
            self.bump();
            return Ok((Tok::String(s), line));
        }
        // number
        if c.is_ascii_digit()
            || (c == b'.' && self.i + 1 < self.src.len() && self.src[self.i + 1].is_ascii_digit())
        {
            let start = self.i;
            while self.peek().is_ascii_digit() {
                self.bump();
            }
            if self.peek() == b'.' {
                self.bump();
                while self.peek().is_ascii_digit() {
                    self.bump();
                }
            }
            // optional trailing `s` for seconds (JS-ish CSS hybrid): 0.35s
            if self.peek() == b's'
                && (self.i + 1 >= self.src.len()
                    || !self.src[self.i + 1].is_ascii_alphanumeric())
            {
                // swallow unit, value already pure number
                let raw = std::str::from_utf8(&self.src[start..self.i]).unwrap_or("0");
                let n: f32 = raw.parse().unwrap_or(0.0);
                self.bump();
                return Ok((Tok::Number(n), line));
            }
            let raw = std::str::from_utf8(&self.src[start..self.i]).unwrap_or("0");
            let n: f32 = raw.parse().map_err(|_| ScriptError::Parse {
                line,
                msg: format!("bad number `{raw}`"),
            })?;
            return Ok((Tok::Number(n), line));
        }
        // ident / keyword
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = self.i;
            self.bump();
            while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' {
                self.bump();
            }
            let id = std::str::from_utf8(&self.src[start..self.i])
                .unwrap_or("")
                .to_string();
            return Ok((Tok::Ident(id), line));
        }
        // two-char ops
        let c = self.bump();
        let n = self.peek();
        let tok = match (c, n) {
            (b'=', b'=') => {
                self.bump();
                Tok::EqEq
            }
            (b'!', b'=') => {
                self.bump();
                Tok::Ne
            }
            (b'<', b'=') => {
                self.bump();
                Tok::Le
            }
            (b'>', b'=') => {
                self.bump();
                Tok::Ge
            }
            (b'&', b'&') => {
                self.bump();
                Tok::AndAnd
            }
            (b'|', b'|') => {
                self.bump();
                Tok::OrOr
            }
            (b'(', _) => Tok::LParen,
            (b')', _) => Tok::RParen,
            (b'{', _) => Tok::LBrace,
            (b'}', _) => Tok::RBrace,
            (b'[', _) => Tok::LBracket,
            (b']', _) => Tok::RBracket,
            (b',', _) => Tok::Comma,
            (b':', _) => Tok::Colon,
            (b';', _) => Tok::Semi,
            (b'.', _) => Tok::Dot,
            (b'+', _) => Tok::Plus,
            (b'-', _) => Tok::Minus,
            (b'*', _) => Tok::Star,
            (b'/', _) => Tok::Slash,
            (b'%', _) => Tok::Percent,
            (b'=', _) => Tok::Assign,
            (b'<', _) => Tok::Lt,
            (b'>', _) => Tok::Gt,
            (b'!', _) => Tok::Bang,
            (other, _) => {
                return Err(ScriptError::Parse {
                    line,
                    msg: format!("unexpected character `{}`", other as char),
                });
            }
        };
        Ok((tok, line))
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<(Tok, usize)>,
    i: usize,
}

impl Parser {
    fn parse_module(src: &str) -> Result<ScriptModule, ScriptError> {
        let mut lex = Lexer::new(src);
        let mut tokens = Vec::new();
        loop {
            let (t, line) = lex.next_tok()?;
            let done = t == Tok::Eof;
            tokens.push((t, line));
            if done {
                break;
            }
        }
        let mut p = Parser { tokens, i: 0 };
        p.parse_top_level()
    }

    fn peek(&self) -> &Tok {
        self.tokens
            .get(self.i)
            .map(|(t, _)| t)
            .unwrap_or(&Tok::Eof)
    }

    fn line(&self) -> usize {
        self.tokens.get(self.i).map(|(_, l)| *l).unwrap_or(1)
    }

    fn bump(&mut self) -> Tok {
        let t = self.tokens.get(self.i).map(|(t, _)| t.clone()).unwrap_or(Tok::Eof);
        if self.i < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    fn expect(&mut self, want: &Tok) -> Result<(), ScriptError> {
        let got = self.bump();
        if std::mem::discriminant(&got) != std::mem::discriminant(want)
            && !(matches!(want, Tok::Ident(_)) && matches!(got, Tok::Ident(_)))
        {
            // exact match for simple tokens
            let ok = match (want, &got) {
                (Tok::LParen, Tok::LParen)
                | (Tok::RParen, Tok::RParen)
                | (Tok::LBrace, Tok::LBrace)
                | (Tok::RBrace, Tok::RBrace)
                | (Tok::LBracket, Tok::LBracket)
                | (Tok::RBracket, Tok::RBracket)
                | (Tok::Comma, Tok::Comma)
                | (Tok::Colon, Tok::Colon)
                | (Tok::Semi, Tok::Semi)
                | (Tok::Assign, Tok::Assign) => true,
                _ => false,
            };
            if !ok {
                return Err(ScriptError::Parse {
                    line: self.line(),
                    msg: format!("expected {want:?}, got {got:?}"),
                });
            }
        }
        Ok(())
    }

    fn eat_semi(&mut self) {
        if matches!(self.peek(), Tok::Semi) {
            self.bump();
        }
    }

    fn parse_top_level(&mut self) -> Result<ScriptModule, ScriptError> {
        let mut module = ScriptModule::default();
        while !matches!(self.peek(), Tok::Eof) {
            match self.peek() {
                Tok::Ident(id) if id == "let" => {
                    self.bump();
                    let name = self.expect_ident()?;
                    self.expect(&Tok::Assign)?;
                    let value = self.parse_expr()?;
                    self.eat_semi();
                    module.globals.insert(name, value);
                }
                Tok::Ident(id) if id == "fn" || id == "function" => {
                    self.bump();
                    let name = self.expect_ident()?;
                    self.expect(&Tok::LParen)?;
                    let mut params = Vec::new();
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            params.push(self.expect_ident()?);
                            if matches!(self.peek(), Tok::Comma) {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    let body = self.parse_block()?;
                    module.functions.insert(name, Function { params, body });
                }
                Tok::Ident(id) if id == "on" => {
                    // on("event", handlerName) or on("event", fn name() {})
                    self.bump();
                    self.expect(&Tok::LParen)?;
                    let event = match self.bump() {
                        Tok::String(s) => s,
                        Tok::Ident(s) => s,
                        other => {
                            return Err(ScriptError::Parse {
                                line: self.line(),
                                msg: format!("on() event must be string, got {other:?}"),
                            });
                        }
                    };
                    self.expect(&Tok::Comma)?;
                    let fn_name = match self.peek() {
                        Tok::Ident(s) if s != "fn" && s != "function" => {
                            let n = self.expect_ident()?;
                            n
                        }
                        Tok::Ident(s) if s == "fn" || s == "function" => {
                            // inline fn — hoist as __on_{event}
                            self.bump();
                            let inline = format!(
                                "__on_{}",
                                event.replace('.', "_").replace('-', "_")
                            );
                            self.expect(&Tok::LParen)?;
                            let mut params = Vec::new();
                            if !matches!(self.peek(), Tok::RParen) {
                                loop {
                                    params.push(self.expect_ident()?);
                                    if matches!(self.peek(), Tok::Comma) {
                                        self.bump();
                                        continue;
                                    }
                                    break;
                                }
                            }
                            self.expect(&Tok::RParen)?;
                            let body = self.parse_block()?;
                            module
                                .functions
                                .insert(inline.clone(), Function { params, body });
                            inline
                        }
                        other => {
                            return Err(ScriptError::Parse {
                                line: self.line(),
                                msg: format!("on() handler must be fn name, got {other:?}"),
                            });
                        }
                    };
                    self.expect(&Tok::RParen)?;
                    self.eat_semi();
                    module.handlers.push(EventHandler { event, fn_name });
                }
                Tok::Semi => {
                    self.bump();
                }
                other => {
                    return Err(ScriptError::Parse {
                        line: self.line(),
                        msg: format!("expected let/fn/on at top level, got {other:?}"),
                    });
                }
            }
        }
        Ok(module)
    }

    fn expect_ident(&mut self) -> Result<String, ScriptError> {
        match self.bump() {
            Tok::Ident(s) => Ok(s),
            other => Err(ScriptError::Parse {
                line: self.line(),
                msg: format!("expected identifier, got {other:?}"),
            }),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ScriptError> {
        self.expect(&Tok::LBrace)?;
        let mut body = Vec::new();
        while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
            body.push(self.parse_stmt()?);
        }
        self.expect(&Tok::RBrace)?;
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ScriptError> {
        match self.peek() {
            Tok::Ident(id) if id == "let" => {
                self.bump();
                let name = self.expect_ident()?;
                self.expect(&Tok::Assign)?;
                let value = self.parse_expr()?;
                self.eat_semi();
                Ok(Stmt::Let { name, value })
            }
            Tok::Ident(id) if id == "for" => self.parse_for(),
            Tok::Ident(id) if id == "if" => self.parse_if(),
            Tok::Ident(id) if id == "return" => {
                self.bump();
                let e = if matches!(self.peek(), Tok::Semi | Tok::RBrace) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.eat_semi();
                Ok(Stmt::Return(e))
            }
            Tok::Ident(_) => {
                // assign or expr
                let save = self.i;
                if let Tok::Ident(name) = self.bump() {
                    if matches!(self.peek(), Tok::Assign) {
                        self.bump();
                        let value = self.parse_expr()?;
                        self.eat_semi();
                        return Ok(Stmt::Assign { name, value });
                    }
                }
                self.i = save;
                let e = self.parse_expr()?;
                self.eat_semi();
                Ok(Stmt::Expr(e))
            }
            _ => {
                let e = self.parse_expr()?;
                self.eat_semi();
                Ok(Stmt::Expr(e))
            }
        }
    }

    fn parse_for(&mut self) -> Result<Stmt, ScriptError> {
        self.bump(); // for
        self.expect(&Tok::LParen)?;
        // for (let i = 0; cond; update)
        let var;
        let init;
        if matches!(self.peek(), Tok::Ident(id) if id == "let") {
            self.bump();
            var = self.expect_ident()?;
            self.expect(&Tok::Assign)?;
            init = self.parse_expr()?;
        } else {
            var = self.expect_ident()?;
            self.expect(&Tok::Assign)?;
            init = self.parse_expr()?;
        }
        self.expect(&Tok::Semi)?;
        let cond = self.parse_expr()?;
        self.expect(&Tok::Semi)?;
        // update: i = i + 1  or  i++
        let update = {
            let name = self.expect_ident()?;
            if matches!(self.peek(), Tok::Plus) {
                // i++
                self.bump();
                if matches!(self.peek(), Tok::Plus) {
                    self.bump();
                }
                Stmt::Assign {
                    name: name.clone(),
                    value: Expr::Binary {
                        left: Box::new(Expr::Ident(name)),
                        op: BinOp::Add,
                        right: Box::new(Expr::Number(1.0)),
                    },
                }
            } else {
                self.expect(&Tok::Assign)?;
                let value = self.parse_expr()?;
                Stmt::Assign { name, value }
            }
        };
        self.expect(&Tok::RParen)?;
        let body = self.parse_block()?;
        Ok(Stmt::For {
            var,
            init,
            cond,
            update: Box::new(update),
            body,
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, ScriptError> {
        self.bump(); // if
        self.expect(&Tok::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&Tok::RParen)?;
        let then_body = self.parse_block()?;
        let else_body = if matches!(self.peek(), Tok::Ident(id) if id == "else") {
            self.bump();
            if matches!(self.peek(), Tok::Ident(id) if id == "if") {
                vec![self.parse_if()?]
            } else {
                self.parse_block()?
            }
        } else {
            Vec::new()
        };
        Ok(Stmt::If {
            cond,
            then_body,
            else_body,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, ScriptError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ScriptError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Tok::OrOr) {
            self.bump();
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ScriptError> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek(), Tok::AndAnd) {
            self.bump();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ScriptError> {
        let mut left = self.parse_cmp()?;
        loop {
            let op = match self.peek() {
                Tok::EqEq => BinOp::Eq,
                Tok::Ne => BinOp::Ne,
                _ => break,
            };
            self.bump();
            let right = self.parse_cmp()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ScriptError> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Tok::Lt => BinOp::Lt,
                Tok::Le => BinOp::Le,
                Tok::Gt => BinOp::Gt,
                Tok::Ge => BinOp::Ge,
                _ => break,
            };
            self.bump();
            let right = self.parse_add()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, ScriptError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_mul()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, ScriptError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ScriptError> {
        match self.peek() {
            Tok::Minus => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Tok::Bang => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Tok::LParen => {
                    // call — only if expr is Ident
                    let name = match &expr {
                        Expr::Ident(n) => n.clone(),
                        _ => {
                            return Err(ScriptError::Parse {
                                line: self.line(),
                                msg: "can only call identifiers".into(),
                            });
                        }
                    };
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if matches!(self.peek(), Tok::Comma) {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    expr = Expr::Call { name, args };
                }
                Tok::Dot => {
                    self.bump();
                    let key = self.expect_ident()?;
                    expr = Expr::Index {
                        base: Box::new(expr),
                        index: Box::new(Expr::String(key)),
                        dot: true,
                    };
                }
                Tok::LBracket => {
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(&Tok::RBracket)?;
                    expr = Expr::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                        dot: false,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ScriptError> {
        match self.bump() {
            Tok::Number(n) => Ok(Expr::Number(n)),
            Tok::String(s) => Ok(Expr::String(s)),
            Tok::Ident(id) => match id.as_str() {
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "null" | "undefined" => Ok(Expr::Null),
                _ => Ok(Expr::Ident(id)),
            },
            Tok::LParen => {
                let e = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if !matches!(self.peek(), Tok::RBracket) {
                    loop {
                        items.push(self.parse_expr()?);
                        if matches!(self.peek(), Tok::Comma) {
                            self.bump();
                            if matches!(self.peek(), Tok::RBracket) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&Tok::RBracket)?;
                Ok(Expr::Array(items))
            }
            Tok::LBrace => {
                // object literal
                let mut pairs = Vec::new();
                if !matches!(self.peek(), Tok::RBrace) {
                    loop {
                        let key = match self.bump() {
                            Tok::Ident(s) | Tok::String(s) => s,
                            other => {
                                return Err(ScriptError::Parse {
                                    line: self.line(),
                                    msg: format!("object key expected, got {other:?}"),
                                });
                            }
                        };
                        self.expect(&Tok::Colon)?;
                        let val = self.parse_expr()?;
                        pairs.push((key, val));
                        if matches!(self.peek(), Tok::Comma) {
                            self.bump();
                            if matches!(self.peek(), Tok::RBrace) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&Tok::RBrace)?;
                Ok(Expr::Object(pairs))
            }
            other => Err(ScriptError::Parse {
                line: self.line(),
                msg: format!("unexpected token in expression: {other:?}"),
            }),
        }
    }
}

/// Parse a `@script` body into a [`ScriptModule`].
pub fn parse_script(src: &str) -> Result<ScriptModule, ScriptError> {
    Parser::parse_module(src)
}

// ─── Evaluator ───────────────────────────────────────────────────────────────

/// Result of running a script function.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptRun {
    /// Actions to apply (play / animate / wait / emit).
    pub actions: Vec<StyleAction>,
    /// Timeline plans resolved against the stylesheet (for Play/Animate).
    pub timelines: Vec<TimelinePlan>,
    /// Return value if any.
    pub value: JsValue,
}

struct Vm<'a> {
    module: &'a ScriptModule,
    env: IndexMap<String, JsValue>,
    actions: Vec<StyleAction>,
    runtime: Option<&'a mut StyleRuntime>,
}

impl<'a> Vm<'a> {
    fn run_fn(&mut self, name: &str, args: &[JsValue]) -> Result<JsValue, ScriptError> {
        let func = self
            .module
            .functions
            .get(name)
            .ok_or_else(|| ScriptError::Runtime(format!("unknown function `{name}`")))?
            .clone();
        // bind params
        let mut frame = self.env.clone();
        for (i, p) in func.params.iter().enumerate() {
            frame.insert(
                p.clone(),
                args.get(i).cloned().unwrap_or(JsValue::Null),
            );
        }
        let prev = std::mem::replace(&mut self.env, frame);
        let result = self.exec_block(&func.body)?;
        self.env = prev;
        Ok(result.unwrap_or(JsValue::Null))
    }

    fn exec_block(&mut self, body: &[Stmt]) -> Result<Option<JsValue>, ScriptError> {
        for s in body {
            if let Some(v) = self.exec_stmt(s)? {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    fn exec_stmt(&mut self, s: &Stmt) -> Result<Option<JsValue>, ScriptError> {
        match s {
            Stmt::Let { name, value } => {
                let v = self.eval(value)?;
                self.env.insert(name.clone(), v);
                Ok(None)
            }
            Stmt::Assign { name, value } => {
                let v = self.eval(value)?;
                self.env.insert(name.clone(), v);
                Ok(None)
            }
            Stmt::Expr(e) => {
                self.eval(e)?;
                Ok(None)
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(x) => self.eval(x)?,
                    None => JsValue::Null,
                };
                Ok(Some(v))
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                if self.eval(cond)?.truthy() {
                    self.exec_block(then_body)
                } else {
                    self.exec_block(else_body)
                }
            }
            Stmt::For {
                var,
                init,
                cond,
                update,
                body,
            } => {
                let v = self.eval(init)?;
                self.env.insert(var.clone(), v);
                let mut guard = 0u32;
                while self.eval(cond)?.truthy() {
                    guard += 1;
                    if guard > 100_000 {
                        return Err(ScriptError::Runtime(
                            "for-loop exceeded 100000 iterations".into(),
                        ));
                    }
                    if let Some(ret) = self.exec_block(body)? {
                        return Ok(Some(ret));
                    }
                    self.exec_stmt(update)?;
                }
                Ok(None)
            }
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<JsValue, ScriptError> {
        match e {
            Expr::Null => Ok(JsValue::Null),
            Expr::Number(n) => Ok(JsValue::Number(*n)),
            Expr::String(s) => Ok(JsValue::String(s.clone())),
            Expr::Bool(b) => Ok(JsValue::Bool(*b)),
            Expr::Ident(name) => Ok(self
                .env
                .get(name)
                .cloned()
                .unwrap_or(JsValue::Null)),
            Expr::Array(items) => {
                let mut out = Vec::new();
                for it in items {
                    out.push(self.eval(it)?);
                }
                Ok(JsValue::Array(out))
            }
            Expr::Object(pairs) => {
                let mut map = IndexMap::new();
                for (k, v) in pairs {
                    map.insert(k.clone(), self.eval(v)?);
                }
                Ok(JsValue::Object(map))
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr)?;
                Ok(match op {
                    UnaryOp::Neg => JsValue::Number(-v.as_f32().unwrap_or(0.0)),
                    UnaryOp::Not => JsValue::Bool(!v.truthy()),
                })
            }
            Expr::Binary { left, op, right } => {
                // short-circuit
                if *op == BinOp::And {
                    let l = self.eval(left)?;
                    if !l.truthy() {
                        return Ok(l);
                    }
                    return self.eval(right);
                }
                if *op == BinOp::Or {
                    let l = self.eval(left)?;
                    if l.truthy() {
                        return Ok(l);
                    }
                    return self.eval(right);
                }
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                Ok(eval_bin(op, &l, &r))
            }
            Expr::Index { base, index, .. } => {
                let b = self.eval(base)?;
                let i = self.eval(index)?;
                Ok(match (&b, &i) {
                    (JsValue::Object(map), JsValue::String(k)) => {
                        map.get(k).cloned().unwrap_or(JsValue::Null)
                    }
                    (JsValue::Array(arr), JsValue::Number(n)) => arr
                        .get(*n as usize)
                        .cloned()
                        .unwrap_or(JsValue::Null),
                    (JsValue::String(s), JsValue::Number(n)) => s
                        .chars()
                        .nth(*n as usize)
                        .map(|c| JsValue::String(c.to_string()))
                        .unwrap_or(JsValue::Null),
                    _ => JsValue::Null,
                })
            }
            Expr::Call { name, args } => {
                let mut argv = Vec::new();
                for a in args {
                    argv.push(self.eval(a)?);
                }
                self.call_builtin_or_fn(name, &argv)
            }
        }
    }

    fn call_builtin_or_fn(
        &mut self,
        name: &str,
        args: &[JsValue],
    ) -> Result<JsValue, ScriptError> {
        match name {
            "play" => {
                // play("deal", { target, delay, duration, ease })
                let anim = args
                    .first()
                    .map(|v| v.as_string())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ScriptError::Runtime("play(name, opts?) needs name".into()))?;
                let opts = args.get(1).cloned().unwrap_or(JsValue::Null);
                let (target, delay, duration, ease) = read_play_opts(&opts);
                self.actions.push(StyleAction::Play {
                    animation: anim,
                    target,
                    delay,
                    duration,
                    ease,
                });
                Ok(JsValue::Null)
            }
            "animate" => {
                // animate(target, { opacity: [0,1], y: [-20,0] }, duration, ease?)
                let target = args
                    .first()
                    .map(|v| v.as_string())
                    .ok_or_else(|| ScriptError::Runtime("animate(target, props, dur)".into()))?;
                let props = args.get(1).cloned().unwrap_or(JsValue::Null);
                let duration = args.get(2).and_then(|v| v.as_f32()).unwrap_or(0.35);
                let ease = args
                    .get(3)
                    .map(|v| v.as_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "cubic_out".into());
                let delay = match &props {
                    JsValue::Object(m) => m.get("delay").and_then(|v| v.as_f32()).unwrap_or(0.0),
                    _ => 0.0,
                };
                let mut channels = Vec::new();
                if let JsValue::Object(m) = &props {
                    for (k, v) in m {
                        if k == "delay" || k == "ease" || k == "duration" {
                            continue;
                        }
                        if let JsValue::Array(pair) = v {
                            if pair.len() >= 2 {
                                let from = pair[0].as_f32().unwrap_or(0.0);
                                let to = pair[1].as_f32().unwrap_or(0.0);
                                channels.push((k.clone(), from, to));
                            }
                        } else if let Some(to) = v.as_f32() {
                            // single value → animate to (from current unknown → use 0)
                            channels.push((k.clone(), 0.0, to));
                        }
                    }
                }
                self.actions.push(StyleAction::Animate {
                    target: strip_hash(&target),
                    channels,
                    duration,
                    delay,
                    ease,
                });
                Ok(JsValue::Null)
            }
            "wait" => {
                let secs = args.first().and_then(|v| v.as_f32()).unwrap_or(0.0);
                self.actions.push(StyleAction::Wait { secs });
                Ok(JsValue::Null)
            }
            "emit" => {
                let event = args
                    .first()
                    .map(|v| v.as_string())
                    .unwrap_or_default();
                let payload = args.get(1).map(|v| v.as_string());
                self.actions.push(StyleAction::Emit {
                    event,
                    payload,
                });
                Ok(JsValue::Null)
            }
            "str" | "String" => Ok(JsValue::String(
                args.first().map(|v| v.as_string()).unwrap_or_default(),
            )),
            "num" | "Number" => Ok(JsValue::Number(
                args.first().and_then(|v| v.as_f32()).unwrap_or(0.0),
            )),
            "len" => {
                let n = match args.first() {
                    Some(JsValue::Array(a)) => a.len() as f32,
                    Some(JsValue::String(s)) => s.len() as f32,
                    Some(JsValue::Object(o)) => o.len() as f32,
                    _ => 0.0,
                };
                Ok(JsValue::Number(n))
            }
            "min" => {
                let a = args.first().and_then(|v| v.as_f32()).unwrap_or(0.0);
                let b = args.get(1).and_then(|v| v.as_f32()).unwrap_or(a);
                Ok(JsValue::Number(a.min(b)))
            }
            "max" => {
                let a = args.first().and_then(|v| v.as_f32()).unwrap_or(0.0);
                let b = args.get(1).and_then(|v| v.as_f32()).unwrap_or(a);
                Ok(JsValue::Number(a.max(b)))
            }
            "abs" => Ok(JsValue::Number(
                args.first().and_then(|v| v.as_f32()).unwrap_or(0.0).abs(),
            )),
            "floor" => Ok(JsValue::Number(
                args.first()
                    .and_then(|v| v.as_f32())
                    .unwrap_or(0.0)
                    .floor(),
            )),
            "ceil" => Ok(JsValue::Number(
                args.first()
                    .and_then(|v| v.as_f32())
                    .unwrap_or(0.0)
                    .ceil(),
            )),
            "sin" => Ok(JsValue::Number(
                args.first()
                    .and_then(|v| v.as_f32())
                    .unwrap_or(0.0)
                    .sin(),
            )),
            "cos" => Ok(JsValue::Number(
                args.first()
                    .and_then(|v| v.as_f32())
                    .unwrap_or(0.0)
                    .cos(),
            )),
            "clamp" => {
                let x = args.first().and_then(|v| v.as_f32()).unwrap_or(0.0);
                let lo = args.get(1).and_then(|v| v.as_f32()).unwrap_or(0.0);
                let hi = args.get(2).and_then(|v| v.as_f32()).unwrap_or(1.0);
                Ok(JsValue::Number(x.clamp(lo.min(hi), lo.max(hi))))
            }
            "lerp" => {
                let a = args.first().and_then(|v| v.as_f32()).unwrap_or(0.0);
                let b = args.get(1).and_then(|v| v.as_f32()).unwrap_or(0.0);
                let t = args.get(2).and_then(|v| v.as_f32()).unwrap_or(0.0);
                Ok(JsValue::Number(a + (b - a) * t))
            }
            "rand" => {
                // simple LCG from env seed or time-ish counter
                let seed = self
                    .env
                    .get("__rand_seed")
                    .and_then(|v| v.as_f32())
                    .unwrap_or(1.0);
                let next = (seed * 1103515245.0 + 12345.0) % 2147483648.0;
                self.env
                    .insert("__rand_seed".into(), JsValue::Number(next.abs()));
                let u = (next.abs() % 10000.0) / 10000.0;
                let lo = args.first().and_then(|v| v.as_f32());
                let hi = args.get(1).and_then(|v| v.as_f32());
                Ok(JsValue::Number(match (lo, hi) {
                    (Some(a), Some(b)) => a + (b - a) * u,
                    (Some(a), None) => a * u,
                    _ => u,
                }))
            }
            "pick" => {
                let arr = args.first().cloned().unwrap_or(JsValue::Null);
                match arr {
                    JsValue::Array(items) if !items.is_empty() => {
                        let seed = self
                            .env
                            .get("__rand_seed")
                            .and_then(|v| v.as_f32())
                            .unwrap_or(3.0);
                        let next = (seed * 1103515245.0 + 12345.0) % 2147483648.0;
                        self.env
                            .insert("__rand_seed".into(), JsValue::Number(next.abs()));
                        let i = (next.abs() as usize) % items.len();
                        Ok(items[i].clone())
                    }
                    _ => Ok(JsValue::Null),
                }
            }
            "set" => {
                // set(target, prop, value)
                let target = args
                    .first()
                    .map(|v| strip_hash(&v.as_string()))
                    .unwrap_or_default();
                let prop = args
                    .get(1)
                    .map(|v| v.as_string())
                    .unwrap_or_else(|| "opacity".into());
                let value = args.get(2).and_then(|v| v.as_f32()).unwrap_or(0.0);
                if !target.is_empty() {
                    self.actions.push(StyleAction::Set {
                        target: target.clone(),
                        prop: prop.clone(),
                        value,
                    });
                    if let Some(rt) = self.runtime.as_mut() {
                        rt.set(&target, &prop, value);
                    }
                }
                Ok(JsValue::Null)
            }
            "query" | "get" => {
                // query(target, prop) → number from runtime
                let target = args
                    .first()
                    .map(|v| strip_hash(&v.as_string()))
                    .unwrap_or_default();
                let prop = args
                    .get(1)
                    .map(|v| v.as_string())
                    .unwrap_or_else(|| "opacity".into());
                let v = self
                    .runtime
                    .as_ref()
                    .map(|rt| rt.get(&target, &prop))
                    .unwrap_or(0.0);
                Ok(JsValue::Number(v))
            }
            other if self.module.functions.contains_key(other) => self.run_fn(other, args),
            other => Err(ScriptError::Runtime(format!("unknown function `{other}`"))),
        }
    }
}

fn strip_hash(s: &str) -> String {
    s.trim().trim_start_matches('#').to_string()
}

fn read_play_opts(opts: &JsValue) -> (Option<String>, f32, Option<f32>, Option<String>) {
    let mut target = None;
    let mut delay = 0.0f32;
    let mut duration = None;
    let mut ease = None;
    if let JsValue::Object(m) = opts {
        if let Some(t) = m.get("target") {
            let s = t.as_string();
            if !s.is_empty() {
                target = Some(strip_hash(&s));
            }
        }
        if let Some(d) = m.get("delay").and_then(|v| v.as_f32()) {
            delay = d;
        }
        if let Some(d) = m.get("duration").and_then(|v| v.as_f32()) {
            duration = Some(d);
        }
        if let Some(e) = m.get("ease").or_else(|| m.get("easing")) {
            let s = e.as_string();
            if !s.is_empty() {
                ease = Some(s);
            }
        }
    } else if let JsValue::String(s) = opts {
        // play("deal", "card0")
        if !s.is_empty() {
            target = Some(strip_hash(s));
        }
    }
    (target, delay, duration, ease)
}

fn eval_bin(op: &BinOp, l: &JsValue, r: &JsValue) -> JsValue {
    match op {
        BinOp::Add => {
            // string concat if either is string
            if matches!(l, JsValue::String(_)) || matches!(r, JsValue::String(_)) {
                return JsValue::String(format!("{}{}", l.as_string(), r.as_string()));
            }
            JsValue::Number(l.as_f32().unwrap_or(0.0) + r.as_f32().unwrap_or(0.0))
        }
        BinOp::Sub => JsValue::Number(l.as_f32().unwrap_or(0.0) - r.as_f32().unwrap_or(0.0)),
        BinOp::Mul => JsValue::Number(l.as_f32().unwrap_or(0.0) * r.as_f32().unwrap_or(0.0)),
        BinOp::Div => {
            let d = r.as_f32().unwrap_or(1.0);
            JsValue::Number(if d == 0.0 {
                0.0
            } else {
                l.as_f32().unwrap_or(0.0) / d
            })
        }
        BinOp::Mod => {
            let d = r.as_f32().unwrap_or(1.0);
            JsValue::Number(if d == 0.0 {
                0.0
            } else {
                l.as_f32().unwrap_or(0.0) % d
            })
        }
        BinOp::Lt => JsValue::Bool(l.as_f32().unwrap_or(0.0) < r.as_f32().unwrap_or(0.0)),
        BinOp::Le => JsValue::Bool(l.as_f32().unwrap_or(0.0) <= r.as_f32().unwrap_or(0.0)),
        BinOp::Gt => JsValue::Bool(l.as_f32().unwrap_or(0.0) > r.as_f32().unwrap_or(0.0)),
        BinOp::Ge => JsValue::Bool(l.as_f32().unwrap_or(0.0) >= r.as_f32().unwrap_or(0.0)),
        BinOp::Eq => JsValue::Bool(values_eq(l, r)),
        BinOp::Ne => JsValue::Bool(!values_eq(l, r)),
        BinOp::And | BinOp::Or => unreachable!("short-circuit"),
    }
}

fn values_eq(l: &JsValue, r: &JsValue) -> bool {
    match (l, r) {
        (JsValue::Number(a), JsValue::Number(b)) => (a - b).abs() < 1e-6,
        (JsValue::String(a), JsValue::String(b)) => a == b,
        (JsValue::Bool(a), JsValue::Bool(b)) => a == b,
        (JsValue::Null, JsValue::Null) => true,
        _ => l.as_string() == r.as_string(),
    }
}

/// Run a named function from a script module, optionally resolving timelines via sheet.
pub fn run_function(
    module: &ScriptModule,
    sheet: Option<&Stylesheet>,
    name: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    run_function_with_runtime(module, sheet, name, args, None)
}

/// Like [`run_function`] but `set`/`query` touch a [`StyleRuntime`].
pub fn run_function_with_runtime(
    module: &ScriptModule,
    sheet: Option<&Stylesheet>,
    name: &str,
    args: &[JsValue],
    mut runtime: Option<&mut StyleRuntime>,
) -> Result<ScriptRun, ScriptError> {
    let mut env = IndexMap::new();
    // seed globals (no runtime needed)
    {
        let mut seed_vm = Vm {
            module,
            env: IndexMap::new(),
            actions: Vec::new(),
            runtime: None,
        };
        for (k, expr) in &module.globals {
            let v = seed_vm.eval(expr)?;
            env.insert(k.clone(), v);
        }
    }
    let mut vm = Vm {
        module,
        env,
        actions: Vec::new(),
        runtime: runtime.as_deref_mut(),
    };
    let value = vm.run_fn(name, args)?;
    let actions = vm.actions;
    let timelines = actions_to_timelines(sheet, &actions);
    Ok(ScriptRun {
        actions,
        timelines,
        value,
    })
}

/// Fire all handlers registered for `event`.
pub fn run_event(
    module: &ScriptModule,
    sheet: Option<&Stylesheet>,
    event: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    let mut combined = ScriptRun::default();
    for h in &module.handlers {
        if h.event == event {
            let r = run_function(module, sheet, &h.fn_name, args)?;
            combined.actions.extend(r.actions);
            combined.timelines.extend(r.timelines);
            combined.value = r.value;
        }
    }
    Ok(combined)
}

/// Convert Play/Animate actions into [`TimelinePlan`]s when a sheet is available.
pub fn actions_to_timelines(
    sheet: Option<&Stylesheet>,
    actions: &[StyleAction],
) -> Vec<TimelinePlan> {
    let mut out = Vec::new();
    for a in actions {
        match a {
            StyleAction::Play {
                animation,
                target,
                delay,
                duration,
                ease,
            } => {
                let Some(sheet) = sheet else { continue };
                let mut spec = AnimationSpec {
                    name: animation.clone(),
                    delay: *delay,
                    target: target.clone(),
                    ..Default::default()
                };
                if let Some(d) = duration {
                    spec.duration = *d;
                }
                if let Some(e) = ease {
                    spec.easing = e.clone();
                }
                // fall back to class `.animation` defaults if duration still default-ish
                if let Some(plan) = plan_from_spec(sheet, &spec) {
                    out.push(plan);
                }
            }
            StyleAction::Animate {
                target,
                channels,
                duration,
                delay,
                ease,
            } => {
                let mut plans = Vec::new();
                for (ch, from, to) in channels {
                    plans.push(crate::animation::ChannelPlan {
                        channel: ch.clone(),
                        keys: vec![(*delay, *from), (*delay + *duration, *to)],
                        ease: ease.clone(),
                    });
                }
                if !plans.is_empty() {
                    out.push(TimelinePlan {
                        channels: plans,
                        duration: *delay + *duration,
                        target: Some(target.clone()),
                    });
                }
            }
            StyleAction::Wait { .. }
            | StyleAction::Emit { .. }
            | StyleAction::Set { .. } => {}
        }
    }
    out
}

/// Convenience: parse script body and run a function.
pub fn eval_script_fn(
    script_src: &str,
    sheet: Option<&Stylesheet>,
    name: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    let module = parse_script(script_src)?;
    run_function(&module, sheet, name, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_stylesheet;

    #[test]
    fn deal_hand_stagger() {
        let src = r##"
            let stagger = 0.08;
            fn dealHand(count) {
                for (let i = 0; i < count; i = i + 1) {
                    play("deal", {
                        target: "card" + i,
                        delay: i * stagger,
                        duration: 0.32,
                        ease: "cubic_out"
                    });
                }
            }
        "##;
        let run = eval_script_fn(src, None, "dealHand", &[JsValue::num(3.0)]).unwrap();
        assert_eq!(run.actions.len(), 3);
        match &run.actions[2] {
            StyleAction::Play {
                target, delay, ..
            } => {
                assert_eq!(target.as_deref(), Some("card2"));
                assert!((*delay - 0.16).abs() < 1e-4);
            }
            _ => panic!("expected play"),
        }
    }

    #[test]
    fn animate_logo() {
        // r## so `"#logo"` does not terminate the raw string early
        let src = r##"
            fn logoIn() {
                animate("#logo", { opacity: [0, 1], y: [-24, 0] }, 0.45, "cubic_out");
            }
        "##;
        let run = eval_script_fn(src, None, "logoIn", &[]).unwrap();
        assert_eq!(run.actions.len(), 1);
        assert_eq!(run.timelines.len(), 1);
        assert_eq!(run.timelines[0].channels.len(), 2);
    }

    #[test]
    fn play_resolves_keyframes() {
        let sheet = parse_stylesheet(
            r#"
            @keyframes deal {
              from { opacity: 0; }
              to { opacity: 1; }
            }
            "#,
        )
        .unwrap();
        let src = r#"
            fn go() { play("deal", { target: "c0", duration: 0.5 }); }
        "#;
        let run = eval_script_fn(src, Some(&sheet), "go", &[]).unwrap();
        assert_eq!(run.timelines.len(), 1);
        assert!((run.timelines[0].duration - 0.5).abs() < 1e-4);
    }

    #[test]
    fn on_event() {
        let src = r#"
            fn boot() { emit("ready"); }
            on("menu.open", boot);
        "#;
        let module = parse_script(src).unwrap();
        assert_eq!(module.handlers.len(), 1);
        let run = run_event(&module, None, "menu.open", &[]).unwrap();
        assert!(matches!(run.actions[0], StyleAction::Emit { .. }));
    }

    #[test]
    fn set_query_math_helpers() {
        use crate::runtime::StyleRuntime;
        let src = r#"
            fn go() {
                set("card0", "opacity", clamp(lerp(0, 1, 0.5), 0, 1));
                let o = query("card0", "opacity");
                return o;
            }
        "#;
        let module = parse_script(src).unwrap();
        let mut rt = StyleRuntime::new();
        let run =
            run_function_with_runtime(&module, None, "go", &[], Some(&mut rt)).unwrap();
        assert!(matches!(run.actions[0], StyleAction::Set { .. }));
        assert!((rt.get("card0", "opacity") - 0.5).abs() < 1e-4);
        assert!((run.value.as_f32().unwrap_or(0.0) - 0.5).abs() < 1e-4);
    }
}
