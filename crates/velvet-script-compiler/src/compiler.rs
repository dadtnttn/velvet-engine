//! AST to bytecode.

use std::collections::HashMap;

use thiserror::Error;
use velvet_script_ast::{BinOp, Diagnostic, Expr, Item, Module, SourceLoc, Stmt, UnaryOp};
use velvet_script_bytecode::{
    fnv1a64, lookup_math_constant, lookup_native, BytecodeModule, Chunk, Constant, ModuleMetadata,
    Op,
};
use velvet_script_parser::{parse_file, ParseError};

/// Compile error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Parse failed fatally.
    #[error("parse error: {0}")]
    Parse(String),
    /// Semantic / codegen error with location.
    #[error("{loc}: {message}")]
    Codegen {
        /// Message.
        message: String,
        /// Location display.
        loc: String,
    },
    /// Multiple codegen errors (first is shown in Display; all available via `errors()`).
    #[error("{count} compile error(s); first: {first}")]
    Many {
        /// Number of errors.
        count: usize,
        /// First error summary.
        first: String,
        /// All diagnostics.
        diagnostics: Vec<Diagnostic>,
    },
}

impl CompileError {
    /// All diagnostics when multiple errors were collected.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Many { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }
}

impl From<ParseError> for CompileError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value.to_string())
    }
}

/// Compile result.
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Bytecode module.
    pub module: BytecodeModule,
    /// Diagnostics (may include parse recovery errors and soft warnings).
    pub diagnostics: Vec<Diagnostic>,
}

/// Compile from source string.
pub fn compile_source(source: &str, file: Option<&str>) -> Result<CompileResult, CompileError> {
    let parsed = parse_file(source, file)?;
    let mut result = compile(&parsed.module)?;
    // Attach source hash metadata.
    result.module.metadata.source_hash = Some(fnv1a64(source.as_bytes()));
    if let Some(f) = file {
        result.module.metadata.source_path = Some(f.to_string());
        result.module.file = Some(f.to_string());
    }
    Ok(result)
}

/// Compile an AST module.
pub fn compile(ast: &Module) -> Result<CompileResult, CompileError> {
    let mut cx = Compiler {
        module: BytecodeModule {
            file: ast.file.clone(),
            metadata: ModuleMetadata::current(),
            ..Default::default()
        },
        diagnostics: ast.diagnostics.clone(),
        globals: HashMap::new(),
        errors: Vec::new(),
    };
    if let Some(ref f) = ast.file {
        cx.module.metadata.source_path = Some(f.clone());
    }

    // Predeclare functions and stdlib names as globals.
    for item in &ast.items {
        if let Item::Function { name, .. } = item {
            cx.declare_global(name);
        }
    }
    for native in velvet_script_bytecode::NativeId::all() {
        cx.declare_global(native.name());
    }
    for item in &ast.items {
        if let Item::State { bindings, .. } = item {
            for b in bindings {
                cx.declare_global(&b.name);
            }
        }
        if let Item::Character { name, .. } = item {
            cx.declare_global(name);
        }
    }

    // Compile functions first.
    for item in &ast.items {
        if let Item::Function {
            name,
            params,
            body,
            loc,
            ..
        } = item
        {
            // Soft-fail: keep compiling other functions after errors.
            if let Err(e) = cx.compile_function(name, params, body, loc) {
                cx.errors
                    .push(Diagnostic::error(e.to_string(), loc.clone()));
            }
        }
    }

    // Main script chunk: state, characters as globals, top-level stmts, scenes as callable labels.
    let mut main = Chunk::new("<script>");
    let mut main_cx = FnCompiler {
        chunk: &mut main,
        locals: Vec::new(),
        scope_depth: 0,
        globals: &mut cx.globals,
        module_globals: &mut cx.module.globals,
        errors: &mut cx.errors,
        loops: Vec::new(),
    };

    for item in &ast.items {
        match item {
            Item::State { bindings, .. } => {
                for b in bindings {
                    if let Err(e) = main_cx.compile_expr(&b.init) {
                        main_cx
                            .errors
                            .push(Diagnostic::error(e.to_string(), b.loc.clone()));
                        main_cx.chunk.emit_op(Op::Null);
                    }
                    let idx = main_cx.global_index(&b.name);
                    main_cx.chunk.emit_op(Op::DefineGlobal);
                    main_cx.chunk.emit_u16(idx);
                }
            }
            Item::Character { name, fields, loc } => {
                // Represent character as list of field values for now + name string constant.
                main_cx.chunk.map_source(loc.line, loc.column);
                main_cx.chunk.emit_constant(Constant::String(name.clone()));
                for (fname, fexpr) in fields {
                    main_cx.chunk.emit_constant(Constant::String(fname.clone()));
                    if let Err(e) = main_cx.compile_expr(fexpr) {
                        main_cx
                            .errors
                            .push(Diagnostic::error(e.to_string(), loc.clone()));
                        main_cx.chunk.emit_op(Op::Null);
                    }
                }
                // Not a full object model yet: store name string in global.
                let idx = main_cx.global_index(name);
                // Stack currently has name + pairs; simplify: just store name.
                // Pop extras then define.
                let extra = 1 + fields.len() * 2;
                for _ in 1..extra {
                    main_cx.chunk.emit_op(Op::Pop);
                }
                // top is name string
                main_cx.chunk.emit_op(Op::DefineGlobal);
                main_cx.chunk.emit_u16(idx);
            }
            Item::Scene { name, body, loc } => {
                // Compile scene body as a function.
                let mut scene_chunk = Chunk::new(name.clone());
                {
                    let mut sc = FnCompiler {
                        chunk: &mut scene_chunk,
                        locals: Vec::new(),
                        scope_depth: 0,
                        globals: main_cx.globals,
                        module_globals: main_cx.module_globals,
                        errors: main_cx.errors,
                        loops: Vec::new(),
                    };
                    sc.chunk.map_source(loc.line, loc.column);
                    for stmt in body {
                        if let Err(e) = sc.compile_stmt(stmt) {
                            sc.errors
                                .push(Diagnostic::error(e.to_string(), loc.clone()));
                        }
                    }
                    sc.chunk.emit_op(Op::Null);
                    sc.chunk.emit_op(Op::Return);
                }
                let fidx = cx.module.functions.len() as u16;
                cx.module.exports.insert(name.clone(), fidx);
                cx.module.functions.push(scene_chunk);
                // Bind scene name as function constant global.
                main_cx.chunk.emit_constant(Constant::Function(fidx));
                let g = main_cx.global_index(name);
                main_cx.chunk.emit_op(Op::DefineGlobal);
                main_cx.chunk.emit_u16(g);
            }
            Item::Stmt(stmt) => {
                if let Err(e) = main_cx.compile_stmt(stmt) {
                    main_cx
                        .errors
                        .push(Diagnostic::error(e.to_string(), stmt.loc().clone()));
                }
            }
            Item::Import { .. } | Item::Function { .. } | Item::Screen { .. } => {
                // Declarative screens compile through `velvet-script-layers`.
                // They intentionally add no executable v1 bytecode.
            }
        }
    }
    main.emit_op(Op::Null);
    main.emit_op(Op::Return);

    let main_idx = cx.module.functions.len() as u16;
    cx.module.exports.insert("<script>".into(), main_idx);
    cx.module.functions.push(main);

    // Merge collected errors into diagnostics.
    cx.diagnostics.extend(cx.errors.iter().cloned());

    if !cx.errors.is_empty() {
        if cx.errors.len() == 1 {
            let e = &cx.errors[0];
            return Err(CompileError::Codegen {
                message: e.message.clone(),
                loc: e.loc.display(),
            });
        }
        let first = format!("{}: {}", cx.errors[0].loc.display(), cx.errors[0].message);
        return Err(CompileError::Many {
            count: cx.errors.len(),
            first,
            diagnostics: cx.errors.clone(),
        });
    }

    Ok(CompileResult {
        module: cx.module,
        diagnostics: cx.diagnostics,
    })
}

struct Compiler {
    module: BytecodeModule,
    diagnostics: Vec<Diagnostic>,
    globals: HashMap<String, u16>,
    errors: Vec<Diagnostic>,
}

impl Compiler {
    fn declare_global(&mut self, name: &str) {
        if self.globals.contains_key(name) {
            return;
        }
        let idx = self.module.globals.len() as u16;
        self.module.globals.push(name.to_string());
        self.globals.insert(name.to_string(), idx);
    }

    fn compile_function(
        &mut self,
        name: &str,
        params: &[velvet_script_ast::Param],
        body: &[Stmt],
        loc: &SourceLoc,
    ) -> Result<(), CompileError> {
        let mut chunk = Chunk::new(name);
        chunk.arity = params.len() as u8;
        let mut fc = FnCompiler {
            chunk: &mut chunk,
            locals: Vec::new(),
            scope_depth: 0,
            globals: &mut self.globals,
            module_globals: &mut self.module.globals,
            errors: &mut self.errors,
            loops: Vec::new(),
        };
        fc.chunk.map_source(loc.line, loc.column);
        for p in params {
            fc.add_local(&p.name);
        }
        fc.chunk.locals = fc.locals.len() as u8;
        fc.begin_scope();
        for stmt in body {
            fc.compile_stmt(stmt)?;
        }
        // Implicit return null if no return.
        fc.chunk.emit_op(Op::Null);
        fc.chunk.emit_op(Op::Return);
        fc.chunk.locals = fc.locals.len() as u8;

        let fidx = self.module.functions.len() as u16;
        self.module.exports.insert(name.to_string(), fidx);
        self.module.functions.push(chunk);

        Ok(())
    }
}

struct Local {
    name: String,
    depth: i32,
}

struct FnCompiler<'a> {
    chunk: &'a mut Chunk,
    locals: Vec<Local>,
    scope_depth: i32,
    globals: &'a mut HashMap<String, u16>,
    module_globals: &'a mut Vec<String>,
    errors: &'a mut Vec<Diagnostic>,
    loops: Vec<LoopContext>,
}

struct LoopContext {
    continue_target: Option<usize>,
    continue_jumps: Vec<usize>,
    break_jumps: Vec<usize>,
    scope_depth: i32,
}

impl FnCompiler<'_> {
    fn error(&mut self, message: impl Into<String>, loc: &SourceLoc) {
        self.errors.push(Diagnostic::error(message, loc.clone()));
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        while let Some(local) = self.locals.last() {
            if local.depth < self.scope_depth {
                break;
            }
            self.locals.pop();
            self.chunk.emit_op(Op::Pop);
        }
        self.scope_depth -= 1;
    }

    fn emit_scope_cleanup(&mut self, target_depth: i32) {
        for local in self.locals.iter().rev() {
            if local.depth <= target_depth {
                break;
            }
            self.chunk.emit_op(Op::Pop);
        }
    }

    fn patch_forward_jump(&mut self, operand: usize, target: usize) {
        let offset = target.saturating_sub(operand + 2);
        self.chunk.patch_u16(operand, offset as u16);
    }

    fn add_local(&mut self, name: &str) {
        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scope_depth,
        });
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i as u8);
            }
        }
        None
    }

    fn global_index(&mut self, name: &str) -> u16 {
        if let Some(i) = self.globals.get(name) {
            return *i;
        }
        let idx = self.module_globals.len() as u16;
        self.module_globals.push(name.to_string());
        self.globals.insert(name.to_string(), idx);
        idx
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Expr { expr, .. } => {
                self.compile_expr(expr)?;
                self.chunk.emit_op(Op::Pop);
            }
            Stmt::Let {
                name, init, loc, ..
            } => {
                self.chunk.map_source(loc.line, loc.column);
                self.compile_expr(init)?;
                if self.scope_depth > 0 {
                    self.add_local(name);
                    // value stays on stack as local slot
                } else {
                    let idx = self.global_index(name);
                    self.chunk.emit_op(Op::DefineGlobal);
                    self.chunk.emit_u16(idx);
                }
            }
            Stmt::Const { name, init, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.compile_expr(init)?;
                if self.scope_depth > 0 {
                    self.add_local(name);
                } else {
                    let idx = self.global_index(name);
                    self.chunk.emit_op(Op::DefineGlobal);
                    self.chunk.emit_u16(idx);
                }
            }
            Stmt::Block { body, .. } => {
                self.begin_scope();
                for s in body {
                    self.compile_stmt(s)?;
                }
                self.end_scope();
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                loc,
            } => {
                self.chunk.map_source(loc.line, loc.column);
                self.compile_expr(cond)?;
                self.chunk.emit_op(Op::JumpIfFalse);
                let else_jump = self.chunk.len();
                self.chunk.emit_u16(0);
                self.chunk.emit_op(Op::Pop);
                self.compile_stmt(then_body)?;
                self.chunk.emit_op(Op::Jump);
                let end_jump = self.chunk.len();
                self.chunk.emit_u16(0);
                // patch else
                let else_offset = self.chunk.len() - (else_jump + 2);
                self.chunk.patch_u16(else_jump, else_offset as u16);
                self.chunk.emit_op(Op::Pop);
                if let Some(e) = else_body {
                    self.compile_stmt(e)?;
                }
                let end_offset = self.chunk.len() - (end_jump + 2);
                self.chunk.patch_u16(end_jump, end_offset as u16);
            }
            Stmt::While { cond, body, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                let loop_start = self.chunk.len();
                self.compile_expr(cond)?;
                self.chunk.emit_op(Op::JumpIfFalse);
                let exit_jump = self.chunk.len();
                self.chunk.emit_u16(0);
                self.chunk.emit_op(Op::Pop);
                self.loops.push(LoopContext {
                    continue_target: Some(loop_start),
                    continue_jumps: Vec::new(),
                    break_jumps: Vec::new(),
                    scope_depth: self.scope_depth,
                });
                self.compile_stmt(body)?;
                self.chunk.emit_op(Op::Loop);
                let back = self.chunk.len() + 2 - loop_start;
                self.chunk.emit_u16(back as u16);
                let exit_off = self.chunk.len() - (exit_jump + 2);
                self.chunk.patch_u16(exit_jump, exit_off as u16);
                self.chunk.emit_op(Op::Pop);
                let break_target = self.chunk.len();
                let loop_context = self.loops.pop().expect("while loop context");
                for jump in loop_context.break_jumps {
                    self.patch_forward_jump(jump, break_target);
                }
            }
            Stmt::Return { value, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                if let Some(v) = value {
                    self.compile_expr(v)?;
                } else {
                    self.chunk.emit_op(Op::Null);
                }
                self.chunk.emit_op(Op::Return);
            }
            Stmt::Dialogue { speaker, text, loc } => {
                // Compile as print of string for executable scripts.
                self.chunk.map_source(loc.line, loc.column);
                let msg = match speaker {
                    Some(s) => format!("{s}: {text}"),
                    None => text.clone(),
                };
                self.chunk.emit_constant(Constant::String(msg));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Jump { label, loc } => {
                // Call scene/function by name if global function.
                self.chunk.map_source(loc.line, loc.column);
                let idx = self.global_index(label);
                self.chunk.emit_op(Op::GetGlobal);
                self.chunk.emit_u16(idx);
                self.chunk.emit_op(Op::Call);
                self.chunk.emit_u8(0);
                self.chunk.emit_op(Op::Pop);
            }
            Stmt::Label { .. } => {
                // Labels are no-ops in bytecode v1 (jump resolves by function name).
            }
            Stmt::Choice { options, loc } => {
                // Deterministic for tests: take first option body.
                self.chunk.map_source(loc.line, loc.column);
                if let Some(arm) = options.first() {
                    for s in &arm.body {
                        self.compile_stmt(s)?;
                    }
                }
            }
            Stmt::Show { target, at, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                let msg = match at {
                    Some(a) => format!("show {target} at {a}"),
                    None => format!("show {target}"),
                };
                self.chunk.emit_constant(Constant::String(msg));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Background { path, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk
                    .emit_constant(Constant::String(format!("background {path}")));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Music { path, fade_in, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                let msg = match fade_in {
                    Some(f) => format!("music {path} fade_in {f}"),
                    None => format!("music {path}"),
                };
                self.chunk.emit_constant(Constant::String(msg));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Hide { target, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk
                    .emit_constant(Constant::String(format!("hide {target}")));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::End { ending, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                let msg = match ending {
                    Some(e) => format!("end {e}"),
                    None => "end".into(),
                };
                self.chunk.emit_constant(Constant::String(msg));
                self.chunk.emit_op(Op::Print);
                self.chunk.emit_op(Op::Null);
                self.chunk.emit_op(Op::Return);
            }
            Stmt::Call { target, loc } => {
                // Call scene/function by name (same as jump in bytecode v1).
                self.chunk.map_source(loc.line, loc.column);
                let idx = self.global_index(target);
                self.chunk.emit_op(Op::GetGlobal);
                self.chunk.emit_u16(idx);
                self.chunk.emit_op(Op::Call);
                self.chunk.emit_u8(0);
                self.chunk.emit_op(Op::Pop);
            }
            Stmt::HostCall { name, loc, .. } => {
                // Classic host commands are story-runtime only; bytecode prints intent.
                self.chunk.map_source(loc.line, loc.column);
                self.chunk
                    .emit_constant(Constant::String(format!("host {name}")));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Transition { name, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk
                    .emit_constant(Constant::String(format!("transition {name}")));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Sound { path, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk
                    .emit_constant(Constant::String(format!("sound {path}")));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::Pause { seconds, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                let msg = match seconds {
                    Some(s) => format!("pause {s}"),
                    None => "pause".into(),
                };
                self.chunk.emit_constant(Constant::String(msg));
                self.chunk.emit_op(Op::Print);
            }
            Stmt::For {
                name,
                iter,
                body,
                loc,
            } => {
                // Desugar: let __it = iter; let __i = 0; while __i < len(__it) { let name = __it[__i]; body; __i += 1 }
                self.chunk.map_source(loc.line, loc.column);
                self.begin_scope();
                // __it
                self.compile_expr(iter)?;
                self.add_local("__it");
                let it_slot = (self.locals.len() - 1) as u8;
                // __i = 0
                self.chunk.emit_constant(Constant::Int(0));
                self.add_local("__i");
                let i_slot = (self.locals.len() - 1) as u8;

                let loop_start = self.chunk.len();
                // cond: __i < len(__it)
                self.chunk.emit_op(Op::GetLocal);
                self.chunk.emit_u8(i_slot);
                self.chunk.emit_op(Op::GetLocal);
                self.chunk.emit_u8(it_slot);
                if let Some(native) = lookup_native("len") {
                    self.chunk.emit_native_call(native.as_u16(), 1);
                } else {
                    self.error("stdlib len missing for for-loop lowering", loc);
                    self.chunk.emit_constant(Constant::Int(0));
                }
                self.chunk.emit_op(Op::Lt);
                self.chunk.emit_op(Op::JumpIfFalse);
                let exit_jump = self.chunk.len();
                self.chunk.emit_u16(0);
                self.chunk.emit_op(Op::Pop);

                // bind name = __it[__i]
                self.begin_scope();
                self.chunk.emit_op(Op::GetLocal);
                self.chunk.emit_u8(it_slot);
                self.chunk.emit_op(Op::GetLocal);
                self.chunk.emit_u8(i_slot);
                self.chunk.emit_op(Op::GetIndex);
                self.add_local(name);

                self.loops.push(LoopContext {
                    continue_target: None,
                    continue_jumps: Vec::new(),
                    break_jumps: Vec::new(),
                    // `name` lives in the per-iteration scope and must be
                    // removed on both continue and break.
                    scope_depth: self.scope_depth - 1,
                });
                self.compile_stmt(body)?;
                self.end_scope();

                // __i += 1
                let continue_target = self.chunk.len();
                if let Some(context) = self.loops.last_mut() {
                    context.continue_target = Some(continue_target);
                    let jumps = std::mem::take(&mut context.continue_jumps);
                    for jump in jumps {
                        self.patch_forward_jump(jump, continue_target);
                    }
                }
                self.chunk.emit_op(Op::GetLocal);
                self.chunk.emit_u8(i_slot);
                self.chunk.emit_constant(Constant::Int(1));
                self.chunk.emit_op(Op::Add);
                self.chunk.emit_op(Op::SetLocal);
                self.chunk.emit_u8(i_slot);
                self.chunk.emit_op(Op::Pop);

                self.chunk.emit_op(Op::Loop);
                let back = self.chunk.len() + 2 - loop_start;
                self.chunk.emit_u16(back as u16);
                let exit_off = self.chunk.len() - (exit_jump + 2);
                self.chunk.patch_u16(exit_jump, exit_off as u16);
                self.chunk.emit_op(Op::Pop);
                let break_target = self.chunk.len();
                let loop_context = self.loops.pop().expect("for loop context");
                for jump in loop_context.break_jumps {
                    self.patch_forward_jump(jump, break_target);
                }
                self.end_scope();
            }
            Stmt::Break { loc } => {
                let Some(context) = self.loops.last() else {
                    self.error("break used outside a loop", loc);
                    return Ok(());
                };
                let scope_depth = context.scope_depth;
                self.emit_scope_cleanup(scope_depth);
                self.chunk.emit_op(Op::Jump);
                let jump = self.chunk.len();
                self.chunk.emit_u16(0);
                self.loops.last_mut().unwrap().break_jumps.push(jump);
            }
            Stmt::Continue { loc } => {
                let Some(context) = self.loops.last() else {
                    self.error("continue used outside a loop", loc);
                    return Ok(());
                };
                let scope_depth = context.scope_depth;
                let continue_target = context.continue_target;
                self.emit_scope_cleanup(scope_depth);
                match continue_target {
                    Some(target) if target <= self.chunk.len() => {
                        self.chunk.emit_op(Op::Loop);
                        let back = self.chunk.len() + 2 - target;
                        self.chunk.emit_u16(back as u16);
                    }
                    _ => {
                        self.chunk.emit_op(Op::Jump);
                        let jump = self.chunk.len();
                        self.chunk.emit_u16(0);
                        self.loops.last_mut().unwrap().continue_jumps.push(jump);
                    }
                }
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        // Constant folding for pure literal trees.
        if let Some(c) = try_fold_const(expr) {
            self.chunk.map_source(expr.loc().line, expr.loc().column);
            match c {
                Folded::Null => self.chunk.emit_op(Op::Null),
                Folded::Bool(true) => self.chunk.emit_op(Op::True),
                Folded::Bool(false) => self.chunk.emit_op(Op::False),
                Folded::Int(i) => self.chunk.emit_constant(Constant::Int(i)),
                Folded::Float(f) => self.chunk.emit_constant(Constant::Float(f)),
                Folded::String(s) => self.chunk.emit_constant(Constant::String(s)),
            }
            return Ok(());
        }

        match expr {
            Expr::Null { loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk.emit_op(Op::Null);
            }
            Expr::Bool { value, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk
                    .emit_op(if *value { Op::True } else { Op::False });
            }
            Expr::Int { value, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk.emit_constant(Constant::Int(*value));
            }
            Expr::Float { value, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk.emit_constant(Constant::Float(*value));
            }
            Expr::String { value, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.chunk.emit_constant(Constant::String(value.clone()));
            }
            Expr::Ident { name, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                if let Some(slot) = self.resolve_local(name) {
                    self.chunk.emit_op(Op::GetLocal);
                    self.chunk.emit_u8(slot);
                } else if let Some(value) = lookup_math_constant(name) {
                    self.chunk.emit_constant(Constant::Float(value));
                } else if let Some(native) = lookup_native(name) {
                    // Push native as a first-class value.
                    self.chunk.emit_constant(Constant::Native(native.as_u16()));
                } else {
                    let idx = self.global_index(name);
                    self.chunk.emit_op(Op::GetGlobal);
                    self.chunk.emit_u16(idx);
                }
            }
            Expr::List { elements, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                for e in elements {
                    self.compile_expr(e)?;
                }
                self.chunk.emit_op(Op::MakeList);
                self.chunk.emit_u16(elements.len() as u16);
            }
            Expr::Map { entries, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                for (key, value) in entries {
                    self.chunk.emit_constant(Constant::String(key.clone()));
                    self.compile_expr(value)?;
                }
                self.chunk.emit_op(Op::MakeMap);
                self.chunk.emit_u16(entries.len() as u16);
            }
            Expr::Unary { op, expr, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.compile_expr(expr)?;
                match op {
                    UnaryOp::Neg => self.chunk.emit_op(Op::Neg),
                    UnaryOp::Not => self.chunk.emit_op(Op::Not),
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                loc,
            } => {
                self.chunk.map_source(loc.line, loc.column);
                match op {
                    BinOp::Assign => match left.as_ref() {
                        Expr::Index {
                            object,
                            index,
                            loc: iloc,
                        } => {
                            self.chunk.map_source(iloc.line, iloc.column);
                            self.compile_expr(object)?;
                            self.compile_expr(index)?;
                            self.compile_expr(right)?;
                            self.chunk.emit_op(Op::SetIndex);
                        }
                        Expr::Field { object, field, loc } => {
                            self.chunk.map_source(loc.line, loc.column);
                            self.compile_expr(object)?;
                            self.chunk.emit_constant(Constant::String(field.clone()));
                            self.compile_expr(right)?;
                            self.chunk.emit_op(Op::SetIndex);
                        }
                        _ => {
                            self.compile_expr(right)?;
                            self.compile_assign_target(left)?;
                        }
                    },
                    BinOp::AddAssign | BinOp::SubAssign | BinOp::MulAssign | BinOp::DivAssign => {
                        let arithmetic = match op {
                            BinOp::AddAssign => Op::Add,
                            BinOp::SubAssign => Op::Sub,
                            BinOp::MulAssign => Op::Mul,
                            BinOp::DivAssign => Op::Div,
                            _ => unreachable!(),
                        };
                        match left.as_ref() {
                            Expr::Index { object, index, .. } => {
                                self.compile_expr(object)?;
                                self.compile_expr(index)?;
                                self.compile_expr(right)?;
                                self.chunk.emit_op(Op::UpdateIndex);
                                self.chunk.emit_u8(arithmetic.to_u8());
                            }
                            Expr::Field { object, field, .. } => {
                                self.compile_expr(object)?;
                                self.chunk.emit_constant(Constant::String(field.clone()));
                                self.compile_expr(right)?;
                                self.chunk.emit_op(Op::UpdateIndex);
                                self.chunk.emit_u8(arithmetic.to_u8());
                            }
                            _ => {
                                self.compile_expr(left)?;
                                self.compile_expr(right)?;
                                self.chunk.emit_op(arithmetic);
                                self.compile_assign_target(left)?;
                            }
                        }
                    }
                    BinOp::And => {
                        self.compile_expr(left)?;
                        self.chunk.emit_op(Op::JumpIfFalse);
                        let jump = self.chunk.len();
                        self.chunk.emit_u16(0);
                        self.chunk.emit_op(Op::Pop);
                        self.compile_expr(right)?;
                        let off = self.chunk.len() - (jump + 2);
                        self.chunk.patch_u16(jump, off as u16);
                    }
                    BinOp::Or => {
                        self.compile_expr(left)?;
                        self.chunk.emit_op(Op::JumpIfTrue);
                        let jump = self.chunk.len();
                        self.chunk.emit_u16(0);
                        self.chunk.emit_op(Op::Pop);
                        self.compile_expr(right)?;
                        let off = self.chunk.len() - (jump + 2);
                        self.chunk.patch_u16(jump, off as u16);
                    }
                    other => {
                        self.compile_expr(left)?;
                        self.compile_expr(right)?;
                        let op = match other {
                            BinOp::Add => Op::Add,
                            BinOp::Sub => Op::Sub,
                            BinOp::Mul => Op::Mul,
                            BinOp::Div => Op::Div,
                            BinOp::Rem => Op::Rem,
                            BinOp::Eq => Op::Eq,
                            BinOp::Ne => Op::Ne,
                            BinOp::Lt => Op::Lt,
                            BinOp::Le => Op::Le,
                            BinOp::Gt => Op::Gt,
                            BinOp::Ge => Op::Ge,
                            _ => unreachable!(),
                        };
                        self.chunk.emit_op(op);
                    }
                }
            }
            Expr::Call { callee, args, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    // `yield(value)` is a general cooperative suspension primitive.
                    // The yielded value stays on the stack and becomes the expression
                    // result when a coroutine resumes (optionally replaced by the host).
                    if name == "yield" {
                        match args.as_slice() {
                            [] => self.chunk.emit_op(Op::Null),
                            [value] => self.compile_expr(value)?,
                            _ => {
                                self.error("yield expects zero or one argument", loc);
                                self.chunk.emit_op(Op::Null);
                            }
                        }
                        self.chunk.emit_op(Op::Yield);
                        return Ok(());
                    }

                    // Direct native call when callee is a known stdlib name.
                    if let Some(native) = lookup_native(name) {
                        for a in args {
                            self.compile_expr(a)?;
                        }
                        self.chunk
                            .emit_native_call(native.as_u16(), args.len() as u8);
                        return Ok(());
                    }
                }
                self.compile_expr(callee)?;
                for a in args {
                    self.compile_expr(a)?;
                }
                self.chunk.emit_op(Op::Call);
                self.chunk.emit_u8(args.len() as u8);
            }
            Expr::Field { object, field, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.compile_expr(object)?;
                self.chunk.emit_constant(Constant::String(field.clone()));
                self.chunk.emit_op(Op::GetIndex);
            }
            Expr::Index { object, index, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.chunk.emit_op(Op::GetIndex);
            }
        }
        Ok(())
    }

    fn compile_assign_target(&mut self, target: &Expr) -> Result<(), CompileError> {
        match target {
            Expr::Ident { name, loc } => {
                self.chunk.map_source(loc.line, loc.column);
                if let Some(slot) = self.resolve_local(name) {
                    self.chunk.emit_op(Op::SetLocal);
                    self.chunk.emit_u8(slot);
                } else {
                    let idx = self.global_index(name);
                    self.chunk.emit_op(Op::SetGlobal);
                    self.chunk.emit_u16(idx);
                }
            }
            Expr::Index { object, index, loc } => {
                // value already on stack. Need container, index, value.
                self.chunk.map_source(loc.line, loc.column);
                // stack: value
                // We need: container, index, value
                // Rotate by compiling object/index then using stack juggling:
                // Dup is only top. Use: store value pattern:
                // Actually: compile object, compile index -> stack: value, object, index
                // Then we need SetIndex expecting container, index, value.
                // Swap: emit sequence carefully.
                //
                // value
                // compile object -> value, object
                // compile index -> value, object, index
                // We need object, index, value. Rotate 3:
                // Not available. Alternative: compile object, index first in Assign path.
                //
                // For now: stack is value; compile object and index then use a local pattern:
                // value; object; index -> SetIndex accepts value, object, index order.
                //
                // Define SetIndex as: pop index, pop container, pop value? No current is container, index, value.
                //
                // Emit: temporary rotation via Dup/not available for mid-stack.
                // Recompile assignment specially; caller for Assign already did right first.
                //
                // Work around: pop value into... we don't have stores.
                // Compile as: object, index, value by reordering at assign site.
                //
                // Fallback approach: leave value, compile object, compile index,
                // then call a helper that treats stack as [value, container, index]
                // Keep VM as container,index,value and fix here by
                // not using compile_assign_target for Index from Assign.

                self.error("internal: index assign should use specialized path", loc);
                let _ = (object, index);
            }
            _ => {
                self.error("invalid assignment target", target.loc());
            }
        }
        Ok(())
    }
}

// Specialized assign handling for index is done by overriding Binary Assign path.
// Patch compile_expr Assign branch by handling Index left specially via a free function
// called from compile_expr; the Assign branch handles it above.

/// Folded constant value.
#[derive(Debug, Clone, PartialEq)]
enum Folded {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

fn try_fold_const(expr: &Expr) -> Option<Folded> {
    match expr {
        Expr::Null { .. } => Some(Folded::Null),
        Expr::Bool { value, .. } => Some(Folded::Bool(*value)),
        Expr::Int { value, .. } => Some(Folded::Int(*value)),
        Expr::Float { value, .. } => Some(Folded::Float(*value)),
        Expr::String { value, .. } => Some(Folded::String(value.clone())),
        Expr::Unary { op, expr, .. } => {
            let v = try_fold_const(expr)?;
            match (op, v) {
                (UnaryOp::Neg, Folded::Int(i)) => i.checked_neg().map(Folded::Int),
                (UnaryOp::Neg, Folded::Float(f)) => Some(Folded::Float(-f)),
                (UnaryOp::Not, Folded::Bool(b)) => Some(Folded::Bool(!b)),
                (UnaryOp::Not, Folded::Null) => Some(Folded::Bool(true)),
                (UnaryOp::Not, Folded::Int(i)) => Some(Folded::Bool(i == 0)),
                _ => None,
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            // Don't fold assignments or short-circuit ops (side effects / control flow).
            if matches!(
                op,
                BinOp::Assign
                    | BinOp::AddAssign
                    | BinOp::SubAssign
                    | BinOp::MulAssign
                    | BinOp::DivAssign
                    | BinOp::And
                    | BinOp::Or
            ) {
                return None;
            }
            let l = try_fold_const(left)?;
            let r = try_fold_const(right)?;
            fold_binary(*op, l, r)
        }
        _ => None,
    }
}

fn fold_binary(op: BinOp, l: Folded, r: Folded) -> Option<Folded> {
    match op {
        BinOp::Add => match (l, r) {
            (Folded::Int(a), Folded::Int(b)) => a.checked_add(b).map(Folded::Int),
            (Folded::Float(a), Folded::Float(b)) => Some(Folded::Float(a + b)),
            (Folded::Int(a), Folded::Float(b)) => Some(Folded::Float(a as f64 + b)),
            (Folded::Float(a), Folded::Int(b)) => Some(Folded::Float(a + b as f64)),
            (Folded::String(a), Folded::String(b)) => Some(Folded::String(format!("{a}{b}"))),
            (Folded::String(a), r) => Some(Folded::String(format!("{a}{}", folded_display(&r)))),
            (l, Folded::String(b)) => Some(Folded::String(format!("{}{b}", folded_display(&l)))),
            _ => None,
        },
        BinOp::Sub => num2(l, r, |a, b| a - b, i64::checked_sub),
        BinOp::Mul => num2(l, r, |a, b| a * b, i64::checked_mul),
        BinOp::Div => match (l, r) {
            (Folded::Int(a), Folded::Int(b)) => a.checked_div(b).map(Folded::Int),
            (Folded::Float(a), Folded::Float(b)) if b != 0.0 => Some(Folded::Float(a / b)),
            (Folded::Int(a), Folded::Float(b)) if b != 0.0 => Some(Folded::Float(a as f64 / b)),
            (Folded::Float(a), Folded::Int(b)) if b != 0 => Some(Folded::Float(a / b as f64)),
            _ => None,
        },
        BinOp::Rem => match (l, r) {
            (Folded::Int(a), Folded::Int(b)) => a.checked_rem(b).map(Folded::Int),
            (Folded::Float(a), Folded::Float(b)) if b != 0.0 => Some(Folded::Float(a % b)),
            _ => None,
        },
        BinOp::Eq => Some(Folded::Bool(folded_eq(&l, &r))),
        BinOp::Ne => Some(Folded::Bool(!folded_eq(&l, &r))),
        BinOp::Lt => cmp2(l, r, |a, b| a < b),
        BinOp::Le => cmp2(l, r, |a, b| a <= b),
        BinOp::Gt => cmp2(l, r, |a, b| a > b),
        BinOp::Ge => cmp2(l, r, |a, b| a >= b),
        _ => None,
    }
}

fn num2(
    l: Folded,
    r: Folded,
    f: impl Fn(f64, f64) -> f64,
    i: impl Fn(i64, i64) -> Option<i64>,
) -> Option<Folded> {
    match (l, r) {
        (Folded::Int(a), Folded::Int(b)) => i(a, b).map(Folded::Int),
        (Folded::Float(a), Folded::Float(b)) => Some(Folded::Float(f(a, b))),
        (Folded::Int(a), Folded::Float(b)) => Some(Folded::Float(f(a as f64, b))),
        (Folded::Float(a), Folded::Int(b)) => Some(Folded::Float(f(a, b as f64))),
        _ => None,
    }
}

fn cmp2(l: Folded, r: Folded, f: impl Fn(f64, f64) -> bool) -> Option<Folded> {
    let a = match l {
        Folded::Int(i) => i as f64,
        Folded::Float(x) => x,
        _ => return None,
    };
    let b = match r {
        Folded::Int(i) => i as f64,
        Folded::Float(x) => x,
        _ => return None,
    };
    Some(Folded::Bool(f(a, b)))
}

fn folded_eq(l: &Folded, r: &Folded) -> bool {
    match (l, r) {
        (Folded::Null, Folded::Null) => true,
        (Folded::Bool(a), Folded::Bool(b)) => a == b,
        (Folded::Int(a), Folded::Int(b)) => a == b,
        (Folded::Float(a), Folded::Float(b)) => a == b,
        (Folded::Int(a), Folded::Float(b)) => *a as f64 == *b,
        (Folded::Float(a), Folded::Int(b)) => *a == *b as f64,
        (Folded::String(a), Folded::String(b)) => a == b,
        _ => false,
    }
}

fn folded_display(f: &Folded) -> String {
    match f {
        Folded::Null => "null".into(),
        Folded::Bool(b) => b.to_string(),
        Folded::Int(i) => i.to_string(),
        Folded::Float(x) => x.to_string(),
        Folded::String(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_bytecode::Op;

    #[test]
    fn compile_arithmetic_function() {
        let src = r#"
function add(a, b) {
    return a + b
}
let x = add(2, 40)
"#;
        let r = compile_source(src, Some("t.vel")).unwrap();
        assert!(!r.module.functions.is_empty());
        assert!(r.module.exports.contains_key("add"));
        assert!(r.module.metadata.source_hash.is_some());
    }

    #[test]
    fn compile_scene() {
        let src = r#"
state {
    trust: int = 0
}
scene main {
    trust += 1
}
"#;
        let r = compile_source(src, None).unwrap();
        assert!(r.module.exports.contains_key("main"));
    }

    #[test]
    fn constant_folding_emits_single_constant() {
        let src = r#"
function f() {
    return 1 + 2 * 3
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        // Folded to 7: CONSTANT, Return path; no ADD/MUL opcodes.
        assert!(
            !chunk
                .code
                .iter()
                .any(|&b| b == Op::Add as u8 || b == Op::Mul as u8),
            "expected folded arithmetic, code={:?}",
            chunk.code
        );
        assert!(chunk
            .constants
            .iter()
            .any(|c| matches!(c, Constant::Int(7))));
    }

    #[test]
    fn native_call_emitted() {
        let src = r#"
function f() {
    return abs(-5)
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        assert!(
            chunk.code.contains(&(Op::NativeCall as u8)),
            "expected NativeCall in {:?}",
            chunk.code
        );
    }

    #[test]
    fn index_expr_emits_get_index() {
        let src = r#"
function f() {
    let xs = [1, 2, 3]
    return xs[0]
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        assert!(chunk.code.contains(&(Op::GetIndex as u8)));
    }

    #[test]
    fn field_access_lowers_to_string_indexing() {
        let src = r#"
function f() {
    let a = x.y
    let b = z.w
    return a
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let chunk = compiled
            .module
            .functions
            .iter()
            .find(|chunk| chunk.name == "f")
            .unwrap();
        assert_eq!(
            chunk
                .code
                .iter()
                .filter(|&&byte| byte == Op::GetIndex.to_u8())
                .count(),
            2
        );
    }

    #[test]
    fn stdlib_globals_declared() {
        let r = compile_source("function f() { return 1 }\n", None).unwrap();
        assert!(r.module.globals.iter().any(|g| g == "abs"));
        assert!(r.module.globals.iter().any(|g| g == "len"));
    }

    #[test]
    fn compile_hide_end_call() {
        let src = r#"
scene main {
    hide hero
    call other
    end "good"
}
scene other {
    "ok"
}
"#;
        let r = compile_source(src, Some("story.vel")).unwrap();
        assert!(r.module.exports.contains_key("main"));
        assert!(r.module.exports.contains_key("other"));
    }

    #[test]
    fn compile_for_loop_uses_len_and_index() {
        let src = r#"
function sum(xs) {
    let total = 0
    for x in xs {
        total += x
    }
    return total
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "sum").unwrap();
        assert!(
            chunk.code.contains(&(Op::GetIndex as u8)),
            "for lowering should index the iterable"
        );
        assert!(
            chunk.code.contains(&(Op::NativeCall as u8)),
            "for lowering should call len"
        );
    }

    #[test]
    fn compile_if_else_chain() {
        let src = r#"
function sign(n) {
    if n > 0 {
        return 1
    } else if n < 0 {
        return -1
    } else {
        return 0
    }
}
"#;
        let r = compile_source(src, None).unwrap();
        assert!(r.module.exports.contains_key("sign"));
        let chunk = r
            .module
            .functions
            .iter()
            .find(|c| c.name == "sign")
            .unwrap();
        assert!(chunk.code.contains(&(Op::JumpIfFalse as u8)));
    }

    #[test]
    fn compile_while_loop_emits_loop_op() {
        let src = r#"
function f() {
    let i = 0
    while i < 3 {
        i += 1
    }
    return i
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        assert!(chunk.code.contains(&(Op::Loop as u8)));
    }

    #[test]
    fn compile_list_literal() {
        let src = r#"
function f() {
    return [1, 2, 3]
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        assert!(chunk.code.contains(&(Op::MakeList as u8)));
    }

    #[test]
    fn compile_short_circuit_and_or() {
        let src = r#"
function f(a, b) {
    return a && b || false
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        assert!(chunk.code.contains(&(Op::JumpIfFalse as u8)));
        assert!(chunk.code.contains(&(Op::JumpIfTrue as u8)));
    }

    #[test]
    fn compile_string_fold() {
        let src = r#"
function f() {
    return "hello" + " " + "world"
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        assert!(
            !chunk.code.contains(&(Op::Add as u8)),
            "string concat should fold"
        );
        assert!(chunk.constants.iter().any(|c| matches!(
            c,
            Constant::String(s) if s == "hello world"
        )));
    }

    #[test]
    fn compile_dialogue_and_choice() {
        let src = r#"
scene main {
    hero "Hi"
    choice {
        "A" { jump end }
        "B" { jump end }
    }
}
scene end {
    "done"
}
"#;
        let r = compile_source(src, None).unwrap();
        assert!(r.module.exports.contains_key("main"));
    }

    #[test]
    fn compile_source_hash_and_path() {
        let src = "function f() { return 1 }\n";
        let r = compile_source(src, Some("path/to.vel")).unwrap();
        assert_eq!(
            r.module.metadata.source_path.as_deref(),
            Some("path/to.vel")
        );
        assert!(r.module.metadata.source_hash.is_some());
    }

    #[test]
    fn break_continue_compile_as_structured_jumps() {
        let src = r#"
function f() {
    while true {
        break
        continue
    }
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let function =
            &compiled.module.functions[*compiled.module.exports.get("f").unwrap() as usize];
        assert!(function.code.contains(&Op::Jump.to_u8()));
        assert!(function.code.contains(&Op::Loop.to_u8()));
    }

    #[test]
    fn compile_unary_not_and_neg() {
        let src = r#"
function f(x) {
    return !x + -1
}
"#;
        // !x + -1 is (!x) + (-1) after unary; fold may leave ops for x.
        let r = compile_source(src, None).unwrap();
        assert!(r.module.exports.contains_key("f"));
    }

    #[test]
    fn golden_add_function_exports() {
        let src = r#"
function add(a, b) { return a + b }
function sub(a, b) { return a - b }
function mul(a, b) { return a * b }
function div(a, b) { return a / b }
function rem(a, b) { return a % b }
"#;
        let r = compile_source(src, None).unwrap();
        for name in ["add", "sub", "mul", "div", "rem"] {
            assert!(r.module.exports.contains_key(name), "missing {name}");
        }
    }

    #[test]
    fn comparison_ops_emitted() {
        let src = r#"
function f(a, b) {
    return a < b && a <= b && a > b && a >= b && a == b && a != b
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r.module.functions.iter().find(|c| c.name == "f").unwrap();
        for op in [Op::Lt, Op::Le, Op::Gt, Op::Ge, Op::Eq, Op::Ne] {
            assert!(
                chunk.code.contains(&(op as u8)),
                "missing {op:?} in {:?}",
                chunk.code
            );
        }
    }

    #[test]
    fn declarative_screen_is_accepted_without_v1_bytecode() {
        let source = r#"
screen main_menu {
    title: "VELVET ARCANA"
    button start { label: "START"; action: "start_run"; }
}
function helper() { return 1 }
"#;
        let compiled = compile_source(source, Some("menu.vel")).unwrap();
        assert!(compiled.module.exports.contains_key("helper"));
        assert!(!compiled.module.exports.contains_key("main_menu"));
    }

    #[test]
    fn yield_call_emits_cooperative_opcode() {
        let src = r#"
function request() {
    let answer = yield(["math.double", 21])
    return answer
}
"#;
        let r = compile_source(src, None).unwrap();
        let chunk = r
            .module
            .functions
            .iter()
            .find(|c| c.name == "request")
            .unwrap();
        assert!(
            chunk.code.contains(&(Op::Yield as u8)),
            "yield call should lower to Op::Yield: {:?}",
            chunk.code
        );
    }

    #[test]
    fn yield_rejects_more_than_one_argument() {
        let src = r#"
function bad() {
    return yield(1, 2)
}
"#;
        let error = compile_source(src, None).unwrap_err();
        assert!(error
            .to_string()
            .contains("yield expects zero or one argument"));
    }
}
