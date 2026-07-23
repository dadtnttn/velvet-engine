use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use velvet_script_ast::{Diagnostic, Expr, Item, Module, SourceLoc, Stmt};
use velvet_script_bytecode::fnv1a64;
use velvet_script_parser::parse_file;

use crate::{detect_edition, Edition, Vs3Error};

#[derive(Debug)]
pub(crate) struct BundleAst {
    pub(crate) module: Module,
    pub(crate) entrypoints: BTreeMap<String, String>,
    pub(crate) source_hash: u64,
    pub(crate) root: String,
}

#[derive(Debug, Clone)]
struct ImportEdge {
    path: String,
    alias: Option<String>,
    loc: SourceLoc,
}

#[derive(Debug)]
struct ParsedUnit {
    module: Module,
    imports: Vec<ImportEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    Function,
    State,
    Global,
}

#[derive(Debug, Clone)]
struct SymbolInfo {
    actual: String,
    kind: SymbolKind,
    exported: bool,
    loc: SourceLoc,
}

#[derive(Debug, Default)]
struct OwnerInfo {
    nominal: bool,
    explicit_exports: bool,
    aliases: BTreeMap<String, String>,
    symbols: BTreeMap<String, SymbolInfo>,
}

impl OwnerInfo {
    fn function_is_public(&self, symbol: &SymbolInfo) -> bool {
        symbol.kind == SymbolKind::Function && (!self.explicit_exports || symbol.exported)
    }
}

pub(crate) fn build<K, V, I>(root: &str, sources: I) -> Result<BundleAst, Vs3Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    let root = normalize_bundle_path("", root).map_err(|message| bundle_error(root, message))?;
    let mut source_map = BTreeMap::new();
    for (name, source) in sources {
        let original = name.into();
        let normalized = normalize_bundle_path("", &original)
            .map_err(|message| bundle_error(&original, message))?;
        if source_map
            .insert(normalized.clone(), source.into())
            .is_some()
        {
            return Err(bundle_error(&normalized, "duplicate source path"));
        }
    }

    let root_source = source_map
        .get(&root)
        .ok_or_else(|| bundle_error(&root, "root source is missing from the bundle"))?;
    if detect_edition(root_source) != Edition::Vs3 {
        return Err(Vs3Error::Edition(format!(
            "VS3 bundle root `{root}` requires `// @edition 3`"
        )));
    }

    let mut units = BTreeMap::new();
    let mut owners = BTreeMap::new();
    let mut aliases_by_owner: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    visit_source(
        &root,
        &root,
        &source_map,
        &mut units,
        &mut owners,
        &mut aliases_by_owner,
        &mut visiting,
        &mut visited,
        &mut order,
    )?;

    let mut owner_info: BTreeMap<String, OwnerInfo> = BTreeMap::new();
    for owner in owners.values() {
        owner_info
            .entry(owner.clone())
            .or_insert_with(|| OwnerInfo {
                nominal: owner != &root,
                explicit_exports: false,
                aliases: aliases_by_owner.remove(owner).unwrap_or_default(),
                symbols: BTreeMap::new(),
            });
    }

    let mut structural_diagnostics = Vec::new();
    for path in &order {
        let owner = owners
            .get(path)
            .expect("reachable source has an assigned owner");
        let info = owner_info
            .get_mut(owner)
            .expect("assigned owner has metadata");
        let unit = units.get(path).expect("reachable source was parsed");
        collect_symbols(info, &unit.module, owner, &mut structural_diagnostics);
    }
    for (owner, info) in &owner_info {
        for alias in info.aliases.keys() {
            if let Some(symbol) = info.symbols.get(alias) {
                structural_diagnostics.push(Diagnostic::error(
                    format!(
                        "module alias `{alias}` conflicts with a declaration in module `{owner}`"
                    ),
                    symbol.loc.clone(),
                ));
            }
        }
    }

    let mut merged = Module {
        file: Some(root.clone()),
        items: Vec::new(),
        diagnostics: structural_diagnostics,
    };
    for path in order {
        let owner = owners
            .get(&path)
            .expect("reachable source has an owner")
            .clone();
        let mut unit = units.remove(&path).expect("reachable source was parsed");
        merged.diagnostics.append(&mut unit.module.diagnostics);
        let mut rewritten = Vec::with_capacity(unit.module.items.len());
        for mut item in unit.module.items {
            let mut rewriter = Rewriter::new(&owner, &owner_info, &mut merged.diagnostics);
            rewriter.rewrite_item(&mut item);
            rewritten.push(item);
        }
        merged.items.extend(rewritten);
    }

    let root_info = owner_info
        .get(&root)
        .expect("root owner metadata must exist");
    let mut entrypoints = BTreeMap::new();
    for (name, symbol) in &root_info.symbols {
        if root_info.function_is_public(symbol) {
            entrypoints.insert(name.clone(), symbol.actual.clone());
        }
    }
    for (alias, target_owner) in &root_info.aliases {
        let target = owner_info
            .get(target_owner)
            .expect("module alias target metadata must exist");
        for (name, symbol) in &target.symbols {
            if target.function_is_public(symbol) {
                entrypoints.insert(format!("{alias}.{name}"), symbol.actual.clone());
            }
        }
    }

    let mut hash_input = Vec::new();
    for path in owners.keys() {
        hash_input.extend_from_slice(path.as_bytes());
        hash_input.push(0);
        hash_input.extend_from_slice(
            source_map
                .get(path)
                .expect("reachable source exists")
                .as_bytes(),
        );
        hash_input.push(0xff);
    }

    Ok(BundleAst {
        module: merged,
        entrypoints,
        source_hash: fnv1a64(&hash_input),
        root,
    })
}

pub(crate) fn load_path(path: &Path) -> Result<(String, BTreeMap<String, String>), Vs3Error> {
    let root_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let canonical_root = root_dir.canonicalize().map_err(|error| {
        bundle_error(
            &root_dir.display().to_string(),
            format!("cannot resolve root directory: {error}"),
        )
    })?;
    let root_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            bundle_error(
                &path.display().to_string(),
                "root path has no UTF-8 file name",
            )
        })?;
    let root_name = normalize_bundle_path("", root_name)
        .map_err(|message| bundle_error(&path.display().to_string(), message))?;
    let mut sources = BTreeMap::new();
    collect_filesystem_sources(&canonical_root, &root_name, &mut sources)?;
    Ok((root_name, sources))
}

fn collect_filesystem_sources(
    root_dir: &Path,
    root: &str,
    sources: &mut BTreeMap<String, String>,
) -> Result<(), Vs3Error> {
    let mut pending = vec![root.to_string()];
    while let Some(virtual_path) = pending.pop() {
        if sources.contains_key(&virtual_path) {
            continue;
        }
        let joined = root_dir.join(PathBuf::from(&virtual_path));
        let actual = joined.canonicalize().map_err(|error| {
            bundle_error(
                &virtual_path,
                format!("cannot resolve {}: {error}", joined.display()),
            )
        })?;
        if !actual.starts_with(root_dir) {
            return Err(bundle_error(
                &virtual_path,
                format!("resolved path escapes bundle root: {}", actual.display()),
            ));
        }
        let source = std::fs::read_to_string(&actual).map_err(|error| {
            bundle_error(
                &virtual_path,
                format!("cannot read {}: {error}", actual.display()),
            )
        })?;
        let parsed = parse_unit(&virtual_path, &source)?;
        for import in parsed.imports {
            let resolved =
                normalize_bundle_path(&virtual_path, &import.path).map_err(|message| {
                    bundle_error(
                        &virtual_path,
                        format!("{}:{}: {message}", import.loc.line, import.loc.column),
                    )
                })?;
            pending.push(resolved);
        }
        sources.insert(virtual_path, source);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn visit_source(
    path: &str,
    owner: &str,
    sources: &BTreeMap<String, String>,
    units: &mut BTreeMap<String, ParsedUnit>,
    owners: &mut BTreeMap<String, String>,
    aliases_by_owner: &mut BTreeMap<String, BTreeMap<String, String>>,
    visiting: &mut Vec<String>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) -> Result<(), Vs3Error> {
    if let Some(existing) = owners.get(path) {
        if existing != owner {
            return Err(bundle_error(
                path,
                format!(
                    "source belongs to both module `{existing}` and module `{owner}`; do not mix aliased and unaliased imports"
                ),
            ));
        }
    } else {
        owners.insert(path.to_string(), owner.to_string());
    }
    if let Some(index) = visiting.iter().position(|item| item == path) {
        let mut cycle = visiting[index..].to_vec();
        cycle.push(path.to_string());
        return Err(bundle_error(
            path,
            format!("cyclic import: {}", cycle.join(" -> ")),
        ));
    }
    if visited.contains(path) {
        return Ok(());
    }

    let source = sources
        .get(path)
        .ok_or_else(|| bundle_error(path, "imported source is missing from the bundle"))?;
    if detect_edition(source) == Edition::Vs2 {
        return Err(Vs3Error::Edition(format!(
            "imported source `{path}` uses obsolete edition 2"
        )));
    }
    if !units.contains_key(path) {
        units.insert(path.to_string(), parse_unit(path, source)?);
    }
    let imports = units.get(path).expect("source was parsed").imports.clone();

    visiting.push(path.to_string());
    for import in imports {
        let target = normalize_bundle_path(path, &import.path).map_err(|message| {
            bundle_error(
                path,
                format!("{}:{}: {message}", import.loc.line, import.loc.column),
            )
        })?;
        if !sources.contains_key(&target) {
            return Err(bundle_error(
                path,
                format!(
                    "{}:{}: imported source `{target}` is missing",
                    import.loc.line, import.loc.column
                ),
            ));
        }
        let target_owner = if let Some(alias) = &import.alias {
            validate_alias(alias, &import.loc)?;
            let aliases = aliases_by_owner.entry(owner.to_string()).or_default();
            if let Some(previous) = aliases.insert(alias.clone(), target.clone()) {
                if previous != target {
                    return Err(bundle_error(
                        path,
                        format!(
                            "{}:{}: module alias `{alias}` refers to both `{previous}` and `{target}`",
                            import.loc.line, import.loc.column
                        ),
                    ));
                }
            }
            target.clone()
        } else {
            owner.to_string()
        };
        visit_source(
            &target,
            &target_owner,
            sources,
            units,
            owners,
            aliases_by_owner,
            visiting,
            visited,
            order,
        )?;
    }
    visiting.pop();
    visited.insert(path.to_string());
    order.push(path.to_string());
    Ok(())
}

fn parse_unit(path: &str, source: &str) -> Result<ParsedUnit, Vs3Error> {
    let parsed = parse_file(source, Some(path))
        .map_err(|error| bundle_error(path, format!("cannot parse import graph: {error}")))?;
    let mut module = parsed.module;
    let mut imports = Vec::new();
    let mut items = Vec::with_capacity(module.items.len());
    for item in std::mem::take(&mut module.items) {
        match item {
            Item::Import { path, alias, loc } => imports.push(ImportEdge { path, alias, loc }),
            other => items.push(other),
        }
    }
    module.items = items;
    Ok(ParsedUnit { module, imports })
}

fn collect_symbols(
    owner: &mut OwnerInfo,
    module: &Module,
    owner_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in &module.items {
        match item {
            Item::Function {
                exported,
                name,
                loc,
                ..
            } => {
                owner.explicit_exports |= *exported;
                insert_symbol(
                    owner,
                    owner_name,
                    name,
                    SymbolKind::Function,
                    *exported,
                    loc,
                    diagnostics,
                );
            }
            Item::State { bindings, .. } => {
                for binding in bindings {
                    insert_symbol(
                        owner,
                        owner_name,
                        &binding.name,
                        SymbolKind::State,
                        false,
                        &binding.loc,
                        diagnostics,
                    );
                }
            }
            Item::Stmt(Stmt::Let { name, loc, .. }) | Item::Stmt(Stmt::Const { name, loc, .. }) => {
                insert_symbol(
                    owner,
                    owner_name,
                    name,
                    SymbolKind::Global,
                    false,
                    loc,
                    diagnostics,
                )
            }
            _ => {}
        }
    }
}

fn insert_symbol(
    owner: &mut OwnerInfo,
    owner_name: &str,
    name: &str,
    kind: SymbolKind,
    exported: bool,
    loc: &SourceLoc,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let actual = if owner.nominal {
        mangle(owner_name, name)
    } else {
        name.to_string()
    };
    let symbol = SymbolInfo {
        actual,
        kind,
        exported,
        loc: loc.clone(),
    };
    if let Some(previous) = owner.symbols.insert(name.to_string(), symbol) {
        diagnostics.push(Diagnostic::error(
            format!(
                "duplicate definition `{name}` in module `{owner_name}`; previous declaration at {}",
                previous.loc.display()
            ),
            loc.clone(),
        ));
    }
}

fn mangle(owner: &str, name: &str) -> String {
    format!("__vs3m_{:016x}_{name}", fnv1a64(owner.as_bytes()))
}

struct Rewriter<'a> {
    owner: &'a OwnerInfo,
    owners: &'a BTreeMap<String, OwnerInfo>,
    scopes: Vec<BTreeSet<String>>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> Rewriter<'a> {
    fn new(
        owner_name: &'a str,
        owners: &'a BTreeMap<String, OwnerInfo>,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            owner: owners
                .get(owner_name)
                .expect("rewritten module owner metadata exists"),
            owners,
            scopes: Vec::new(),
            diagnostics,
        }
    }

    fn rewrite_item(&mut self, item: &mut Item) {
        match item {
            Item::Import { .. } => unreachable!("imports are removed before rewriting"),
            Item::Function {
                name, params, body, ..
            } => {
                let original = name.clone();
                if let Some(symbol) = self.owner.symbols.get(&original) {
                    *name = symbol.actual.clone();
                }
                self.scopes
                    .push(params.iter().map(|param| param.name.clone()).collect());
                self.rewrite_stmt_list(body);
                self.scopes.pop();
            }
            Item::State { bindings, .. } => {
                for binding in bindings {
                    self.rewrite_expr(&mut binding.init);
                    if let Some(symbol) = self.owner.symbols.get(&binding.name) {
                        binding.name = symbol.actual.clone();
                    }
                }
            }
            Item::Stmt(stmt) => self.rewrite_top_level_stmt(stmt),
            Item::Character { fields, .. } => {
                for (_, value) in fields {
                    self.rewrite_expr(value);
                }
            }
            Item::Scene { body, .. } => self.rewrite_stmt_list(body),
            Item::Screen {
                properties,
                buttons,
                ..
            } => {
                for property in properties {
                    self.rewrite_expr(&mut property.value);
                }
                for button in buttons {
                    for property in &mut button.properties {
                        self.rewrite_expr(&mut property.value);
                    }
                }
            }
        }
    }

    fn rewrite_top_level_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Let { name, init, .. } | Stmt::Const { name, init, .. } => {
                self.rewrite_expr(init);
                if let Some(symbol) = self.owner.symbols.get(name) {
                    *name = symbol.actual.clone();
                }
            }
            other => self.rewrite_stmt(other),
        }
    }

    fn rewrite_stmt_list(&mut self, body: &mut [Stmt]) {
        for stmt in body {
            self.rewrite_stmt(stmt);
        }
    }

    fn rewrite_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Expr { expr, .. } => self.rewrite_expr(expr),
            Stmt::Let { name, init, .. } | Stmt::Const { name, init, .. } => {
                self.rewrite_expr(init);
                self.define_local(name.clone());
            }
            Stmt::Block { body, .. } => {
                self.scopes.push(BTreeSet::new());
                self.rewrite_stmt_list(body);
                self.scopes.pop();
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                self.rewrite_expr(cond);
                self.rewrite_stmt(then_body);
                if let Some(else_body) = else_body {
                    self.rewrite_stmt(else_body);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.rewrite_expr(cond);
                self.rewrite_stmt(body);
            }
            Stmt::For {
                name, iter, body, ..
            } => {
                self.rewrite_expr(iter);
                self.scopes.push(BTreeSet::from([name.clone()]));
                self.rewrite_stmt(body);
                self.scopes.pop();
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.rewrite_expr(value);
                }
            }
            Stmt::Choice { options, .. } => {
                for option in options {
                    self.scopes.push(BTreeSet::new());
                    self.rewrite_stmt_list(&mut option.body);
                    self.scopes.pop();
                }
            }
            Stmt::HostCall { args, .. } => {
                for (_, value) in args {
                    self.rewrite_expr(value);
                }
            }
            Stmt::Dialogue { .. }
            | Stmt::Jump { .. }
            | Stmt::Label { .. }
            | Stmt::Show { .. }
            | Stmt::Background { .. }
            | Stmt::Music { .. }
            | Stmt::Hide { .. }
            | Stmt::End { .. }
            | Stmt::Call { .. }
            | Stmt::Transition { .. }
            | Stmt::Sound { .. }
            | Stmt::Pause { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }

    fn rewrite_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Ident { name, loc } => {
                if self.is_local(name) {
                    return;
                }
                if let Some(symbol) = self.owner.symbols.get(name) {
                    *name = symbol.actual.clone();
                } else if self.owner.aliases.contains_key(name) {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "module alias `{name}` is only valid in a call such as `{name}.function(...)`"
                        ),
                        loc.clone(),
                    ));
                }
            }
            Expr::List { elements, .. } => {
                for element in elements {
                    self.rewrite_expr(element);
                }
            }
            Expr::Map { entries, .. } => {
                for (_, value) in entries {
                    self.rewrite_expr(value);
                }
            }
            Expr::Unary { expr, .. } => self.rewrite_expr(expr),
            Expr::Binary { left, right, .. } => {
                self.rewrite_expr(left);
                self.rewrite_expr(right);
            }
            Expr::Call { callee, args, .. } => {
                for argument in args {
                    self.rewrite_expr(argument);
                }
                let module_call = match callee.as_ref() {
                    Expr::Field { object, field, loc } => match object.as_ref() {
                        Expr::Ident { name: alias, .. }
                            if !self.is_local(alias) && self.owner.aliases.contains_key(alias) =>
                        {
                            Some((alias.clone(), field.clone(), loc.clone()))
                        }
                        _ => None,
                    },
                    _ => None,
                };
                if let Some((alias, function, loc)) = module_call {
                    if let Some(actual) = self.resolve_module_function(&alias, &function, &loc) {
                        **callee = Expr::Ident { name: actual, loc };
                    }
                } else {
                    self.rewrite_expr(callee);
                }
            }
            Expr::Field { object, field, loc } => {
                let module_alias = match object.as_ref() {
                    Expr::Ident { name, .. }
                        if !self.is_local(name) && self.owner.aliases.contains_key(name) =>
                    {
                        Some(name.clone())
                    }
                    _ => None,
                };
                if let Some(alias) = module_alias {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "module member `{alias}.{field}` cannot be used as a value; call an exported function instead"
                        ),
                        loc.clone(),
                    ));
                } else {
                    self.rewrite_expr(object);
                }
            }
            Expr::Index { object, index, .. } => {
                self.rewrite_expr(object);
                self.rewrite_expr(index);
            }
            Expr::Null { .. }
            | Expr::Bool { .. }
            | Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::String { .. } => {}
        }
    }

    fn resolve_module_function(
        &mut self,
        alias: &str,
        function: &str,
        loc: &SourceLoc,
    ) -> Option<String> {
        let target_owner = self
            .owner
            .aliases
            .get(alias)
            .expect("known alias has a target owner");
        let target = self
            .owners
            .get(target_owner)
            .expect("alias target owner metadata exists");
        match target.symbols.get(function) {
            Some(symbol) if target.function_is_public(symbol) => Some(symbol.actual.clone()),
            Some(symbol) if symbol.kind == SymbolKind::Function => {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "function `{alias}.{function}` is private; declare `export function {function}(...)` in module `{target_owner}`"
                    ),
                    loc.clone(),
                ));
                None
            }
            Some(_) => {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "module state `{alias}.{function}` is private; expose a function in module `{target_owner}`"
                    ),
                    loc.clone(),
                ));
                None
            }
            None => {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "module `{alias}` has no function `{function}` (imported from `{target_owner}`)"
                    ),
                    loc.clone(),
                ));
                None
            }
        }
    }

    fn define_local(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name);
        }
    }

    fn is_local(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

fn validate_alias(alias: &str, loc: &SourceLoc) -> Result<(), Vs3Error> {
    let valid = !alias.is_empty()
        && alias.len() <= 128
        && alias.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphanumeric() && (index > 0 || !character.is_ascii_digit())
        });
    if valid && !is_reserved(alias) {
        Ok(())
    } else {
        Err(bundle_error(
            loc.file.as_deref().unwrap_or("<bundle>"),
            format!(
                "{}:{}: invalid module alias `{alias}`",
                loc.line, loc.column
            ),
        ))
    }
}

fn is_reserved(name: &str) -> bool {
    matches!(
        name,
        "as" | "import"
            | "export"
            | "function"
            | "fn"
            | "state"
            | "let"
            | "const"
            | "if"
            | "else"
            | "while"
            | "for"
            | "return"
            | "break"
            | "continue"
            | "true"
            | "false"
            | "null"
    )
}

fn normalize_bundle_path(current: &str, target: &str) -> Result<String, String> {
    let target = target.replace('\\', "/");
    if target.is_empty() || target.starts_with('/') || target.contains(':') {
        return Err(format!("invalid relative import path `{target}`"));
    }
    let mut parts = Vec::new();
    if let Some((parent, _)) = current.rsplit_once('/') {
        parts.extend(
            parent
                .split('/')
                .filter(|part| !part.is_empty())
                .map(str::to_string),
        );
    }
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("import path escapes the bundle root: `{target}`"));
                }
            }
            value if value.contains('\0') => return Err("import path contains NUL".into()),
            value => parts.push(value.to_string()),
        }
    }
    if parts.is_empty() {
        return Err(format!("invalid relative import path `{target}`"));
    }
    let normalized = parts.join("/");
    if normalized.len() > 512 {
        return Err("import path exceeds 512 bytes".into());
    }
    Ok(normalized)
}

fn bundle_error(path: &str, message: impl Into<String>) -> Vs3Error {
    Vs3Error::Bundle {
        path: path.to_string(),
        message: message.into(),
    }
}
