//! Recursive-descent parser for Velvet Story (indent blocks).

use crate::ast::*;
use crate::diag::StoryDiag;
use crate::lexer::lex;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Parse result.
#[derive(Debug)]
pub struct ParseResult {
    /// File AST (may be partial on errors).
    pub file: StoryFile,
    /// Diagnostics.
    pub diags: Vec<StoryDiag>,
}

/// Parse source into AST.
pub fn parse(source: &str, file: &str) -> ParseResult {
    let lexed = lex(source, file);
    let mut p = Parser {
        tokens: lexed.tokens,
        i: 0,
        file: file.to_string(),
        diags: lexed.diags,
    };
    let items = p.parse_file();
    ParseResult {
        file: StoryFile {
            file: file.to_string(),
            items,
        },
        diags: p.diags,
    }
}

struct Parser {
    tokens: Vec<Token>,
    i: usize,
    file: String,
    diags: Vec<StoryDiag>,
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.i).unwrap_or_else(|| {
            self.tokens.last().expect("eof token")
        })
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if self.i < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    /// Skip blank lines only — **comments are kept** so they become `Stmt::Comment`
    /// and survive the formatter (Velvet 2.5).
    fn skip_newlines(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline) {
            self.bump();
        }
    }

    /// Skip newlines and comments (use only when comments must not enter the AST).
    fn skip_nl_comments(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Comment(_) => {
                    self.bump();
                }
                _ => break,
            }
        }
    }

    fn error(&mut self, code: &str, msg: impl Into<String>, span: Span) {
        self.diags
            .push(StoryDiag::error(code, msg, self.file.clone(), span));
    }

    fn parse_file(&mut self) -> Vec<TopItem> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek_kind(), TokenKind::Eof) {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::Eof) {
                break;
            }
            // Top-level comments: discard (not attached to a scene).
            if matches!(self.peek_kind(), TokenKind::Comment(_)) {
                self.bump();
                continue;
            }
            // recover from stray dedent
            if matches!(self.peek_kind(), TokenKind::Dedent) {
                self.bump();
                continue;
            }
            match self.parse_top() {
                Some(it) => items.push(it),
                None => {
                    // recovery: skip to next newline
                    let span = self.peek().span;
                    self.error("VST010", "No se pudo interpretar esta línea.", span);
                    while !matches!(
                        self.peek_kind(),
                        TokenKind::Newline | TokenKind::Eof | TokenKind::Dedent
                    ) {
                        self.bump();
                    }
                    self.skip_newlines();
                }
            }
        }
        items
    }

    fn parse_top(&mut self) -> Option<TopItem> {
        self.skip_newlines();
        let t = self.peek().clone();
        match &t.kind {
            TokenKind::Ident(name) if name == "scene" => {
                self.bump();
                let name_tok = self.expect_ident()?;
                let name = name_tok;
                let span = t.span;
                self.expect_newline_or_eof();
                let body = self.parse_block_body();
                Some(TopItem::Scene(Scene {
                    name,
                    body,
                    span,
                    origin_file: Some(self.file.clone()),
                }))
            }
            TokenKind::Ident(name) if name == "include" => {
                self.bump();
                let path = match self.peek_kind() {
                    TokenKind::String(s) => {
                        let s = s.clone();
                        self.bump();
                        s
                    }
                    TokenKind::Ident(s) => {
                        let s = s.clone();
                        self.bump();
                        s
                    }
                    _ => {
                        self.error("VST011", "Tras include hace falta una ruta.", t.span);
                        return None;
                    }
                };
                self.expect_newline_or_eof();
                Some(TopItem::Include {
                    path,
                    span: t.span,
                })
            }
            TokenKind::Ident(name) if name == "character" => {
                self.bump();
                let n = self.expect_ident()?;
                let display = if matches!(self.peek_kind(), TokenKind::String(_)) {
                    if let TokenKind::String(s) = self.bump().kind {
                        Some(s)
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.expect_newline_or_eof();
                Some(TopItem::CharacterDecl {
                    name: n,
                    display,
                    span: t.span,
                })
            }
            _ => None,
        }
    }

    fn parse_block_body(&mut self) -> Vec<Stmt> {
        let mut body = Vec::new();
        // optional indent
        if matches!(self.peek_kind(), TokenKind::Indent(_)) {
            self.bump();
            loop {
                self.skip_newlines();
                if matches!(self.peek_kind(), TokenKind::Dedent) {
                    self.bump();
                    break;
                }
                if matches!(self.peek_kind(), TokenKind::Eof) {
                    break;
                }
                if let Some(st) = self.parse_stmt() {
                    body.push(st);
                } else {
                    // recovery
                    while !matches!(
                        self.peek_kind(),
                        TokenKind::Newline | TokenKind::Eof | TokenKind::Dedent
                    ) {
                        self.bump();
                    }
                    self.skip_newlines();
                }
            }
        } else {
            // statements at same level until next scene/top or eof
            loop {
                self.skip_newlines();
                if matches!(self.peek_kind(), TokenKind::Eof) {
                    break;
                }
                if let TokenKind::Ident(n) = self.peek_kind() {
                    if n == "scene" || n == "include" || n == "character" {
                        break;
                    }
                }
                if matches!(self.peek_kind(), TokenKind::Dedent) {
                    break;
                }
                if let Some(st) = self.parse_stmt() {
                    body.push(st);
                } else {
                    break;
                }
            }
        }
        body
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        self.skip_newlines();
        let t = self.peek().clone();
        match &t.kind {
            TokenKind::Comment(text) => {
                let text = text.clone();
                self.bump();
                self.skip_newlines();
                Some(Stmt::Comment {
                    text,
                    span: t.span,
                })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                match name.as_str() {
                    "background" => {
                        self.bump();
                        let id = self.expect_ident_or_string()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Background { id, span: t.span })
                    }
                    "music" => {
                        self.bump();
                        let id = self.expect_ident_or_string()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Music { id, span: t.span })
                    }
                    "sound" | "sfx" => {
                        self.bump();
                        let id = self.expect_ident_or_string()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Sound { id, span: t.span })
                    }
                    "show" => {
                        self.bump();
                        let character = self.expect_ident()?;
                        let mut expression = None;
                        let mut at = None;
                        // optional expression then optional `at pos`
                        if let TokenKind::Ident(e) = self.peek_kind() {
                            if e != "at" {
                                expression = Some(e.clone());
                                self.bump();
                            }
                        }
                        if let TokenKind::Ident(a) = self.peek_kind() {
                            if a == "at" {
                                self.bump();
                                at = self.expect_ident();
                            }
                        }
                        self.expect_newline_or_eof();
                        Some(Stmt::Show {
                            character,
                            expression,
                            at,
                            span: t.span,
                        })
                    }
                    "hide" => {
                        self.bump();
                        let character = self.expect_ident()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Hide {
                            character,
                            span: t.span,
                        })
                    }
                    "goto" | "jump" => {
                        self.bump();
                        let target = self.expect_ident()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Goto {
                            target,
                            span: t.span,
                        })
                    }
                    "call" => {
                        self.bump();
                        // call scene X  OR  call cmd.name: kwargs
                        if let TokenKind::Ident(n) = self.peek_kind() {
                            if n == "scene" {
                                self.bump();
                                let target = self.expect_ident()?;
                                self.expect_newline_or_eof();
                                return Some(Stmt::CallScene {
                                    target,
                                    span: t.span,
                                });
                            }
                        }
                        let cmd = self.expect_ident()?;
                        let mut args = Vec::new();
                        if matches!(self.peek_kind(), TokenKind::Colon) {
                            self.bump();
                            self.expect_newline_or_eof();
                            if matches!(self.peek_kind(), TokenKind::Indent(_)) {
                                self.bump();
                                loop {
                                    self.skip_newlines();
                                    if matches!(self.peek_kind(), TokenKind::Dedent) {
                                        self.bump();
                                        break;
                                    }
                                    if matches!(self.peek_kind(), TokenKind::Eof) {
                                        break;
                                    }
                                    let key = self.expect_ident()?;
                                    if !matches!(self.peek_kind(), TokenKind::Colon) {
                                        self.error(
                                            "VST012",
                                            "En call, usa `parametro: valor`.",
                                            self.peek().span,
                                        );
                                        break;
                                    }
                                    self.bump();
                                    let val = self.parse_expr()?;
                                    self.expect_newline_or_eof();
                                    args.push((key, val));
                                }
                            }
                        } else {
                            self.expect_newline_or_eof();
                        }
                        Some(Stmt::CallCommand {
                            name: cmd,
                            args,
                            span: t.span,
                        })
                    }
                    "return" => {
                        self.bump();
                        self.expect_newline_or_eof();
                        Some(Stmt::Return { span: t.span })
                    }
                    "end" => {
                        self.bump();
                        self.expect_newline_or_eof();
                        Some(Stmt::End { span: t.span })
                    }
                    "label" => {
                        self.bump();
                        let name = self.expect_ident()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Label {
                            name,
                            span: t.span,
                        })
                    }
                    "set" => {
                        self.bump();
                        let var = self.expect_ident()?;
                        if !matches!(self.peek_kind(), TokenKind::Eq) {
                            self.error("VST013", "Tras set usa `set nombre = valor`.", t.span);
                        } else {
                            self.bump();
                        }
                        let value = self.parse_expr()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Set {
                            name: var,
                            value,
                            span: t.span,
                        })
                    }
                    "add" => {
                        self.bump();
                        let var = self.expect_ident()?;
                        let value = self.parse_expr()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Add {
                            name: var,
                            value,
                            span: t.span,
                        })
                    }
                    "sub" => {
                        self.bump();
                        let var = self.expect_ident()?;
                        let value = self.parse_expr()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Sub {
                            name: var,
                            value,
                            span: t.span,
                        })
                    }
                    "if" => {
                        self.bump();
                        let cond = self.parse_expr()?;
                        if matches!(self.peek_kind(), TokenKind::Colon) {
                            self.bump();
                        }
                        self.expect_newline_or_eof();
                        let then_body = self.parse_block_body();
                        let mut else_body = None;
                        self.skip_newlines();
                        if let TokenKind::Ident(n) = self.peek_kind() {
                            if n == "else" {
                                self.bump();
                                if matches!(self.peek_kind(), TokenKind::Colon) {
                                    self.bump();
                                }
                                self.expect_newline_or_eof();
                                else_body = Some(self.parse_block_body());
                            }
                        }
                        Some(Stmt::If {
                            cond,
                            then_body,
                            else_body,
                            span: t.span,
                        })
                    }
                    "choice" => {
                        self.bump();
                        if matches!(self.peek_kind(), TokenKind::Colon) {
                            self.bump();
                        }
                        self.expect_newline_or_eof();
                        let mut options = Vec::new();
                        if matches!(self.peek_kind(), TokenKind::Indent(_)) {
                            self.bump();
                            loop {
                                self.skip_newlines();
                                if matches!(self.peek_kind(), TokenKind::Dedent) {
                                    self.bump();
                                    break;
                                }
                                if matches!(self.peek_kind(), TokenKind::Eof) {
                                    break;
                                }
                                // "label":
                                let lab_tok = self.peek().clone();
                                let (label, msg_id) = match &lab_tok.kind {
                                    TokenKind::String(s) => {
                                        let s = s.clone();
                                        self.bump();
                                        (s, None)
                                    }
                                    TokenKind::Ident(s) => {
                                        let s = s.clone();
                                        self.bump();
                                        (s, None)
                                    }
                                    _ => {
                                        self.error(
                                            "VST014",
                                            "Cada opción de choice debe ser un texto entre comillas.",
                                            lab_tok.span,
                                        );
                                        break;
                                    }
                                };
                                if matches!(self.peek_kind(), TokenKind::Colon) {
                                    self.bump();
                                }
                                self.expect_newline_or_eof();
                                let body = self.parse_block_body();
                                options.push(ChoiceArm {
                                    label,
                                    msg_id,
                                    body,
                                    span: lab_tok.span,
                                });
                            }
                        }
                        Some(Stmt::Choice {
                            options,
                            span: t.span,
                        })
                    }
                    "pause" | "wait" => {
                        self.bump();
                        let duration = if !matches!(
                            self.peek_kind(),
                            TokenKind::Newline | TokenKind::Eof | TokenKind::Comment(_)
                        ) {
                            self.parse_expr()
                        } else {
                            None
                        };
                        self.expect_newline_or_eof();
                        Some(Stmt::Pause {
                            duration,
                            span: t.span,
                        })
                    }
                    "with" => {
                        self.bump();
                        let name = self.expect_ident_or_string()?;
                        self.expect_newline_or_eof();
                        Some(Stmt::Transition {
                            name,
                            span: t.span,
                        })
                    }
                    // dialogue: speaker [@id]:
                    _ => {
                        // speaker
                        let speaker = name;
                        self.bump();
                        let mut msg_id = None;
                        if matches!(self.peek_kind(), TokenKind::At) {
                            self.bump();
                            msg_id = self.expect_ident();
                        }
                        if !matches!(self.peek_kind(), TokenKind::Colon) {
                            self.error(
                                "VST015",
                                format!(
                                    "Línea desconocida `{speaker}`. ¿Quisiste `speaker:` para diálogo?"
                                ),
                                t.span,
                            );
                            return None;
                        }
                        self.bump();
                        // inline text after colon?
                        let mut text = String::new();
                        if let TokenKind::String(s) = self.peek_kind() {
                            text = s.clone();
                            self.bump();
                            self.expect_newline_or_eof();
                        } else {
                            self.expect_newline_or_eof();
                            if matches!(self.peek_kind(), TokenKind::Indent(_)) {
                                self.bump();
                                let mut lines = Vec::new();
                                loop {
                                    self.skip_newlines();
                                    if matches!(self.peek_kind(), TokenKind::Dedent) {
                                        self.bump();
                                        break;
                                    }
                                    if matches!(self.peek_kind(), TokenKind::Eof) {
                                        break;
                                    }
                                    // gather rest of line as text
                                    let mut line = String::new();
                                    while !matches!(
                                        self.peek_kind(),
                                        TokenKind::Newline
                                            | TokenKind::Eof
                                            | TokenKind::Dedent
                                            | TokenKind::Indent(_)
                                    ) {
                                        match self.bump().kind {
                                            TokenKind::String(s) => line.push_str(&s),
                                            TokenKind::Ident(s) => {
                                                if !line.is_empty() {
                                                    line.push(' ');
                                                }
                                                line.push_str(&s);
                                            }
                                            TokenKind::Int(n) => {
                                                if !line.is_empty() {
                                                    line.push(' ');
                                                }
                                                line.push_str(&n.to_string());
                                            }
                                            TokenKind::Comment(_) => break,
                                            other => {
                                                let s = format!("{other:?}");
                                                if !line.is_empty() {
                                                    line.push(' ');
                                                }
                                                line.push_str(&s);
                                            }
                                        }
                                    }
                                    if !line.is_empty() {
                                        lines.push(line);
                                    }
                                    if matches!(self.peek_kind(), TokenKind::Newline) {
                                        self.bump();
                                    }
                                }
                                text = lines.join("\n");
                            }
                        }
                        Some(Stmt::Dialogue {
                            speaker,
                            msg_id,
                            text,
                            span: t.span,
                        })
                    }
                }
            }
            _ => None,
        }
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;
        while matches!(self.peek_kind(), TokenKind::Or) {
            let op_span = self.bump().span;
            let right = self.parse_and()?;
            let span = Span::at(
                left.span().line,
                left.span().column,
                left.span().start,
                right.span().end,
            );
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span: if span.line == 0 { op_span } else { span },
            };
        }
        Some(left)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut left = self.parse_cmp()?;
        while matches!(self.peek_kind(), TokenKind::And) {
            self.bump();
            let right = self.parse_cmp()?;
            let span = left.span();
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_cmp(&mut self) -> Option<Expr> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.bump();
            let right = self.parse_add()?;
            let span = left.span();
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_add(&mut self) -> Option<Expr> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_mul()?;
            let span = left.span();
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_mul(&mut self) -> Option<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            let span = left.span();
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if matches!(self.peek_kind(), TokenKind::Not) {
            let sp = self.bump().span;
            let e = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(e),
                span: sp,
            });
        }
        if matches!(self.peek_kind(), TokenKind::Minus) {
            let sp = self.bump().span;
            let e = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(e),
                span: sp,
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::Int(n) => {
                self.bump();
                Some(Expr::Int(n, t.span))
            }
            TokenKind::Float(s) => {
                self.bump();
                Some(Expr::Float(s, t.span))
            }
            TokenKind::True => {
                self.bump();
                Some(Expr::Bool(true, t.span))
            }
            TokenKind::False => {
                self.bump();
                Some(Expr::Bool(false, t.span))
            }
            TokenKind::String(s) => {
                self.bump();
                Some(Expr::Str(s, t.span))
            }
            TokenKind::Ident(s) => {
                self.bump();
                Some(Expr::Ident(s, t.span))
            }
            _ => {
                self.error(
                    "VST016",
                    "Se esperaba un valor (número, texto, true/false o variable).",
                    t.span,
                );
                None
            }
        }
    }

    fn expect_ident(&mut self) -> Option<String> {
        match self.peek_kind() {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.bump();
                Some(s)
            }
            _ => {
                let sp = self.peek().span;
                self.error("VST017", "Se esperaba un nombre.", sp);
                None
            }
        }
    }

    fn expect_ident_or_string(&mut self) -> Option<String> {
        match self.peek_kind() {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.bump();
                Some(s)
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.bump();
                Some(s)
            }
            _ => {
                let sp = self.peek().span;
                self.error("VST018", "Se esperaba un identificador o texto.", sp);
                None
            }
        }
    }

    fn expect_newline_or_eof(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Comment(_) => {
                    self.bump();
                }
                TokenKind::Eof | TokenKind::Dedent | TokenKind::Indent(_) => break,
                TokenKind::Ident(_) | TokenKind::String(_) if false => break,
                _ => break,
            }
            // only consume one newline typically
            break;
        }
        // actually consume one newline if present
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.bump();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_welcome_story() {
        let src = r#"
scene start

background bedroom
show luna happy

luna:
    Hola. Bienvenido a Velvet.

set affection = 0

choice:
    "Saludar":
        add affection 1

        luna:
            Me alegra verte.

        goto ending

    "No responder":
        narrator:
            Luna guarda silencio.

        goto ending

scene ending

if affection > 0:
    narrator:
        Parece que esta historia empieza bien.
else:
    narrator:
        Quizás mañana sea diferente.

end
"#;
        let r = parse(src, "welcome.vstory");
        let scenes: Vec<_> = r
            .file
            .items
            .iter()
            .filter_map(|i| match i {
                TopItem::Scene(s) => Some(s.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(scenes.contains(&"start"));
        assert!(scenes.contains(&"ending"));
        assert!(
            r.diags.iter().filter(|d| d.is_error()).count() < 5,
            "diags: {:?}",
            r.diags
        );
    }
}
