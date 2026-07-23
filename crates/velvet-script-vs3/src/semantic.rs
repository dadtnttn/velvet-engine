use std::collections::{BTreeMap, HashMap};

use velvet_script_ast::{BinOp, Expr, Item, Module, SourceLoc, Stmt, UnaryOp};
use velvet_script_bytecode::{lookup_math_constant, lookup_native, NativeId, NativeType};

use crate::{Vs3Diagnostic, Vs3Type};

#[derive(Debug, Default)]
pub(crate) struct SemanticInfo {
    pub(crate) signatures: BTreeMap<String, Vec<Option<Vs3Type>>>,
    pub(crate) diagnostics: Vec<Vs3Diagnostic>,
}

#[derive(Debug, Clone, Copy)]
struct Binding {
    ty: Option<Vs3Type>,
    mutable: bool,
}

pub(crate) fn validate(module: &Module) -> SemanticInfo {
    Validator::new().validate(module)
}

struct Validator {
    globals: HashMap<String, Binding>,
    signatures: BTreeMap<String, Vec<Option<Vs3Type>>>,
    scopes: Vec<HashMap<String, Binding>>,
    diagnostics: Vec<Vs3Diagnostic>,
    loop_depth: usize,
}

impl Validator {
    fn new() -> Self {
        let mut globals = HashMap::new();
        for native in NativeId::all() {
            globals.insert(
                native.name().to_string(),
                Binding {
                    ty: None,
                    mutable: false,
                },
            );
        }
        for name in ["PI", "TAU", "E", "EPSILON", "INFINITY", "NAN"] {
            debug_assert!(lookup_math_constant(name).is_some());
            globals.insert(
                name.into(),
                Binding {
                    ty: Some(Vs3Type::Float),
                    mutable: false,
                },
            );
        }
        globals.insert(
            "yield".into(),
            Binding {
                ty: None,
                mutable: false,
            },
        );
        Self {
            globals,
            signatures: BTreeMap::new(),
            scopes: Vec::new(),
            diagnostics: Vec::new(),
            loop_depth: 0,
        }
    }

    fn validate(mut self, module: &Module) -> SemanticInfo {
        self.collect_globals(module);
        for item in &module.items {
            self.check_item(item);
        }
        SemanticInfo {
            signatures: self.signatures,
            diagnostics: self.diagnostics,
        }
    }

    fn collect_globals(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::Import {
                    alias: Some(alias),
                    loc,
                    ..
                } => {
                    self.define_global(
                        alias,
                        Binding {
                            ty: Some(Vs3Type::Any),
                            mutable: false,
                        },
                        loc,
                    );
                }
                Item::Import { .. } => {}
                Item::Function {
                    name, params, loc, ..
                } => {
                    let signature = params
                        .iter()
                        .map(|param| self.parse_type(param.ty.as_deref(), loc))
                        .collect::<Vec<_>>();
                    if self.define_global(
                        name,
                        Binding {
                            ty: None,
                            mutable: false,
                        },
                        loc,
                    ) {
                        self.signatures.insert(name.clone(), signature);
                    }
                }
                Item::State { bindings, .. } => {
                    for binding in bindings {
                        let ty = self.parse_type(binding.ty.as_deref(), &binding.loc);
                        self.define_global(
                            &binding.name,
                            Binding { ty, mutable: true },
                            &binding.loc,
                        );
                    }
                }
                Item::Stmt(Stmt::Let { name, ty, loc, .. }) => {
                    let ty = self.parse_type(ty.as_deref(), loc);
                    self.define_global(name, Binding { ty, mutable: true }, loc);
                }
                Item::Stmt(Stmt::Const { name, loc, .. }) => {
                    self.define_global(
                        name,
                        Binding {
                            ty: None,
                            mutable: false,
                        },
                        loc,
                    );
                }
                _ => {}
            }
        }
    }

    fn define_global(&mut self, name: &str, binding: Binding, loc: &SourceLoc) -> bool {
        if self.globals.contains_key(name) {
            self.error(format!("duplicate or reserved definition `{name}`"), loc);
            false
        } else {
            self.globals.insert(name.to_string(), binding);
            true
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Import { .. } => {}
            Item::Function {
                name,
                params,
                body,
                loc,
            } => {
                let signature = self.signatures.get(name).cloned().unwrap_or_default();
                self.scopes.push(HashMap::new());
                for (index, param) in params.iter().enumerate() {
                    let binding = Binding {
                        ty: signature.get(index).copied().flatten(),
                        mutable: true,
                    };
                    self.define_local(&param.name, binding, loc);
                }
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.scopes.pop();
            }
            Item::State { bindings, .. } => {
                for binding in bindings {
                    let found = self.check_expr(&binding.init);
                    let expected = self.globals.get(&binding.name).and_then(|value| value.ty);
                    self.check_compatible(expected, found, &binding.loc);
                }
            }
            Item::Stmt(stmt @ (Stmt::Let { .. } | Stmt::Const { .. } | Stmt::Expr { .. })) => {
                self.check_stmt(stmt);
            }
            Item::Stmt(stmt) => self.error(
                "only declarations and expressions are allowed at VS3 module scope",
                stmt.loc(),
            ),
            Item::Character { loc, .. } | Item::Scene { loc, .. } | Item::Screen { loc, .. } => {
                self.error(
                    "narrative declarations are not part of VS3; use the story runtime or a host service",
                    loc,
                );
            }
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr { expr, .. } => {
                self.check_expr(expr);
            }
            Stmt::Let {
                name,
                ty,
                init,
                loc,
            } => {
                let found = self.check_expr(init);
                let expected = self.parse_type(ty.as_deref(), loc);
                self.check_compatible(expected, found, loc);
                if let Some(scope) = self.scopes.last() {
                    if scope.contains_key(name) {
                        self.error(format!("duplicate local `{name}`"), loc);
                        return;
                    }
                }
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(
                        name.clone(),
                        Binding {
                            ty: expected.or(found),
                            mutable: true,
                        },
                    );
                }
            }
            Stmt::Const { name, init, loc } => {
                let found = self.check_expr(init);
                if let Some(scope) = self.scopes.last() {
                    if scope.contains_key(name) {
                        self.error(format!("duplicate local `{name}`"), loc);
                        return;
                    }
                }
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(
                        name.clone(),
                        Binding {
                            ty: found,
                            mutable: false,
                        },
                    );
                }
            }
            Stmt::Block { body, .. } => {
                self.scopes.push(HashMap::new());
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.scopes.pop();
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let cond_ty = self.check_expr(cond);
                self.check_compatible(Some(Vs3Type::Bool), cond_ty, cond.loc());
                self.check_stmt(then_body);
                if let Some(else_body) = else_body {
                    self.check_stmt(else_body);
                }
            }
            Stmt::While { cond, body, .. } => {
                let cond_ty = self.check_expr(cond);
                self.check_compatible(Some(Vs3Type::Bool), cond_ty, cond.loc());
                self.loop_depth += 1;
                self.check_stmt(body);
                self.loop_depth -= 1;
            }
            Stmt::For {
                name,
                iter,
                body,
                loc,
            } => {
                let iter_ty = self.check_expr(iter);
                if iter_ty.is_some_and(|ty| {
                    !matches!(
                        ty,
                        Vs3Type::List | Vs3Type::Map | Vs3Type::String | Vs3Type::Any
                    )
                }) {
                    self.error("for expects a list, map, or string", iter.loc());
                }
                self.scopes.push(HashMap::new());
                self.define_local(
                    name,
                    Binding {
                        ty: Some(Vs3Type::Any),
                        mutable: true,
                    },
                    loc,
                );
                self.loop_depth += 1;
                self.check_stmt(body);
                self.loop_depth -= 1;
                self.scopes.pop();
            }
            Stmt::Break { loc } | Stmt::Continue { loc } => {
                if self.loop_depth == 0 {
                    self.error("loop control used outside a loop", loc);
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.check_expr(value);
                }
            }
            Stmt::Dialogue { loc, .. }
            | Stmt::Jump { loc, .. }
            | Stmt::Label { loc, .. }
            | Stmt::Choice { loc, .. }
            | Stmt::Show { loc, .. }
            | Stmt::Background { loc, .. }
            | Stmt::Music { loc, .. }
            | Stmt::Hide { loc, .. }
            | Stmt::End { loc, .. }
            | Stmt::Call { loc, .. }
            | Stmt::HostCall { loc, .. }
            | Stmt::Transition { loc, .. }
            | Stmt::Sound { loc, .. }
            | Stmt::Pause { loc, .. } => self.error(
                "narrative statement is not part of VS3; call a host service through yield instead",
                loc,
            ),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Option<Vs3Type> {
        match expr {
            Expr::Null { .. } => Some(Vs3Type::Null),
            Expr::Bool { .. } => Some(Vs3Type::Bool),
            Expr::Int { .. } => Some(Vs3Type::Int),
            Expr::Float { .. } => Some(Vs3Type::Float),
            Expr::String { .. } => Some(Vs3Type::String),
            Expr::List { elements, .. } => {
                for element in elements {
                    self.check_expr(element);
                }
                Some(Vs3Type::List)
            }
            Expr::Map { entries, .. } => {
                for (_, value) in entries {
                    self.check_expr(value);
                }
                Some(Vs3Type::Map)
            }
            Expr::Ident { name, loc } => match self.lookup(name) {
                Some(binding) => binding.ty,
                None => {
                    self.error(format!("unknown name `{name}`"), loc);
                    None
                }
            },
            Expr::Unary { op, expr, loc } => {
                let found = self.check_expr(expr);
                match op {
                    UnaryOp::Not => {
                        self.check_compatible(Some(Vs3Type::Bool), found, loc);
                        Some(Vs3Type::Bool)
                    }
                    UnaryOp::Neg => {
                        if !found.is_some_and(is_numeric_or_vector) {
                            self.require_number(found, loc);
                        }
                        found
                    }
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                loc,
            } => {
                if matches!(
                    op,
                    BinOp::Assign
                        | BinOp::AddAssign
                        | BinOp::SubAssign
                        | BinOp::MulAssign
                        | BinOp::DivAssign
                ) {
                    let left_ty = self.check_assignment_target(left);
                    let right_ty = self.check_expr(right);
                    self.check_compatible(left_ty, right_ty, loc);
                    return right_ty.or(left_ty);
                }
                let left_ty = self.check_expr(left);
                let right_ty = self.check_expr(right);
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem
                        if left_ty == Some(Vs3Type::Any) || right_ty == Some(Vs3Type::Any) =>
                    {
                        Some(Vs3Type::Any)
                    }
                    BinOp::Add => {
                        if left_ty == Some(Vs3Type::String) || right_ty == Some(Vs3Type::String) {
                            Some(Vs3Type::String)
                        } else if left_ty.is_some_and(is_vector) && left_ty == right_ty {
                            left_ty
                        } else {
                            self.require_number(left_ty, left.loc());
                            self.require_number(right_ty, right.loc());
                            numeric_result(left_ty, right_ty)
                        }
                    }
                    BinOp::Sub if left_ty.is_some_and(is_vector) && left_ty == right_ty => left_ty,
                    BinOp::Mul
                        if left_ty.is_some_and(is_vector) && right_ty.is_some_and(is_number) =>
                    {
                        left_ty
                    }
                    BinOp::Mul
                        if right_ty.is_some_and(is_vector) && left_ty.is_some_and(is_number) =>
                    {
                        right_ty
                    }
                    BinOp::Mul | BinOp::Div
                        if left_ty.is_some_and(is_vector)
                            && (left_ty == right_ty || right_ty.is_some_and(is_number)) =>
                    {
                        left_ty
                    }
                    BinOp::Mul if left_ty.is_some_and(is_matrix) && left_ty == right_ty => left_ty,
                    BinOp::Mul if left_ty == Some(Vs3Type::Quat) && left_ty == right_ty => left_ty,
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        self.require_number(left_ty, left.loc());
                        self.require_number(right_ty, right.loc());
                        numeric_result(left_ty, right_ty)
                    }
                    BinOp::Eq | BinOp::Ne => Some(Vs3Type::Bool),
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        self.require_number(left_ty, left.loc());
                        self.require_number(right_ty, right.loc());
                        Some(Vs3Type::Bool)
                    }
                    BinOp::And | BinOp::Or => {
                        self.check_compatible(Some(Vs3Type::Bool), left_ty, left.loc());
                        self.check_compatible(Some(Vs3Type::Bool), right_ty, right.loc());
                        Some(Vs3Type::Bool)
                    }
                    _ => unreachable!(),
                }
            }
            Expr::Call { callee, args, loc } => {
                let direct_name = match callee.as_ref() {
                    Expr::Ident { name, .. } => Some(name.as_str()),
                    _ => None,
                };
                if direct_name == Some("yield") && args.len() > 1 {
                    self.error("yield expects zero or one argument", loc);
                }
                let signature = direct_name.and_then(|name| self.signatures.get(name).cloned());
                let native = direct_name.and_then(lookup_native);
                if let Some(signature) = &signature {
                    if signature.len() != args.len() {
                        self.error(
                            format!(
                                "function `{}` expects {} arguments, got {}",
                                direct_name.unwrap_or_default(),
                                signature.len(),
                                args.len()
                            ),
                            loc,
                        );
                    }
                }
                if let Some(native) = native {
                    let spec = native.spec();
                    if args.len() < spec.min_args as usize || args.len() > spec.max_args as usize {
                        let expected = if spec.min_args == spec.max_args {
                            spec.min_args.to_string()
                        } else {
                            format!("{}..={}", spec.min_args, spec.max_args)
                        };
                        self.error(
                            format!(
                                "native `{}` expects {expected} arguments, got {}",
                                spec.name,
                                args.len()
                            ),
                            loc,
                        );
                    }
                }
                self.check_expr(callee);
                let mut argument_types = Vec::with_capacity(args.len());
                for (index, argument) in args.iter().enumerate() {
                    let found = self.check_expr(argument);
                    argument_types.push(found);
                    let expected = signature
                        .as_ref()
                        .and_then(|signature| signature.get(index).copied().flatten());
                    if expected.is_some() {
                        self.check_compatible(expected, found, argument.loc());
                    } else if let Some(parameter) = native.and_then(|native| {
                        let spec = native.spec();
                        spec.parameters
                            .get(index)
                            .or_else(|| spec.parameters.last())
                            .copied()
                    }) {
                        self.check_native_parameter(parameter, found, argument.loc());
                    }
                }
                native.and_then(|native| native_result_type(native, &argument_types))
            }
            Expr::Field { object, field, loc } => {
                let object_type = self.check_expr(object);
                if object_type.is_some_and(is_vector_or_quat) {
                    let max_component = match object_type {
                        Some(Vs3Type::Vec2) => 2,
                        Some(Vs3Type::Vec3) => 3,
                        Some(Vs3Type::Vec4 | Vs3Type::Quat) => 4,
                        _ => 0,
                    };
                    let component = match field.as_str() {
                        "x" => 1,
                        "y" => 2,
                        "z" => 3,
                        "w" => 4,
                        _ => 0,
                    };
                    if component == 0 || component > max_component {
                        self.error(
                            format!(
                                "type `{}` has no component `{field}`",
                                object_type.unwrap().as_str()
                            ),
                            loc,
                        );
                    }
                    Some(Vs3Type::Float)
                } else {
                    Some(Vs3Type::Any)
                }
            }
            Expr::Index { object, index, .. } => {
                let object_type = self.check_expr(object);
                self.check_expr(index);
                if object_type.is_some_and(|ty| is_vector_or_quat(ty) || is_matrix(ty)) {
                    Some(Vs3Type::Float)
                } else {
                    Some(Vs3Type::Any)
                }
            }
        }
    }

    fn check_assignment_target(&mut self, expr: &Expr) -> Option<Vs3Type> {
        match expr {
            Expr::Ident { name, loc } => match self.lookup(name) {
                Some(binding) => {
                    if !binding.mutable {
                        self.error(format!("cannot assign to immutable `{name}`"), loc);
                    }
                    binding.ty
                }
                None => {
                    self.error(format!("unknown assignment target `{name}`"), loc);
                    None
                }
            },
            Expr::Index { object, index, .. } => {
                let object_type = self.check_expr(object);
                self.check_expr(index);
                if object_type.is_some_and(|ty| is_vector_or_quat(ty) || is_matrix(ty)) {
                    self.error("mathematical values are immutable", expr.loc());
                }
                None
            }
            Expr::Field { object, .. } => {
                let object_type = self.check_expr(object);
                if object_type.is_some_and(is_vector_or_quat) {
                    self.error("vector and quaternion components are immutable", expr.loc());
                }
                None
            }
            _ => {
                self.error("invalid assignment target", expr.loc());
                None
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .or_else(|| self.globals.get(name).copied())
    }

    fn define_local(&mut self, name: &str, binding: Binding, loc: &SourceLoc) {
        let Some(scope) = self.scopes.last_mut() else {
            return;
        };
        if scope.contains_key(name) {
            self.error(format!("duplicate local `{name}`"), loc);
        } else {
            scope.insert(name.to_string(), binding);
        }
    }

    fn parse_type(&mut self, ty: Option<&str>, loc: &SourceLoc) -> Option<Vs3Type> {
        let ty = ty?;
        match Vs3Type::parse(ty) {
            Some(ty) => Some(ty),
            None => {
                self.error(format!("unknown type `{ty}`"), loc);
                None
            }
        }
    }

    fn check_compatible(
        &mut self,
        expected: Option<Vs3Type>,
        found: Option<Vs3Type>,
        loc: &SourceLoc,
    ) {
        if let (Some(expected), Some(found)) = (expected, found) {
            if !expected.accepts(found) {
                self.error(
                    format!(
                        "type mismatch: expected `{}`, found `{}`",
                        expected.as_str(),
                        found.as_str()
                    ),
                    loc,
                );
            }
        }
    }

    fn require_number(&mut self, found: Option<Vs3Type>, loc: &SourceLoc) {
        if found.is_some_and(|ty| !matches!(ty, Vs3Type::Int | Vs3Type::Float | Vs3Type::Any)) {
            self.error("expected a number", loc);
        }
    }

    fn check_native_parameter(
        &mut self,
        expected: NativeType,
        found: Option<Vs3Type>,
        loc: &SourceLoc,
    ) {
        let Some(found) = found else {
            return;
        };
        let accepted = match expected {
            NativeType::Any => true,
            NativeType::Number => is_number(found),
            NativeType::Vector => is_vector(found) || found == Vs3Type::Any,
            NativeType::Matrix => is_matrix(found) || found == Vs3Type::Any,
            other => Vs3Type::from_native(other).is_some_and(|expected| expected.accepts(found)),
        };
        if !accepted {
            let expected = match expected {
                NativeType::Number => "number".into(),
                NativeType::Vector => "vector".into(),
                NativeType::Matrix => "matrix".into(),
                other => format!("{:?}", other).to_ascii_lowercase(),
            };
            self.error(
                format!(
                    "type mismatch: expected `{expected}`, found `{}`",
                    found.as_str()
                ),
                loc,
            );
        }
    }

    fn error(&mut self, message: impl Into<String>, loc: &SourceLoc) {
        self.diagnostics.push(Vs3Diagnostic {
            message: message.into(),
            loc: loc.clone(),
        });
    }
}

fn numeric_result(left: Option<Vs3Type>, right: Option<Vs3Type>) -> Option<Vs3Type> {
    if left == Some(Vs3Type::Any) || right == Some(Vs3Type::Any) {
        Some(Vs3Type::Any)
    } else if left == Some(Vs3Type::Float) || right == Some(Vs3Type::Float) {
        Some(Vs3Type::Float)
    } else if left == Some(Vs3Type::Int) && right == Some(Vs3Type::Int) {
        Some(Vs3Type::Int)
    } else {
        None
    }
}

fn native_result_type(native: NativeId, arguments: &[Option<Vs3Type>]) -> Option<Vs3Type> {
    use NativeId as N;
    match native {
        N::Abs | N::Min | N::Max | N::Clamp => {
            if arguments
                .iter()
                .all(|argument| *argument == Some(Vs3Type::Int))
            {
                Some(Vs3Type::Int)
            } else if arguments.contains(&Some(Vs3Type::Any)) {
                Some(Vs3Type::Any)
            } else {
                Some(Vs3Type::Float)
            }
        }
        N::Normalize
        | N::Reflect
        | N::Refract
        | N::Project
        | N::VecLerp
        | N::VecMin
        | N::VecMax
        | N::ClampLength
        | N::QuadraticBezier
        | N::CubicBezier
        | N::CatmullRom
        | N::Hermite
        | N::ClosestPointSegment
        | N::MatMul
        | N::MatTranspose
        | N::MatInverse => arguments.first().copied().flatten(),
        N::TransformPoint | N::TransformVector => arguments.get(1).copied().flatten(),
        N::Cross => match arguments.first().copied().flatten() {
            Some(Vs3Type::Vec2) => Some(Vs3Type::Float),
            Some(Vs3Type::Vec3) => Some(Vs3Type::Vec3),
            _ => None,
        },
        _ => Vs3Type::from_native(native.spec().result),
    }
}

fn is_number(ty: Vs3Type) -> bool {
    matches!(ty, Vs3Type::Int | Vs3Type::Float | Vs3Type::Any)
}

fn is_vector(ty: Vs3Type) -> bool {
    matches!(ty, Vs3Type::Vec2 | Vs3Type::Vec3 | Vs3Type::Vec4)
}

fn is_matrix(ty: Vs3Type) -> bool {
    matches!(ty, Vs3Type::Mat3 | Vs3Type::Mat4)
}

fn is_vector_or_quat(ty: Vs3Type) -> bool {
    is_vector(ty) || ty == Vs3Type::Quat
}

fn is_numeric_or_vector(ty: Vs3Type) -> bool {
    is_number(ty) || is_vector_or_quat(ty)
}
