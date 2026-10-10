//! Recursive descent parser.

/// Fold a trailing bare `repeat [n]` (empty steps) into a loop over the
/// preceding steps of its block. Bare repeats elsewhere are an error.
fn fold_trailing_repeat(steps: Vec<AtlStep>) -> Result<Vec<AtlStep>, String> {
    let mut out = steps;
    for (i, step) in out.iter().enumerate() {
        if matches!(step, AtlStep::Repeat { steps, .. } if steps.is_empty()) {
            if i + 1 != out.len() {
                return Err("repeat must end its block".into());
            }
            let body: Vec<AtlStep> = out.drain(..i).collect();
            if body.is_empty() {
                return Err("repeat needs steps to loop".into());
            }
            let (count, loc) = match out.pop() {
                Some(AtlStep::Repeat { count, loc, .. }) => (count, loc),
                _ => unreachable!(),
            };
            out.push(AtlStep::Repeat {
                count,
                steps: body,
                loc,
            });
            break;
        }
    }
    Ok(out)
}

/// Warper names accepted after an ATL duration.
fn is_atl_warper(name: &str) -> bool {
    matches!(
        name,
        "linear"
            | "ease"
            | "ease_in"
            | "ease_out"
            | "ease_in_out"
            | "quad_in"
            | "quad_out"
            | "quad_in_out"
            | "cubic_in"
            | "cubic_out"
            | "cubic_in_out"
            | "sine_in"
            | "sine_out"
            | "sine_in_out"
            | "back_in"
            | "back_out"
            | "back_in_out"
            | "elastic_in"
            | "elastic_out"
            | "elastic_in_out"
            | "bounce_in"
            | "bounce_out"
            | "bounce_in_out"
    )
}

use thiserror::Error;
use velvet_script_ast::{
    AtlStep, AtlValue, BinOp, ChoiceArm, Diagnostic, Expr, Item, Module, Param, ScreenActionKind,
    ScreenButton, ScreenNode, ScreenParam, ScreenProperty, SourceLoc, StateBinding, Stmt, UnaryOp,
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
            "import"
                | "export"
                | "function"
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
                | "unlock"
                | "movie"
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
                    "import"
                        | "export"
                        | "function"
                        | "fn"
                        | "character"
                        | "state"
                        | "scene"
                        | "screen"
                        | "transform"
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
        if self.check(|t| matches!(t, Token::Ident(s) if s == "import")) {
            return self.parse_import();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "export")) {
            return self.parse_export();
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "function" || s == "fn")) {
            return self.parse_function(false);
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
        if self.check(|t| matches!(t, Token::Ident(s) if s == "transform")) {
            return self.parse_transform();
        }
        Ok(Item::Stmt(self.parse_stmt()?))
    }

    fn parse_import(&mut self) -> Result<Item, String> {
        let loc = self.peek_loc();
        self.advance(); // import
        let token = self.expect(
            |token| matches!(token, Token::String(_)),
            "expected quoted import path",
        )?;
        let path = match token.token {
            Token::String(path) => path,
            _ => unreachable!(),
        };
        let alias = if self.check(|token| matches!(token, Token::Ident(name) if name == "as")) {
            self.advance();
            Some(self.expect_ident()?.0)
        } else {
            None
        };
        let _ = self.match_token(|token| matches!(token, Token::Semi));
        Ok(Item::Import { path, alias, loc })
    }

    fn parse_export(&mut self) -> Result<Item, String> {
        self.advance(); // export
        if !self.check(
            |token| matches!(token, Token::Ident(name) if name == "function" || name == "fn"),
        ) {
            return Err("expected `function` after `export`".into());
        }
        self.parse_function(true)
    }

    fn parse_function(&mut self, exported: bool) -> Result<Item, String> {
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
            exported,
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
        // Optional params: `screen inventory(items, gold = 0)`.
        let mut params = Vec::new();
        if self.check(|t| matches!(t, Token::LParen)) {
            self.advance();
            loop {
                if self.check(|t| matches!(t, Token::RParen)) {
                    self.advance();
                    break;
                }
                let (pname, ploc) = self.expect_ident()?;
                let mut default = None;
                if self.match_token(|t| matches!(t, Token::Assign)) {
                    default = Some(self.parse_expression()?);
                }
                params.push(ScreenParam {
                    name: pname,
                    default,
                    loc: ploc,
                });
                if self.check(|t| matches!(t, Token::RParen)) {
                    self.advance();
                    break;
                }
                self.expect(|t| matches!(t, Token::Comma), "expected ',' between params")?;
            }
        }
        self.expect(
            |t| matches!(t, Token::LBrace),
            "expected '{' after screen name",
        )?;

        let mut properties = Vec::new();
        let mut buttons = Vec::new();
        let mut children = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            if self.check(|t| matches!(t, Token::Ident(s) if s == "button")) {
                buttons.push(self.parse_screen_button()?);
            } else if let Some(node) = self.try_parse_screen_child()? {
                children.push(node);
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
            params,
            children,
            loc: start,
        })
    }

    /// Try `text|textbutton|use|<widget>`, or `button`. Returns None when the
    /// next line is a `name: value` property (or anything else).
    fn try_parse_screen_child(&mut self) -> Result<Option<ScreenNode>, String> {
        let Some(Token::Ident(kind)) = self.peek_token().cloned() else {
            return Ok(None);
        };
        // `name: value` is a property, not a widget.
        let is_prop = matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.token),
            Some(Token::Colon)
        );
        if is_prop {
            return Ok(None);
        }
        const WIDGETS: &[&str] = &[
            "text",
            "textbutton",
            "button",
            "image",
            "bar",
            "vbox",
            "hbox",
            "fixed",
            "use",
            "timer",
            "key",
            "input",
        ];
        // Top-level `button id { ... }` is still handled by parse_screen(),
        // while nested buttons are regular widget nodes so layouts can style
        // and position them inside hbox/vbox/fixed containers.
        if !WIDGETS.contains(&kind.as_str()) {
            return Ok(None);
        }
        let start = self.peek_loc();
        self.advance();
        match kind.as_str() {
            "text" => {
                let text = self.parse_expression()?;
                Ok(Some(ScreenNode::Text { text, loc: start }))
            }
            "textbutton" => {
                let label = self.parse_expression()?;
                self.expect(
                    |t| matches!(t, Token::Ident(s) if s == "action"),
                    "expected 'action' after textbutton label",
                )?;
                let action = self.parse_expression()?;
                Ok(Some(ScreenNode::TextButton {
                    label,
                    action,
                    loc: start,
                }))
            }
            "use" => {
                let (screen, _) = self.expect_ident()?;
                let args = self.parse_paren_args()?;
                Ok(Some(ScreenNode::Use {
                    screen,
                    args,
                    loc: start,
                }))
            }
            _ => {
                // Generic widget block: `{ prop: value, … + nested children }`.
                let (mut properties, mut children) = (Vec::new(), Vec::new());
                if self.check(|t| matches!(t, Token::LBrace)) {
                    self.advance();
                    while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
                        if let Some(node) = self.try_parse_screen_child()? {
                            children.push(node);
                        } else {
                            properties.push(self.parse_screen_property()?);
                        }
                        let _ = self.match_token(|t| matches!(t, Token::Comma | Token::Semi));
                    }
                    self.expect(|t| matches!(t, Token::RBrace), "expected '}' after widget")?;
                }
                Ok(Some(ScreenNode::Widget {
                    kind,
                    properties,
                    children,
                    loc: start,
                }))
            }
        }
    }

    /// Optional `(expr, …)` argument list (empty vec when absent).
    fn parse_paren_args(&mut self) -> Result<Vec<Expr>, String> {
        if !self.check(|t| matches!(t, Token::LParen)) {
            return Ok(Vec::new());
        }
        self.advance();
        let mut args = Vec::new();
        loop {
            if self.check(|t| matches!(t, Token::RParen)) {
                self.advance();
                break;
            }
            args.push(self.parse_expression()?);
            if self.check(|t| matches!(t, Token::RParen)) {
                self.advance();
                break;
            }
            self.expect(|t| matches!(t, Token::Comma), "expected ',' between args")?;
        }
        Ok(args)
    }

    fn parse_transform(&mut self) -> Result<Item, String> {
        let start = self.peek_loc();
        self.advance(); // transform
        let (name, _) = self.expect_ident()?;
        self.expect(
            |t| matches!(t, Token::LBrace),
            "expected '{' after transform name",
        )?;
        let steps = self.parse_atl_steps()?;
        self.expect(
            |t| matches!(t, Token::RBrace),
            "expected '}' after transform",
        )?;
        let steps = fold_trailing_repeat(steps)?;
        Ok(Item::Transform {
            name,
            steps,
            loc: start,
        })
    }

    fn parse_atl_steps(&mut self) -> Result<Vec<AtlStep>, String> {
        let mut steps = Vec::new();
        while !self.is_at_end() && !self.check(|t| matches!(t, Token::RBrace)) {
            steps.push(self.parse_atl_step()?);
            let _ = self.match_token(|t| matches!(t, Token::Comma | Token::Semi));
        }
        Ok(steps)
    }

    fn parse_atl_step(&mut self) -> Result<AtlStep, String> {
        let start = self.peek_loc();
        // Block forms first.
        if self.check(|t| matches!(t, Token::Ident(s) if s == "parallel")) {
            self.advance();
            self.expect(
                |t| matches!(t, Token::LBrace),
                "expected '{' after parallel",
            )?;
            let steps = self.parse_atl_steps()?;
            self.expect(
                |t| matches!(t, Token::RBrace),
                "expected '}' after parallel",
            )?;
            return Ok(AtlStep::Parallel {
                steps: fold_trailing_repeat(steps)?,
                loc: start,
            });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "choice")) {
            self.advance();
            self.expect(|t| matches!(t, Token::LBrace), "expected '{' after choice")?;
            let steps = self.parse_atl_steps()?;
            self.expect(|t| matches!(t, Token::RBrace), "expected '}' after choice")?;
            return Ok(AtlStep::Choice {
                steps: fold_trailing_repeat(steps)?,
                loc: start,
            });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "on")) {
            self.advance();
            let mut events = Vec::new();
            loop {
                let (ev, _) = self.expect_ident()?;
                if ev != "show" && ev != "hide" && ev != "replace" {
                    return Err(format!(
                        "expected show, hide or replace after `on`, got `{ev}`"
                    ));
                }
                events.push(ev);
                if self.check(|t| matches!(t, Token::Comma)) {
                    self.advance();
                    continue;
                }
                break;
            }
            self.expect(|t| matches!(t, Token::LBrace), "expected '{' after on …")?;
            let steps = self.parse_atl_steps()?;
            self.expect(|t| matches!(t, Token::RBrace), "expected '}' after on …")?;
            return Ok(AtlStep::On {
                events,
                steps: fold_trailing_repeat(steps)?,
                loc: start,
            });
        }
        if self.check(|t| matches!(t, Token::Ident(s) if s == "repeat")) {
            self.advance();
            let mut count = None;
            if let Some(Token::Int(n)) = self.peek_token().cloned() {
                self.advance();
                count = Some(n.max(0) as u32);
            }
            // Bare `repeat [n]` loops the enclosing block (folded by the
            // caller); `repeat [n] { … }` loops explicit steps.
            if self.check(|t| matches!(t, Token::LBrace)) {
                self.advance();
                let steps = self.parse_atl_steps()?;
                self.expect(|t| matches!(t, Token::RBrace), "expected '}' after repeat")?;
                return Ok(AtlStep::Repeat {
                    count,
                    steps,
                    loc: start,
                });
            }
            return Ok(AtlStep::Repeat {
                count,
                steps: Vec::new(),
                loc: start,
            });
        }
        // Channel step: `prop value [prop value …] [duration] [warper]`.
        // A new `prop` starts only when an ident (not a warper) is followed
        // by a value token; otherwise the step ends here.
        let mut props = Vec::new();
        loop {
            let (prop, _) = self.expect_ident()?;
            props.push((prop, self.parse_atl_value()?));
            if !self.atl_channel_follows() {
                break;
            }
        }
        let (duration, warper) = self.parse_atl_timing()?;
        Ok(AtlStep::Set {
            props,
            duration,
            warper,
            loc: start,
        })
    }

    /// One ATL channel value: number (signed), string, or keyword ident.
    fn parse_atl_value(&mut self) -> Result<AtlValue, String> {
        match self.peek_token().cloned() {
            Some(Token::String(s)) => {
                self.advance();
                Ok(AtlValue::Keyword(s))
            }
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(AtlValue::Keyword(s))
            }
            Some(Token::Int(n)) => {
                self.advance();
                Ok(AtlValue::Number(n as f64))
            }
            Some(Token::Float(f)) => {
                self.advance();
                Ok(AtlValue::Number(f))
            }
            Some(Token::Minus) => {
                self.advance();
                match self.peek_token().cloned() {
                    Some(Token::Int(n)) => {
                        self.advance();
                        Ok(AtlValue::Number(-(n as f64)))
                    }
                    Some(Token::Float(f)) => {
                        self.advance();
                        Ok(AtlValue::Number(-f))
                    }
                    _ => Err("expected number after '-'".into()),
                }
            }
            other => Err(format!("expected channel value, got {other:?}")),
        }
    }

    /// True when another `prop value` pair follows on this step.
    fn atl_channel_follows(&self) -> bool {
        if !matches!(self.peek_token(), Some(Token::Ident(w)) if !is_atl_warper(w)) {
            return false;
        }
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.token),
            Some(
                Token::String(_) | Token::Ident(_) | Token::Int(_) | Token::Float(_) | Token::Minus
            )
        )
    }

    /// Optional `[duration] [warper]` tail of a channel step.
    fn parse_atl_timing(&mut self) -> Result<(f64, String), String> {
        let mut duration = 0.0;
        match self.peek_token().cloned() {
            Some(Token::Int(n)) => {
                self.advance();
                duration = n as f64;
            }
            Some(Token::Float(f)) => {
                self.advance();
                duration = f;
            }
            _ => {}
        }
        let mut warper = "linear".to_string();
        if let Some(Token::Ident(w)) = self.peek_token().cloned() {
            if is_atl_warper(&w) {
                self.advance();
                warper = w;
            }
        }
        Ok((duration, warper))
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
        if self.check(|t| matches!(t, Token::Ident(s) if s == "screen"))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.token),
                Some(Token::Ident(_))
            )
        {
            let loc = self.peek_loc();
            self.advance();
            let (word, _) = self.expect_ident()?;
            // `screen show|hide|call <id> [(args)] [with …] [into var]`.
            if let Some(action) = ScreenActionKind::parse(&word) {
                let (id, _) = self.expect_ident()?;
                let args = self.parse_paren_args()?;
                let with = self.parse_with_words()?;
                let result = self.parse_into_var()?;
                return Ok(Stmt::ScreenAction {
                    action,
                    id,
                    args,
                    with,
                    result,
                    loc,
                });
            }
            return Ok(Stmt::Screen { id: word, loc });
        }
        // `unlock <catalog> <id>` — guarded so that `unlock = 5` or
        // `unlock(x)` still parse as expressions; unknown catalogs get a
        // clear diagnostic at lowering (`file:line` + valid options).
        if self.check(|t| matches!(t, Token::Ident(s) if s == "unlock"))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.token),
                Some(Token::Ident(_))
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.token),
                Some(Token::Ident(_)) | Some(Token::String(_))
            )
        {
            let loc = self.peek_loc();
            self.advance();
            let (kind, _) = self.expect_ident()?;
            let id = if self.check(|t| matches!(t, Token::String(_))) {
                let t = self.advance().unwrap().clone();
                match t.token {
                    Token::String(s) => s,
                    _ => unreachable!(),
                }
            } else {
                let (name, _) = self.expect_ident()?;
                name
            };
            return Ok(Stmt::Unlock { kind, id, loc });
        }
        // `movie "dir" [loop]` — guarded so `movie = 5` / `movie(x)`
        // still parse as expressions.
        if self.check(|t| matches!(t, Token::Ident(s) if s == "movie"))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.token),
                Some(Token::String(_))
            )
        {
            let loc = self.peek_loc();
            self.advance();
            let t = self.advance().unwrap().clone();
            let path = match t.token {
                Token::String(s) => s,
                _ => unreachable!(),
            };
            let loop_clip = if self.check(|t| matches!(t, Token::Ident(s) if s == "loop")) {
                self.advance();
                true
            } else {
                false
            };
            return Ok(Stmt::Movie {
                path,
                loop_clip,
                loc,
            });
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

    /// Optional `with <name> [num] [num]` suffix (mini-ATL or transform).
    fn parse_with_words(&mut self) -> Result<Option<String>, String> {
        if !self.check(|t| matches!(t, Token::Ident(s) if s == "with")) {
            return Ok(None);
        }
        self.advance();
        let (kind, _) = self.expect_ident()?;
        let mut parts = vec![kind];
        for _ in 0..2 {
            let num = match self.peek_token() {
                Some(Token::Int(n)) => Some(n.to_string()),
                Some(Token::Float(f)) => Some(f.to_string()),
                _ => None,
            };
            match num {
                Some(n) => {
                    self.advance();
                    parts.push(n);
                }
                None => break,
            }
        }
        Ok(Some(parts.join(" ")))
    }

    /// Optional `into <var>` suffix for `screen call` result binding.
    fn parse_into_var(&mut self) -> Result<Option<String>, String> {
        if !self.check(|t| matches!(t, Token::Ident(s) if s == "into")) {
            return Ok(None);
        }
        self.advance();
        let (name, _) = self.expect_ident()?;
        Ok(Some(name))
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
        // `hide screen <id>` sugar (a lone `hide screen` stays a sprite show).
        if target == "screen"
            && matches!(
                self.tokens.get(self.pos).map(|t| &t.token),
                Some(Token::Ident(_))
            )
        {
            let (id, _) = self.expect_ident()?;
            let args = self.parse_paren_args()?;
            let with = self.parse_with_words()?;
            let result = self.parse_into_var()?;
            return Ok(Stmt::ScreenAction {
                action: ScreenActionKind::Hide,
                id,
                args,
                with,
                result,
                loc,
            });
        }
        // Optional motion suffix: `hide luna with fade`.
        let with = self.parse_with_words()?;
        Ok(Stmt::Hide { target, with, loc })
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
        // `show screen <id> [(args)]` sugar (a lone `show screen at …`
        // keeps meaning a sprite literally named `screen`).
        if target == "screen"
            && matches!(
                self.tokens.get(self.pos).map(|t| &t.token),
                Some(Token::Ident(s)) if s != "at" && s != "with"
            )
        {
            let (id, _) = self.expect_ident()?;
            let args = self.parse_paren_args()?;
            let with = self.parse_with_words()?;
            let result = self.parse_into_var()?;
            return Ok(Stmt::ScreenAction {
                action: ScreenActionKind::Show,
                id,
                args,
                with,
                result,
                loc,
            });
        }
        let mut at = None;
        if self.check(|t| matches!(t, Token::Ident(s) if s == "at")) {
            self.advance();
            let (place, _) = self.expect_ident()?;
            at = Some(place);
        }
        // Optional mini-ATL suffix: `with <move|zoom|shake|fade> [secs] [amount]`.
        let with = self.parse_with_words()?;
        Ok(Stmt::Show {
            target,
            at,
            with,
            loc,
        })
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
        if self.check(|t| {
            matches!(
                t,
                Token::Assign
                    | Token::PlusAssign
                    | Token::MinusAssign
                    | Token::StarAssign
                    | Token::SlashAssign
            )
        }) {
            let op_tok = self.advance().unwrap().clone();
            let op = match op_tok.token {
                Token::Assign => BinOp::Assign,
                Token::PlusAssign => BinOp::AddAssign,
                Token::MinusAssign => BinOp::SubAssign,
                Token::StarAssign => BinOp::MulAssign,
                Token::SlashAssign => BinOp::DivAssign,
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
                if name == "null" {
                    Ok(Expr::Null { loc })
                } else {
                    Ok(Expr::Ident { name, loc })
                }
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
            Token::LBrace => {
                self.advance();
                let mut entries = Vec::new();
                while !self.check(|token| matches!(token, Token::RBrace)) {
                    let key = match self.advance().cloned() {
                        Some(LexedToken {
                            token: Token::String(key) | Token::Ident(key),
                            ..
                        }) => key,
                        _ => return Err("expected string or identifier map key".into()),
                    };
                    self.expect(|token| matches!(token, Token::Colon), "expected ':'")?;
                    let value = self.parse_expression()?;
                    entries.push((key, value));
                    if !self.match_token(|token| matches!(token, Token::Comma)) {
                        break;
                    }
                }
                self.expect(|token| matches!(token, Token::RBrace), "expected '}'")?;
                Ok(Expr::Map { entries, loc })
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
    fn parse_imports_with_and_without_aliases() {
        let parsed = parse_ok(
            "import \"shared.vel\"\nimport \"combat.vel\" as combat\nfunction run() { return combat.fire() }",
        );
        assert!(matches!(
            &parsed.module.items[0],
            Item::Import { path, alias: None, .. } if path == "shared.vel"
        ));
        assert!(matches!(
            &parsed.module.items[1],
            Item::Import { path, alias: Some(alias), .. }
                if path == "combat.vel" && alias == "combat"
        ));
    }

    #[test]
    fn parse_exported_function() {
        let parsed = parse_ok("export function public_value() { return 17 }");
        assert!(matches!(
            &parsed.module.items[0],
            Item::Function {
                exported: true,
                name,
                ..
            } if name == "public_value"
        ));
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
        assert!(r.module.has_errors(), "expected recovery diagnostics");
        assert!(r
            .module
            .diagnostics
            .iter()
            .all(|d| d.loc.line > 0 && d.loc.column > 0));
        let r = parse("1 +").unwrap();
        assert!(
            r.module.has_errors(),
            "expected diagnostics, got {:?}",
            r.module.diagnostics
        );
        assert!(
            r.module
                .diagnostics
                .iter()
                .any(|d| d.message.contains("unexpected") || d.message.contains("end")),
            "diagnostics must explain the incomplete expression: {:?}",
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
    fn parse_transform_on_handlers() {
        let r = parse_ok(
            "transform pop {\n  on show {\n    scale 1.2 0.2\n  }\n  on hide, replace {\n    opacity 0.0 0.3\n  }\n  y -4 1.0\n}",
        );
        match &r.module.items[0] {
            Item::Transform { name, steps, .. } => {
                assert_eq!(name, "pop");
                assert_eq!(steps.len(), 3);
                match &steps[0] {
                    AtlStep::On { events, steps, .. } => {
                        assert_eq!(events, &vec!["show".to_string()]);
                        assert_eq!(steps.len(), 1);
                    }
                    other => panic!("{other:?}"),
                }
                match &steps[1] {
                    AtlStep::On { events, .. } => {
                        assert_eq!(events, &vec!["hide".to_string(), "replace".to_string()])
                    }
                    other => panic!("{other:?}"),
                }
                assert!(matches!(steps[2], AtlStep::Set { .. }));
            }
            other => panic!("{other:?}"),
        }
        // bad event rejected
        let bad = parse_file("transform t {\n on explode {\n x 1\n }\n}\n", None).unwrap();
        assert!(bad.module.has_errors());
    }

    #[test]
    fn parse_screen_action_with_and_into() {
        let r = parse_ok("scene s { screen show help (\"x\") with fade }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::ScreenAction {
                    action,
                    id,
                    args,
                    with,
                    result,
                    ..
                } => {
                    assert_eq!(*action, ScreenActionKind::Show);
                    assert_eq!(id, "help");
                    assert_eq!(args.len(), 1);
                    assert_eq!(with.as_deref(), Some("fade"));
                    assert!(result.is_none());
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        let r = parse_ok("scene s { screen call help into pick }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::ScreenAction { result, .. } => {
                    assert_eq!(result.as_deref(), Some("pick"))
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // sugar forms take `with` too
        let r = parse_ok("scene s { show screen help with shake }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::ScreenAction { with, .. } => {
                    assert_eq!(with.as_deref(), Some("shake"))
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_hide_with_motion() {
        let r = parse_ok("scene s { hide luna with fade hide kai with shake 0.4 10 hide ren }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => {
                assert_eq!(body.len(), 3);
                match &body[0] {
                    Stmt::Hide { target, with, .. } => {
                        assert_eq!(target, "luna");
                        assert_eq!(with.as_deref(), Some("fade"));
                    }
                    other => panic!("{other:?}"),
                }
                match &body[1] {
                    Stmt::Hide { with, .. } => assert_eq!(with.as_deref(), Some("shake 0.4 10")),
                    other => panic!("{other:?}"),
                }
                match &body[2] {
                    Stmt::Hide { with: None, .. } => {}
                    other => panic!("{other:?}"),
                }
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
        assert!(
            r.module.has_errors(),
            "broken declaration must emit a diagnostic"
        );
        // The valid function after the error must survive recovery.
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
            funcs.contains(&"good"),
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
    fn parse_show_with_mini_atl() {
        let r = parse_ok("scene s { show luna.happy at left with shake 0.4 10 }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Show {
                    target, at, with, ..
                } => {
                    assert_eq!(target, "luna.happy");
                    assert_eq!(at.as_deref(), Some("left"));
                    assert_eq!(with.as_deref(), Some("shake 0.4 10"));
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // bare `with move` without numbers
        let r = parse_ok("scene s { show luna with move }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Show { with, at, .. } => {
                    assert_eq!(with.as_deref(), Some("move"));
                    assert!(at.is_none());
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_screen_actions_and_sugar() {
        let r = parse_ok("scene s { screen show help (1, \"x\") }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::ScreenAction {
                    action, id, args, ..
                } => {
                    assert_eq!(*action, ScreenActionKind::Show);
                    assert_eq!(id, "help");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // `show screen` / `hide screen` sugar
        let r = parse_ok("scene s { show screen help }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::ScreenAction { action, id, .. } => {
                    assert_eq!(*action, ScreenActionKind::Show);
                    assert_eq!(id, "help");
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        let r = parse_ok("scene s { hide screen help }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::ScreenAction { action, .. } => assert_eq!(*action, ScreenActionKind::Hide),
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // a sprite literally named `screen` still works with `at`
        let r = parse_ok("scene s { show screen at left }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Show { target, at, .. } => {
                    assert_eq!(target, "screen");
                    assert_eq!(at.as_deref(), Some("left"));
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
    }

    #[test]
    fn parse_nested_button_widget() {
        let r = parse_ok(
            "screen phone {\n  hbox {\n    button { label: \"Chats\" class: \"app-tile\" action: Return(\"chats\") }\n  }\n}\nscene main { screen call phone }",
        );
        match &r.module.items[0] {
            Item::Screen { children, .. } => match &children[0] {
                ScreenNode::Widget { kind, children, .. } => {
                    assert_eq!(kind, "hbox");
                    assert_eq!(children.len(), 1);
                    match &children[0] {
                        ScreenNode::Widget {
                            kind, properties, ..
                        } => {
                            assert_eq!(kind, "button");
                            assert!(properties.iter().any(|p| p.name == "class"));
                            assert!(properties.iter().any(|p| p.name == "action"));
                        }
                        other => panic!("expected nested button widget, got {other:?}"),
                    }
                }
                other => panic!("expected hbox widget, got {other:?}"),
            },
            other => panic!("expected screen, got {other:?}"),
        }
    }

    #[test]
    fn parse_game_screen_and_transform() {
        let r = parse_ok(
            "screen inv(items, gold = 0) {\n  title: \"Bag\"\n  text \"Loot:\"\n  textbutton done action Return(\"ok\")\n  vbox {\n    text items\n    use row (items)\n  }\n  timer { delay: 3.0 action: Return(\"timeout\") }\n}\ntransform bob {\n  y -8 1.2 ease_in_out, y 0 1.2 ease_in_out, repeat\n}",
        );
        match &r.module.items[0] {
            Item::Screen {
                name,
                params,
                children,
                properties,
                ..
            } => {
                assert_eq!(name, "inv");
                assert_eq!(params.len(), 2);
                assert!(params[1].default.is_some());
                assert_eq!(properties.len(), 1);
                assert_eq!(children.len(), 4);
                assert!(matches!(children[0], ScreenNode::Text { .. }));
                assert!(matches!(children[1], ScreenNode::TextButton { .. }));
                match &children[2] {
                    ScreenNode::Widget { kind, children, .. } => {
                        assert_eq!(kind, "vbox");
                        assert_eq!(children.len(), 2);
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
        match &r.module.items[1] {
            Item::Transform { name, steps, .. } => {
                assert_eq!(name, "bob");
                // trailing bare `repeat` folds the two sets into one loop
                assert_eq!(steps.len(), 1);
                match &steps[0] {
                    AtlStep::Repeat { count, steps, .. } => {
                        assert_eq!(*count, None);
                        assert_eq!(steps.len(), 2);
                        assert!(matches!(steps[0], AtlStep::Set { .. }));
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_movie_statement() {
        let r = parse_ok("scene s { movie \"assets/video/opening\" loop }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Movie {
                    path, loop_clip, ..
                } => {
                    assert_eq!(path, "assets/video/opening");
                    assert!(*loop_clip);
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        let r = parse_ok("scene s { movie \"clip\" }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Movie { loop_clip, .. } => assert!(!loop_clip),
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // `movie = 5` still parses as an assignment expression
        let r = parse_ok("function f() { movie = 5 return movie }");
        assert!(matches!(&r.module.items[0], Item::Function { .. }));
    }

    #[test]
    fn parse_unlock_catalog_statement() {
        let r = parse_ok("scene s { unlock gallery cg_rooftop }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Unlock { kind, id, .. } => {
                    assert_eq!(kind, "gallery");
                    assert_eq!(id, "cg_rooftop");
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // string ids also work
        let r = parse_ok("scene s { unlock music \"night theme\" }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Unlock { kind, id, .. } => {
                    assert_eq!(kind, "music");
                    assert_eq!(id, "night theme");
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected scene"),
        }
        // `unlock = 5` still parses as an assignment expression, not a statement
        let r = parse_ok("function f() { unlock = 5 return unlock }");
        assert!(matches!(&r.module.items[0], Item::Function { .. }));
    }

    #[test]
    fn parse_screen_preset_statement() {
        let r = parse_ok("scene s { screen nvl_page }");
        match &r.module.items[0] {
            Item::Scene { body, .. } => match &body[0] {
                Stmt::Screen { id, .. } => assert_eq!(id, "nvl_page"),
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
        assert!(r.module.has_errors());
        assert!(
            r.module
                .diagnostics
                .iter()
                .all(|d| !d.message.trim().is_empty() && d.loc.line > 0 && d.loc.column > 0),
            "diagnostics must be actionable and located: {:?}",
            r.module.diagnostics
        );
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
