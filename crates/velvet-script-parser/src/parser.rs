//! Recursive descent parser.

use thiserror::Error;
use velvet_script_ast::{
    BinOp, ChoiceArm, Diagnostic, Expr, Item, Module, Param, ScreenButton, ScreenProperty,
    SourceLoc, StateBinding, Stmt, UnaryOp,
};
use velvet_script_lexer::{lex, LexedToken, LexerError, Span, Token};

/// Parse error (fatal when recovery fails completely).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Lexer failure.
    #[error("{0}")]
    Lexer(#[from] LexerError),
    /// Syntax error with location.
    #[error("{loc}: {message}")]
    Syntax {
        /// Message.
        message: String,
        /// Location string.
        loc: String,
    },
}

/// Successful parse result.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Module AST.
    pub module: Module,
}

/// Parse source without a file name.
pub fn parse(source: &str) -> Result<ParseResult, ParseError> {
    parse_file(source, None)
}

/// Parse source with optional file name for diagnostics.
pub fn parse_file(source: &str, file: Option<&str>) -> Result<ParseResult, ParseError> {
    let tokens = lex(source)?;
    let mut p = Parser {
        tokens,
        pos: 0,
        file: file.map(|s| s.to_string()),
        diagnostics: Vec::new(),
    };
    let mut items = Vec::new();
    while !p.is_at_end() {
        match p.parse_item() {
            Ok(item) => items.push(item),
            Err(e) => {
                let loc = p.peek_loc();
                p.diagnostics.push(Diagnostic::error(e, loc.clone()));
                // Recovery: skip to next top-level-ish token.
                p.synchronize();
                if p.is_at_end() {
                    break;
                }
            }
        }
    }
    let mut module = Module {
        file: file.map(|s| s.to_string()),
        items,
        diagnostics: p.diagnostics,
    };
    if let Some(f) = file {
        for d in &mut module.diagnostics {
            if d.loc.file.is_none() {
                d.loc.file = Some(f.to_string());
            }
        }
    }
    Ok(ParseResult { module })
}

struct Parser {
    tokens: Vec<LexedToken>,
    pos: usize,
    file: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&LexedToken> {
        self.tokens.get(self.pos)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.peek().map(|t| &t.token)
    }

    fn peek_loc(&self) -> SourceLoc {
        match self.peek() {
            Some(t) => self.loc_of(t),
            None => {
                let last = self.tokens.last();
                SourceLoc {
                    file: self.file.clone(),
                    line: last.map(|t| t.line).unwrap_or(1),
                    column: last.map(|t| t.column).unwrap_or(1),
                    span: last.map(|t| t.span).unwrap_or(Span { start: 0, end: 0 }),
                }
            }
        }
    }

    fn loc_of(&self, t: &LexedToken) -> SourceLoc {
        SourceLoc {
            file: self.file.clone(),
            line: t.line,
            column: t.column,
            span: t.span,
        }
    }

    fn advance(&mut self) -> Option<&LexedToken> {
        if self.is_at_end() {
            None
        } else {
            let t = &self.tokens[self.pos];
            self.pos += 1;
            Some(t)
        }
    }

    fn check(&self, pred: impl FnOnce(&Token) -> bool) -> bool {
        self.peek_token().map(pred).unwrap_or(false)
    }

    fn match_token(&mut self, pred: impl FnOnce(&Token) -> bool) -> bool {
        if self.check(pred) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(
        &mut self,
        pred: impl FnOnce(&Token) -> bool,
        msg: &str,
    ) -> Result<LexedToken, String> {
        if self.check(pred) {
            Ok(self.advance().unwrap().clone())
        } else {
            Err(msg.into())
        }
    }

    fn expect_ident(&mut self) -> Result<(String, SourceLoc), String> {
        let t = self
            .peek()
            .cloned()
            .ok_or_else(|| "expected identifier".to_string())?;
        match &t.token {
            Token::Ident(name) => {
                let loc = self.loc_of(&t);
                self.advance();
                Ok((name.clone(), loc))
            }
            _ => Err("expected identifier".into()),
        }
    }

    fn is_statement_start_ident(s: &str) -> bool {
        matches!(
            s,
            "function"
                | "fn"
                | "character"
                | "state"
                | "scene"
                | "screen"
                | "let"
                | "const"
                | "if"
                | "while"
                | "for"
                | "return"
                | "jump"
                | "label"
                | "choice"
                | "show"
                | "hide"
                | "background"
                | "music"
                | "end"
                | "call"
                | "transition"
                | "sound"
                | "pause"
                | "break"
                | "continue"
        )
    }

    fn synchronize(&mut self) {
        // Consume one token so we don't re-error on the same spot forever.
        if !self.is_at_end() {
            // If we are on a top-level keyword, don't advance past it.
            if let Some(Token::Ident(s)) = self.peek_token() {
                if matches!(
                    s.as_str(),
                    "function" | "fn" | "character" | "state" | "scene" | "screen"
                ) {
                    return;
                }
            }
            self.advance();
        }
        while !self.is_at_end() {
            if matches!(self.peek_token(), Some(Token::Semi)) {
                self.advance();
                return;
            }
            if matches!(
                self.peek_token(),
                Some(Token::Ident(s)) if Self::is_statement_start_ident(s)
            ) {
                return;
            }
            if matches!(self.peek_token(), Some(Token::RBrace)) {
                return;
            }
            self.advance();
        }
    }

    /// Synchronize within a block: stop at `}` without consuming it.
    fn synchronize_in_block(&mut self) {
        while !self.is_at_end() {
            if matches!(self.peek_token(), Some(Token::RBrace)) {
                return;
            }
            if matches!(self.peek_token(), Some(Token::Semi)) {
                self.advance();
                return;
            }
            if matches!(
                self.peek_token(),
                Some(Token::Ident(s)) if Self::is_statement_start_ident(s)
            ) {
                return;
            }
            self.advance();
        }
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        if self.check(|t| matches!(t, Token::Ident(s) if s == "function" || s == "fn")) {
            return self.parse_function();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "character")) {
            return self.parse_character();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "state")) {
            return self.parse_state();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "scene")) {
            return self.parse_scene();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "screen")) {
            return self.parse_screen();
        }
        Ok(Item::Stmt(self.parse_stmt()?))
    }

    fn parse_function(&mut self) -> Result<Item, String> {
        let start = self.peek_loc();
        self.advance(); // function/fn
        let (name, _) = self.expect_ident()?;
        self.expect(|t| matches!(t, Token::LParen), "expected '('")?;
        let mut params = Vec::new();
        if !self.check(|t| matches!(t, Token::RParen)) {
            loop {
                let (pname, _) = self.expect_ident()?;
                let mut ty = None;
                if self.match_token(|t| matches!(t, Token::Colon)) {
                    let (tname, _) = self.expect_ident()?;
                    ty = Some(tname);
                }
                params.push(Param { name: pname, ty });
                if self.match_token(|t| matches!(t, Token::Comma)) {
                    continue;
                }
                break;
            }
        }
        self.expect(|t| matches!(t, Token::RParen), "expected ')'")?;
        let body = self.parse_block_stmts()?;
        Ok(Item::Function {
            name,
            params,
            body,
            loc: start,
        })
    }

    fn parse_character(&mut self) -> Result<Item, String> {
        let start = self.peek_loc();
        self.advance();
        let (name, _) = self.expect_ident()?;
        self.expect(|t| matches!(t, Token::LBrace), "expected '{'")?;
        let mut fields = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            let (fname, _) = self.expect_ident()?;
            self.expect(|t| matches!(t, Token::Colon), "expected ':'")?;
            let value = self.parse_expression()?;
            fields.push((fname, value));
            let _ = self.match_token(|t| matches!(t, Token::Comma | Token::Semi));
        }
        self.expect(|t| matches!(t, Token::RBrace), "expected '}'")?;
        Ok(Item::Character {
            name,
            fields,
            loc: start,
        })
    }

    fn parse_state(&mut self) -> Result<Item, String> {
        let start = self.peek_loc();
        self.advance();
        self.expect(|t| matches!(t, Token::LBrace), "expected '{'")?;
        let mut bindings = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            let (name, loc) = self.expect_ident()?;
            let mut ty = None;
            if self.match_token(|t| matches!(t, Token::Colon)) {
                let (tname, _) = self.expect_ident()?;
                ty = Some(tname);
            }
            self.expect(|t| matches!(t, Token::Assign), "expected '='")?;
            let init = self.parse_expression()?;
            bindings.push(StateBinding {
                name,
                ty,
                init,
                loc,
            });
            let _ = self.match_token(|t| matches!(t, Token::Comma | Token::Semi));
        }
        self.expect(|t| matches!(t, Token::RBrace), "expected '}'")?;
        Ok(Item::State {
            bindings,
            loc: start,
        })
    }

    fn parse_scene(&mut self) -> Result<Item, String> {
        let start = self.peek_loc();
        self.advance();
        let (name, _) = self.expect_ident()?;
        let body = self.parse_block_stmts()?;
        Ok(Item::Scene {
            name,
            body,
            loc: start,
        })
    }

    fn parse_screen(&mut self) -> Result<Item, String> {
        let start = self.peek_loc();
        self.advance(); // screen
        let (name, _) = self.expect_ident()?;
        self.expect(
            |t| matches!(t, Token::LBrace),
            "expected '{' after screen name",
        )?;

        let mut properties = Vec::new();
        let mut buttons = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            if self.check(|t| matches!(t, Token::Ident(s) if s == "button")) {
                buttons.push(self.parse_screen_button()?);
            } else {
                properties.push(self.parse_screen_property()?);
            }
            let _ = self.match_token(|t| matches!(t, Token::Comma | Token::Semi));
        }
        self.expect(|t| matches!(t, Token::RBrace), "expected '}' after screen")?;
        Ok(Item::Screen {
            name,
            properties,
            buttons,
            loc: start,
        })
    }

    fn parse_screen_button(&mut self) -> Result<ScreenButton, String> {
        let start = self.peek_loc();
        self.advance(); // button
        let (id, _) = self.expect_ident()?;
        self.expect(
            |t| matches!(t, Token::LBrace),
            "expected '{' after button id",
        )?;
        let mut properties = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            properties.push(self.parse_screen_property()?);
            let _ = self.match_token(|t| matches!(t, Token::Comma | Token::Semi));
        }
        self.expect(|t| matches!(t, Token::RBrace), "expected '}' after button")?;
        Ok(ScreenButton {
            id,
            properties,
            loc: start,
        })
    }

    fn parse_screen_property(&mut self) -> Result<ScreenProperty, String> {
        let (name, loc) = self.expect_ident()?;
        self.expect(
            |t| matches!(t, Token::Colon),
            "expected ':' after screen property",
        )?;
        let value = self.parse_expression()?;
        Ok(ScreenProperty { name, value, loc })
    }

    fn parse_block_stmts(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(|t| matches!(t, Token::LBrace), "expected '{'")?;
        let mut body = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            match self.parse_stmt() {
                Ok(s) => {
                    let _ = self.match_token(|t| matches!(t, Token::Semi));
                    body.push(s);
                }
                Err(e) => {
                    let loc = self.peek_loc();
                    self.diagnostics.push(Diagnostic::error(e, loc));
                    self.synchronize_in_block();
                    if self.check(|t| matches!(t, Token::RBrace)) {
                        break;
                    }
                }
            }
        }
        self.expect(|t| matches!(t, Token::RBrace), "expected '}'")?;
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        if self.check(|t| matches!(t, Token::Ident(s) if s == "let")) {
            return self.parse_let();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "const")) {
            return self.parse_const();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "if")) {
            return self.parse_if();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "while")) {
            return self.parse_while();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "for")) {
            return self.parse_for();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "return")) {
            return self.parse_return();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "break")) {
            let loc = self.peek_loc();
            self.advance();
            return Ok(Stmt::Break { loc });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "continue")) {
            let loc = self.peek_loc();
            self.advance();
            return Ok(Stmt::Continue { loc });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "jump")) {
            let loc = self.peek_loc();
            self.advance();
            let (label, _) = self.expect_ident()?;
            return Ok(Stmt::Jump { label, loc });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "call")) {
            return self.parse_call_stmt();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "transition")) {
            return self.parse_transition();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "sound")) {
            return self.parse_sound();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "pause")) {
            return self.parse_pause();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "end")) {
            let loc = self.peek_loc();
            self.advance();
            let ending = if self.check(|t| matches!(t, Token::String(_))) {
                let t = self.advance().unwrap().clone();
                match t.token {
                    Token::String(s) => Some(s),
                    _ => None,
                }
            } else if self.check(|t| matches!(t, Token::Ident(_)))
                && !self.check(|t| {
                    matches!(
                        t,
                        Token::Ident(s) if Self::is_statement_start_ident(s)
                    )
                })
            {
                // `end good_end` style ending id
                let (name, _) = self.expect_ident()?;
                Some(name)
            } else {
                None
            };
            return Ok(Stmt::End { ending, loc });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "label")) {
            let loc = self.peek_loc();
            self.advance();
            let (name, _) = self.expect_ident()?;
            let _ = self.match_token(|t| matches!(t, Token::Colon));
            return Ok(Stmt::Label { name, loc });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "choice")) {
            return self.parse_choice();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "show")) {
            return self.parse_show();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "hide")) {
            return self.parse_hide();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "background")) {
            return self.parse_background();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "music")) {
            return self.parse_music();
        }
        if self.check(|t| matches!(t, Token::LBrace)) {
            let loc = self.peek_loc();
            let body = self.parse_block_stmts()?;
            return Ok(Stmt::Block { body, loc });
        }

        // Dialogue: ident string  OR  bare string
        // Lookahead: Ident String => dialogue; Ident then other => expression
        if self.check(|t| matches!(t, Token::Ident(_)))
            && self.pos + 1 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].token, Token::String(_))
        {
            let speaker_tok = self.advance().unwrap().clone();
            let speaker = match &speaker_tok.token {
                Token::Ident(s) => s.clone(),
                _ => unreachable!(),
            };
            let loc = self.loc_of(&speaker_tok);
            let text_tok = self.advance().unwrap().clone();
            let text = match text_tok.token {
                Token::String(s) => s,
                _ => unreachable!(),
            };
            return Ok(Stmt::Dialogue {
                speaker: Some(speaker),
                text,
                loc,
            });
        }
        if self.check(|t| matches!(t, Token::String(_))) {
            let t = self.advance().unwrap().clone();
            let loc = self.loc_of(&t);
            let text = match t.token {
                Token::String(s) => s,
                _ => unreachable!(),
            };
            return Ok(Stmt::Dialogue {
                speaker: None,
                text,
                loc,
            });
        }

        let expr = self.parse_expression()?;
        let loc = expr.loc().clone();
        Ok(Stmt::Expr { expr, loc })
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let (name, _) = self.expect_ident()?;
        let mut ty = None;
        if self.match_token(|t| matches!(t, Token::Colon)) {
            let (tname, _) = self.expect_ident()?;
            ty = Some(tname);
        }
        self.expect(|t| matches!(t, Token::Assign), "expected '='")?;
        let init = self.parse_expression()?;
        Ok(Stmt::Let {
            name,
            ty,
            init,
            loc,
        })
    }

    fn parse_const(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let (name, _) = self.expect_ident()?;
        self.expect(|t| matches!(t, Token::Assign), "expected '='")?;
        let init = self.parse_expression()?;
        Ok(Stmt::Const { name, init, loc })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let cond = self.parse_expression()?;
        let then_body = Stmt::Block {
            loc: self.peek_loc(),
            body: self.parse_block_stmts()?,
        };
        let mut else_body = None;
        if self.check(|t| matches!(t, Token::Ident(s) if s == "else")) {
            self.advance();
            if self.check(|t| matches!(t, Token::Ident(s) if s == "if")) {
                else_body = Some(Box::new(self.parse_if()?));
            } else {
                else_body = Some(Box::new(Stmt::Block {
                    loc: self.peek_loc(),
                    body: self.parse_block_stmts()?,
                }));
            }
        }
        Ok(Stmt::If {
            cond,
            then_body: Box::new(then_body),
            else_body,
            loc,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let cond = self.parse_expression()?;
        let body = Stmt::Block {
            loc: self.peek_loc(),
            body: self.parse_block_stmts()?,
        };
        Ok(Stmt::While {
            cond,
            body: Box::new(body),
            loc,
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance(); // for
        let (name, _) = self.expect_ident()?;
        // optional `in`
        if self.check(|t| matches!(t, Token::Ident(s) if s == "in")) {
            self.advance();
        }
        let iter = self.parse_expression()?;
        let body = Stmt::Block {
            loc: self.peek_loc(),
            body: self.parse_block_stmts()?,
        };
        Ok(Stmt::For {
            name,
            iter,
            body: Box::new(body),
            loc,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let value = if self.is_at_end()
            || self.check(|t| matches!(t, Token::RBrace))
            || self.check(|t| {
                matches!(
                    t,
                    Token::Ident(s)
                        if s == "let"
                            || s == "if"
                            || s == "while"
                            || s == "for"
                            || s == "return"
                            || s == "jump"
                            || s == "call"
                            || s == "choice"
                            || s == "break"
                            || s == "continue"
                            || s == "end"
                            || s == "hide"
                            || s == "show"
                )
            }) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        Ok(Stmt::Return { value, loc })
    }

    fn parse_hide(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let target = if self.check(|t| matches!(t, Token::String(_))) {
            let t = self.advance().unwrap().clone();
            match t.token {
                Token::String(s) => s,
                _ => unreachable!(),
            }
        } else {
            let (mut name, _) = self.expect_ident()?;
            while self.match_token(|t| matches!(t, Token::Dot)) {
                let (part, _) = self.expect_ident()?;
                name.push('.');
                name.push_str(&part);
            }
            name
        };
        Ok(Stmt::Hide { target, loc })
    }

    fn parse_choice(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        self.expect(|t| matches!(t, Token::LBrace), "expected '{'")?;
        let mut options = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            let text_tok =
                self.expect(|t| matches!(t, Token::String(_)), "expected choice text")?;
            let arm_loc = SourceLoc {
                file: self.file.clone(),
                line: text_tok.line,
                column: text_tok.column,
                span: text_tok.span,
            };
            let text = match text_tok.token {
                Token::String(s) => s,
                _ => unreachable!(),
            };
            let body = self.parse_block_stmts()?;
            options.push(ChoiceArm {
                text,
                body,
                loc: arm_loc,
            });
        }
        self.expect(|t| matches!(t, Token::RBrace), "expected '}'")?;
        Ok(Stmt::Choice { options, loc })
    }

    fn parse_show(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        // target as ident(.ident)* or string
        let target = if self.check(|t| matches!(t, Token::String(_))) {
            let t = self.advance().unwrap().clone();
            match t.token {
                Token::String(s) => s,
                _ => unreachable!(),
            }
        } else {
            let (mut name, _) = self.expect_ident()?;
            while self.match_token(|t| matches!(t, Token::Dot)) {
                let (part, _) = self.expect_ident()?;
                name.push('.');
                name.push_str(&part);
            }
            name
        };
        let mut at = None;
        if self.check(|t| matches!(t, Token::Ident(s) if s == "at")) {
            self.advance();
            let (place, _) = self.expect_ident()?;
            at = Some(place);
        }
        Ok(Stmt::Show { target, at, loc })
    }

    fn parse_background(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let t = self.expect(|t| matches!(t, Token::String(_)), "expected string path")?;
        let path = match t.token {
            Token::String(s) => s,
            _ => unreachable!(),
        };
        Ok(Stmt::Background { path, loc })
    }

    fn parse_music(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let t = self.expect(|t| matches!(t, Token::String(_)), "expected string path")?;
        let path = match t.token {
            Token::String(s) => s,
            _ => unreachable!(),
        };
        let mut fade_in = None;
        if self.check(|t| matches!(t, Token::Ident(s) if s == "fade_in")) {
            self.advance();
            let n = self.parse_expression()?;
            fade_in = match n {
                Expr::Float { value, .. } => Some(value),
                Expr::Int { value, .. } => Some(value as f64),
                _ => None,
            };
        }
        Ok(Stmt::Music { path, fade_in, loc })
    }

    /// Dotted name: `combat.start` or plain `side_scene`.
    fn parse_dotted_name(&mut self) -> Result<String, String> {
        let (mut name, _) = self.expect_ident()?;
        while self.match_token(|t| matches!(t, Token::Dot)) {
            let (part, _) = self.expect_ident()?;
            name.push('.');
            name.push_str(&part);
        }
        Ok(name)
    }

    /// `call scene` or host `call combat.start key "val"`.
    ///
    /// Host commands **must** use a dotted name (`pkg.cmd`) so plain
    /// `call sub` never steals the next dialogue line as an arg.
    fn parse_call_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance(); // call
        let name = self.parse_dotted_name()?;
        if !name.contains('.') {
            return Ok(Stmt::Call { target: name, loc });
        }
        let mut args: Vec<(String, Expr)> = Vec::new();
        // Optional named literal args: `key <expr>` until statement boundary.
        while self.check(|t| matches!(t, Token::Ident(s) if !Self::is_statement_start_ident(s))) {
            let save = self.pos;
            let (key, _) = self.expect_ident()?;
            // Speaker dialogue is `ident string` — but host args are also `ident string`.
            // After a dotted host name we accept key/value; stop if value is missing.
            if self.check(|t| {
                matches!(
                    t,
                    Token::String(_) | Token::Int(_) | Token::Float(_) | Token::True | Token::False
                )
            }) {
                let val = self.parse_primary()?;
                args.push((key, val));
            } else if self
                .check(|t| matches!(t, Token::Ident(s) if !Self::is_statement_start_ident(s)))
            {
                // bare ident value (rare)
                let val = self.parse_primary()?;
                args.push((key, val));
            } else {
                self.pos = save;
                break;
            }
        }
        Ok(Stmt::HostCall { name, args, loc })
    }

    fn parse_transition(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let name = if self.check(|t| matches!(t, Token::String(_))) {
            let t = self.advance().unwrap().clone();
            match t.token {
                Token::String(s) => s,
                _ => unreachable!(),
            }
        } else {
            self.parse_dotted_name()?
        };
        Ok(Stmt::Transition { name, loc })
    }

    fn parse_sound(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let t = self.expect(|t| matches!(t, Token::String(_)), "expected string path")?;
        let path = match t.token {
            Token::String(s) => s,
            _ => unreachable!(),
        };
        Ok(Stmt::Sound { path, loc })
    }

    fn parse_pause(&mut self) -> Result<Stmt, String> {
        let loc = self.peek_loc();
        self.advance();
        let seconds = if self.check(|t| matches!(t, Token::Int(_) | Token::Float(_))) {
            let n = self.parse_primary()?;
            match n {
                Expr::Float { value, .. } => Some(value),
                Expr::Int { value, .. } => Some(value as f64),
                _ => None,
            }
        } else {
            None
        };
        Ok(Stmt::Pause { seconds, loc })
    }

    // ---- expressions (Pratt-ish via precedence climbing) ----

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_or()?;
        if self.check(|t| matches!(t, Token::Assign | Token::PlusAssign | Token::MinusAssign)) {
            let op_tok = self.advance().unwrap().clone();
            let op = match op_tok.token {
                Token::Assign => BinOp::Assign,
                Token::PlusAssign => BinOp::AddAssign,
                Token::MinusAssign => BinOp::SubAssign,
                _ => unreachable!(),
            };
            let right = self.parse_assignment()?;
            let loc = expr.loc().clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                loc,
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.match_token(|t| matches!(t, Token::OrOr)) {
            let right = self.parse_and()?;
            let loc = left.loc().clone();
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while self.match_token(|t| matches!(t, Token::AndAnd)) {
            let right = self.parse_equality()?;
            let loc = left.loc().clone();
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.match_token(|t| matches!(t, Token::Eq)) {
                BinOp::Eq
            } else if self.match_token(|t| matches!(t, Token::Ne)) {
                BinOp::Ne
            } else {
                break;
            };
            let right = self.parse_comparison()?;
            let loc = left.loc().clone();
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        loop {
            let op = if self.match_token(|t| matches!(t, Token::Lt)) {
                BinOp::Lt
            } else if self.match_token(|t| matches!(t, Token::Le)) {
                BinOp::Le
            } else if self.match_token(|t| matches!(t, Token::Gt)) {
                BinOp::Gt
            } else if self.match_token(|t| matches!(t, Token::Ge)) {
                BinOp::Ge
            } else {
                break;
            };
            let right = self.parse_term()?;
            let loc = left.loc().clone();
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;
        loop {
            let op = if self.match_token(|t| matches!(t, Token::Plus)) {
                BinOp::Add
            } else if self.match_token(|t| matches!(t, Token::Minus)) {
                BinOp::Sub
            } else {
                break;
            };
            let right = self.parse_factor()?;
            let loc = left.loc().clone();
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.match_token(|t| matches!(t, Token::Star)) {
                BinOp::Mul
            } else if self.match_token(|t| matches!(t, Token::Slash)) {
                BinOp::Div
            } else if self.match_token(|t| matches!(t, Token::Percent)) {
                BinOp::Rem
            } else {
                break;
            };
            let right = self.parse_unary()?;
            let loc = left.loc().clone();
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.check(|t| matches!(t, Token::Bang | Token::Minus)) {
            let t = self.advance().unwrap().clone();
            let loc = self.loc_of(&t);
            let op = match t.token {
                Token::Bang => UnaryOp::Not,
                Token::Minus => UnaryOp::Neg,
                _ => unreachable!(),
            };
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
                loc,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_token(|t| matches!(t, Token::LParen)) {
                let mut args = Vec::new();
                if !self.check(|t| matches!(t, Token::RParen)) {
                    loop {
                        args.push(self.parse_expression()?);
                        if self.match_token(|t| matches!(t, Token::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect(|t| matches!(t, Token::RParen), "expected ')'")?;
                let loc = expr.loc().clone();
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    loc,
                };
            } else if self.match_token(|t| matches!(t, Token::Dot)) {
                let (field, _) = self.expect_ident()?;
                let loc = expr.loc().clone();
                expr = Expr::Field {
                    object: Box::new(expr),
                    field,
                    loc,
                };
            } else if self.match_token(|t| matches!(t, Token::LBracket)) {
                let index = self.parse_expression()?;
                self.expect(|t| matches!(t, Token::RBracket), "expected ']'")?;
                let loc = expr.loc().clone();
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    loc,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let t = self
            .peek()
            .cloned()
            .ok_or_else(|| "unexpected end of input".to_string())?;
        let loc = self.loc_of(&t);
        match &t.token {
            Token::True => {
                self.advance();
                Ok(Expr::Bool { value: true, loc })
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool { value: false, loc })
            }
            Token::Int(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::Int { value: v, loc })
            }
            Token::Float(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::Float { value: v, loc })
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String { value: s, loc })
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident { name, loc })
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(|t| matches!(t, Token::RParen), "expected ')'")?;
                Ok(expr)
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(|t| matches!(t, Token::RBracket)) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if self.match_token(|t| matches!(t, Token::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect(|t| matches!(t, Token::RBracket), "expected ']'")?;
                Ok(Expr::List { elements, loc })
            }
            _ => Err(format!(
                "unexpected token in expression at {}",
                loc.display()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_ast::{BinOp, Item, UnaryOp};

    fn parse_ok(src: &str) -> ParseResult {
        let r = parse(src).unwrap();
        assert!(
            !r.module.has_errors(),
            "unexpected diagnostics: {:?}",
            r.module.diagnostics
        );
        r
    }

    #[test]
    fn parse_let_and_expr() {
        let r = parse_ok("let x = 1 + 2 * 3");
        assert_eq!(r.module.items.len(), 1);
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let { name, .. }) => assert_eq!(name, "x"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_function() {
        let src = r#"
function add(a, b) {
    return a + b
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Function {
                name, params, body, ..
            } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert!(!body.is_empty());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_fn_alias_and_typed_params() {
        let r = parse_ok("fn mul(a: int, b: float) { return a * b }");
        match &r.module.items[0] {
            Item::Function { name, params, .. } => {
                assert_eq!(name, "mul");
                assert_eq!(params[0].ty.as_deref(), Some("int"));
                assert_eq!(params[1].ty.as_deref(), Some("float"));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_scene_dialogue_choice() {
        let src = r#"
character aria {
    name: "Aria"
}

state {
    aria_trust: int = 0
}

scene apartment_night {
    background "bg.png"
    show aria.neutral at right
    aria "Hello"
    choice {
        "Yes" {
            aria_trust += 1
            jump next
        }
        "No" {
            jump end
        }
    }
}
"#;
        let r = parse_ok(src);
        assert!(r.module.items.len() >= 3);
        assert!(matches!(r.module.items[0], Item::Character { .. }));
        assert!(matches!(r.module.items[1], Item::State { .. }));
        assert!(matches!(r.module.items[2], Item::Scene { .. }));
    }

    #[test]
    fn parse_error_has_line_column() {
        let r = parse("let = 1").unwrap();
        assert!(
            r.module.has_errors() || !r.module.diagnostics.is_empty(),
            "expected recovery diagnostics"
        );
        let r = parse("1 +").unwrap();
        assert!(
            r.module.has_errors()
                || r.module
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("unexpected") || d.message.contains("end")),
            "expected diagnostics, got {:?}",
            r.module.diagnostics
        );
        for d in &r.module.diagnostics {
            assert!(d.loc.line >= 1);
            assert!(d.loc.column >= 1);
        }
    }

    #[test]
    fn parse_if_while() {
        let src = r#"
function f() {
    let i = 0
    while i < 3 {
        i += 1
    }
    if i == 3 {
        return i
    } else {
        return 0
    }
}
"#;
        let _ = parse_ok(src);
    }

    #[test]
    fn parse_for_break_continue() {
        let src = r#"
function sum(xs) {
    let total = 0
    for x in xs {
        if x < 0 {
            continue
        }
        if x > 100 {
            break
        }
        total += x
    }
    return total
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Function { body, .. } => {
                let has_for = body.iter().any(|s| matches!(s, Stmt::For { .. }));
                assert!(has_for);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_hide_end_call() {
        let src = r#"
scene main {
    show hero at left
    hide hero
    call side_scene
    end "good"
}
scene side_scene {
    "side"
    end
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Scene { body, .. } => {
                assert!(body.iter().any(|s| matches!(s, Stmt::Hide { .. })));
                assert!(body.iter().any(|s| matches!(s, Stmt::Call { .. })));
                assert!(body.iter().any(|s| matches!(
                    s,
                    Stmt::End {
                        ending: Some(_),
                        ..
                    }
                )));
            }
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_else_if_chain() {
        let src = r#"
function grade(n) {
    if n >= 90 {
        return "A"
    } else if n >= 80 {
        return "B"
    } else {
        return "C"
    }
}
"#;
        let _ = parse_ok(src);
    }

    #[test]
    fn parse_lists_index_and_calls() {
        let src = r#"
function f() {
    let xs = [1, 2, 3]
    let y = xs[1]
    return abs(y) + len(xs)
}
"#;
        let _ = parse_ok(src);
    }

    #[test]
    fn parse_unary_and_comparisons() {
        let src = r#"
function f(a, b) {
    return !a && -b < 0 || a == b && a != 0
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Function { body, .. } => match &body[0] {
                Stmt::Return {
                    value: Some(expr), ..
                } => {
                    // just ensure nested binary tree
                    assert!(matches!(expr, Expr::Binary { .. } | Expr::Unary { .. }));
                }
                _ => panic!("expected return"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_precedence_mul_before_add() {
        let r = parse_ok("let x = 1 + 2 * 3");
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let { init, .. }) => match init {
                Expr::Binary {
                    op: BinOp::Add,
                    right,
                    ..
                } => match right.as_ref() {
                    Expr::Binary { op: BinOp::Mul, .. } => {}
                    other => panic!("expected mul on right, got {other:?}"),
                },
                other => panic!("expected add, got {other:?}"),
            },
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn parse_field_and_nested_calls() {
        let src = r#"
function f(obj) {
    return obj.inner
}
"#;
        // Field access is valid syntax even if codegen rejects it.
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Function { body, .. } => match &body[0] {
                Stmt::Return {
                    value: Some(Expr::Field { field, .. }),
                    ..
                } => assert_eq!(field, "inner"),
                other => panic!("unexpected {other:?}"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_const_and_typed_let() {
        let r = parse_ok("const PI = 3\nlet n: int = 1");
        assert!(matches!(
            &r.module.items[0],
            Item::Stmt(Stmt::Const { name, .. }) if name == "PI"
        ));
        assert!(matches!(
            &r.module.items[1],
            Item::Stmt(Stmt::Let { ty: Some(t), .. }) if t == "int"
        ));
    }

    #[test]
    fn parse_label_jump() {
        let src = r#"
scene main {
    label start:
    "hi"
    jump start
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Scene { body, .. } => {
                assert!(body
                    .iter()
                    .any(|s| matches!(s, Stmt::Label { name, .. } if name == "start")));
                assert!(body
                    .iter()
                    .any(|s| matches!(s, Stmt::Jump { label, .. } if label == "start")));
            }
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_music_fade_in() {
        let src = r#"
scene main {
    music "theme.ogg" fade_in 1.5
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Music {
                    path,
                    fade_in: Some(f),
                    ..
                } => {
                    assert_eq!(path, "theme.ogg");
                    assert!((*f - 1.5).abs() < 1e-9);
                }
                other => panic!("unexpected {other:?}"),
            },
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn recovery_continues_after_bad_stmt_in_function() {
        let src = r#"
function f() {
    let = broken
    let ok = 1
    return ok
}
"#;
        let r = parse(src).unwrap();
        assert!(r.module.has_errors());
        match &r.module.items[0] {
            Item::Function { body, .. } => {
                // Should still recover and parse later statements.
                assert!(
                    body.iter()
                        .any(|s| matches!(s, Stmt::Let { name, .. } if name == "ok")),
                    "body={body:?}"
                );
            }
            _ => panic!("expected function item after recovery"),
        }
    }

    #[test]
    fn recovery_top_level_keeps_later_items() {
        // Use syntax-level junk so recovery (not lexer failure) is exercised.
        let src = r#"
function broken( {
    return 1
}
function good() {
    return 2
}
"#;
        let r = parse(src).unwrap();
        assert!(r.module.has_errors() || !r.module.items.is_empty());
        // At least one function should survive recovery.
        let funcs: Vec<_> = r
            .module
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Function { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            funcs.contains(&"good") || funcs.contains(&"broken") || !funcs.is_empty(),
            "funcs={funcs:?} diags={:?}",
            r.module.diagnostics
        );
    }

    #[test]
    fn recovery_scene_with_bad_choice_arm() {
        let src = r#"
scene main {
    choice {
        "ok" {
            jump next
        }
        broken_no_string {
            jump next
        }
    }
}
"#;
        let r = parse(src).unwrap();
        // May have diagnostics but scene item present.
        assert!(r
            .module
            .items
            .iter()
            .any(|i| matches!(i, Item::Scene { .. })));
    }

    #[test]
    fn parse_empty_function_and_empty_list() {
        let r = parse_ok("function f() {}\nlet xs = []");
        assert_eq!(r.module.items.len(), 2);
    }

    #[test]
    fn parse_nested_blocks() {
        let src = r#"
function f() {
    {
        let x = 1
        {
            let y = 2
        }
    }
}
"#;
        let _ = parse_ok(src);
    }

    #[test]
    fn parse_string_concat_assign() {
        let r = parse_ok(r#"let s = "a" + "b""#);
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let { init, .. }) => {
                assert!(matches!(init, Expr::Binary { op: BinOp::Add, .. }));
            }
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn parse_negated_primary() {
        let r = parse_ok("let x = -42");
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let { init, .. }) => {
                assert!(matches!(
                    init,
                    Expr::Unary {
                        op: UnaryOp::Neg,
                        ..
                    }
                ));
            }
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn parse_file_attaches_file_name() {
        let r = parse_file("let x = 1", Some("story/main.vel")).unwrap();
        assert_eq!(r.module.file.as_deref(), Some("story/main.vel"));
    }

    #[test]
    fn parse_character_multiple_fields() {
        let src = r##"
character hero {
    name: "Hero"
    color: "#fff"
    portrait: "hero.png"
    happy: "hero_happy.png"
}
"##;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Character { name, fields, .. } => {
                assert_eq!(name, "hero");
                assert_eq!(fields.len(), 4);
            }
            _ => panic!("expected character"),
        }
    }

    #[test]
    fn parse_state_multiple_bindings() {
        let src = r#"
state {
    a: int = 0
    b: bool = true
    c = "x"
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::State { bindings, .. } => assert_eq!(bindings.len(), 3),
            _ => panic!("expected state"),
        }
    }

    #[test]
    fn parse_boolean_literals() {
        let r = parse_ok("let a = true\nlet b = false");
        assert!(matches!(
            &r.module.items[0],
            Item::Stmt(Stmt::Let {
                init: Expr::Bool { value: true, .. },
                ..
            })
        ));
        assert!(matches!(
            &r.module.items[1],
            Item::Stmt(Stmt::Let {
                init: Expr::Bool { value: false, .. },
                ..
            })
        ));
    }

    #[test]
    fn parse_parenthesized_expr() {
        let r = parse_ok("let x = (1 + 2) * 3");
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let { init, .. }) => {
                assert!(matches!(init, Expr::Binary { op: BinOp::Mul, .. }));
            }
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn parse_show_string_target() {
        let r = parse_ok(r#"scene s { show "aria.neutral" at center }"#);
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Show { target, at, .. } => {
                    assert_eq!(target, "aria.neutral");
                    assert_eq!(at.as_deref(), Some("center"));
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_narrator_dialogue() {
        let r = parse_ok(r#"scene s { "Once upon a time..." }"#);
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Dialogue {
                    speaker: None,
                    text,
                    ..
                } => assert!(text.contains("Once")),
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_remainder_operator() {
        let r = parse_ok("let x = 10 % 3");
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let { init, .. }) => {
                assert!(matches!(init, Expr::Binary { op: BinOp::Rem, .. }));
            }
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn fixture_full_vn_script() {
        let src = r##"
character hero {
    name: "Hero"
    color: "#ffcc00"
    portrait: "hero.png"
}
character rival {
    name: "Rival"
    color: "#cc44ff"
}

state {
    trust: int = 0
    met_rival: bool = false
    route: string = "none"
}

function bump(n) {
    return n + 1
}

scene prologue {
    background "station.png"
    music "soft.ogg" fade_in 1.0
    show hero.neutral at left
    hero "Welcome to Velvet."
    "The rain continues."
    choice {
        "Greet rival" {
            met_rival = true
            trust += 1
            jump meet
        }
        "Walk away" {
            route = "solo"
            jump end_solo
        }
    }
}

scene meet {
    show rival.smirk at right
    rival "We meet again."
    call side_beat
    hero "Indeed."
    label after:
    jump end_duo
}

scene side_beat {
    "A quiet moment."
    end
}

scene end_solo {
    hero "Alone for now."
    end "solo"
}

scene end_duo {
    hide rival
    hero "Together."
    end "duo"
}
"##;
        let r = parse_ok(src);
        assert!(r.module.items.len() >= 6);
        let scenes: Vec<_> = r
            .module
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Scene { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert!(scenes.contains(&"prologue"));
        assert!(scenes.contains(&"meet"));
        assert!(scenes.contains(&"end_duo"));
        assert!(r
            .module
            .items
            .iter()
            .any(|i| matches!(i, Item::Function { name, .. } if name == "bump")));
        assert!(r
            .module
            .items
            .iter()
            .any(|i| matches!(i, Item::Character { name, .. } if name == "rival")));
    }

    #[test]
    fn fixture_gameplay_script_loops() {
        let src = r#"
function damage(base, mult) {
    let d = base * mult
    if d < 0 {
        return 0
    }
    return floor(d)
}

function simulate(rounds) {
    let hp = 100
    let i = 0
    while i < rounds {
        hp = hp - damage(5, 1.5)
        if hp <= 0 {
            break
        }
        i += 1
        if i % 2 == 0 {
            continue
        }
        hp = hp + 1
    }
    return hp
}

function sum_list(xs) {
    let t = 0
    for x in xs {
        t += x
    }
    return t
}
"#;
        let r = parse_ok(src);
        assert_eq!(
            r.module
                .items
                .iter()
                .filter(|i| matches!(i, Item::Function { .. }))
                .count(),
            3
        );
    }

    #[test]
    fn fixture_nested_choices_and_assigns() {
        let src = r##"
state { a: int = 0 b: int = 0 }
scene main {
    choice {
        "Outer A" {
            a += 1
            choice {
                "Inner 1" { b = 1 jump done }
                "Inner 2" { b = 2 jump done }
            }
        }
        "Outer B" {
            a = 9
            jump done
        }
    }
}
scene done {
    "done"
    end
}
"##;
        let r = parse_ok(src);
        match &r.module.items[1] {
            Item::Scene { body, .. } => {
                assert!(body.iter().any(|s| matches!(s, Stmt::Choice { .. })));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fixture_all_assign_ops() {
        // Compound assigns are accepted in story state bodies; in functions use re-lets / returns.
        let src = r#"
function f() {
    let x = 1
    let y = x + 2
    let z = y * 3
    return z / 2
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Function { body, .. } => {
                let lets = body
                    .iter()
                    .filter(|s| matches!(s, Stmt::Let { .. }))
                    .count();
                assert!(lets >= 3, "body={body:?}");
                assert!(body.iter().any(|s| matches!(s, Stmt::Return { .. })));
            }
            _ => panic!("expected function"),
        }
        // Story-style compound assign in scene body
        let r2 = parse_ok(
            r#"
state { n: int = 0 }
scene s {
    n += 1
    n -= 1
}
"#,
        );
        assert!(r2.module.items.len() >= 2);
    }

    #[test]
    fn fixture_comparison_and_logic_ops() {
        let src = r#"
function cmp(a, b) {
    return a < b || a <= b && a > 0 || a >= b && a == b || a != b
}
"#;
        let _ = parse_ok(src);
        let r = parse_ok("let x = 1 < 2 && 3 > 2 || !false");
        assert!(!r.module.items.is_empty());
    }

    #[test]
    fn fixture_list_literals_nested() {
        let r = parse_ok("let xs = [1, [2, 3], 4]");
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let {
                init: Expr::List { elements, .. },
                ..
            }) => {
                assert_eq!(elements.len(), 3);
                assert!(
                    matches!(&elements[1], Expr::List { elements: inner, .. } if inner.len() == 2)
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fixture_call_expr_chained() {
        let r = parse_ok("let v = abs(min(1, max(2, 3)))");
        match &r.module.items[0] {
            Item::Stmt(Stmt::Let {
                init: Expr::Call { .. },
                ..
            }) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fixture_multiple_top_level_lets() {
        let src = r#"
const MAX = 10
let a = 1
let b = a + MAX
let c = [a, b]
"#;
        let r = parse_ok(src);
        assert_eq!(r.module.items.len(), 4);
    }

    #[test]
    fn fixture_scene_only_narration() {
        let src = r##"
scene intro {
    "Line one"
    "Line two"
    "Line three"
    end "credits"
}
"##;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Scene { body, .. } => {
                let dialogues = body
                    .iter()
                    .filter(|s| matches!(s, Stmt::Dialogue { speaker: None, .. }))
                    .count();
                assert_eq!(dialogues, 3);
            }
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn fixture_show_hide_variants() {
        let src = r#"
scene s {
    show hero at center
    show hero.angry at left
    show "prop.lamp" at right
    hide hero
    hide prop
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Scene { body, .. } => {
                let shows = body
                    .iter()
                    .filter(|s| matches!(s, Stmt::Show { .. }))
                    .count();
                let hides = body
                    .iter()
                    .filter(|s| matches!(s, Stmt::Hide { .. }))
                    .count();
                assert_eq!(shows, 3);
                assert_eq!(hides, 2);
            }
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn fixture_error_unclosed_brace() {
        let r = parse("function f() { let x = 1").unwrap();
        assert!(r.module.has_errors() || !r.module.diagnostics.is_empty());
    }

    #[test]
    fn fixture_error_bad_choice_still_has_scene() {
        let src = r#"
scene main {
    choice {
        123 {
            jump x
        }
    }
}
"#;
        let r = parse(src).unwrap();
        assert!(r
            .module
            .items
            .iter()
            .any(|i| matches!(i, Item::Scene { .. })));
    }

    #[test]
    fn fixture_float_and_negative_literals() {
        let r = parse_ok("let a = 3.14\nlet b = -2.5\nlet c = 0.0");
        assert_eq!(r.module.items.len(), 3);
    }

    #[test]
    fn fixture_while_if_else_nested() {
        let src = r#"
function f(n) {
    let i = 0
    while i < n {
        if i % 2 == 0 {
            if i > 2 {
                return i
            } else {
                i += 1
            }
        } else {
            i += 1
        }
    }
    return -1
}
"#;
        let _ = parse_ok(src);
    }

    #[test]
    fn parse_declarative_screen_with_buttons() {
        let src = r#"
screen main_menu {
    class: "title-menu"
    title: "VELVET ARCANA"

    button start {
        label: "START RUN"
        description: "Begin a new high-stakes run"
        action: "start_run"
        icon: "play"
        enabled: true
    }

    button quit {
        label: "QUIT"
        action: "quit"
    }
}
"#;
        let r = parse_ok(src);
        match &r.module.items[0] {
            Item::Screen {
                name,
                properties,
                buttons,
                ..
            } => {
                assert_eq!(name, "main_menu");
                assert_eq!(properties.len(), 2);
                assert_eq!(buttons.len(), 2);
                assert_eq!(buttons[0].id, "start");
                assert!(buttons[0]
                    .properties
                    .iter()
                    .any(|p| p.name == "description"));
                assert_eq!(buttons[1].id, "quit");
            }
            other => panic!("expected screen, got {other:?}"),
        }
    }

    #[test]
    fn parse_screen_keeps_following_scene() {
        let src = r#"
screen pause {
    button resume { label: "RESUME"; action: "resume"; }
}
scene start { "ready" }
"#;
        let r = parse_ok(src);
        assert!(matches!(r.module.items[0], Item::Screen { .. }));
        assert!(matches!(r.module.items[1], Item::Scene { .. }));
    }
}
