//! # velvet-script-bytecode
//!
//! Opcode definitions, constants, compiled modules, line tables, and
//! disassembly helpers for the Velvet Script VM.

#![deny(missing_docs)]

use std::collections::HashMap;
use std::fmt::Write as _;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Bytecode format version carried in module metadata.
pub const BYTECODE_VERSION: u16 = 1;

/// Native host function identifiers shared by compiler and VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum NativeId {
    /// `print(…)`
    Print = 0,
    /// `abs(x)`
    Abs = 1,
    /// `min(a, b)`
    Min = 2,
    /// `max(a, b)`
    Max = 3,
    /// `floor(x)`
    Floor = 4,
    /// `ceil(x)`
    Ceil = 5,
    /// `clamp(x, lo, hi)`
    Clamp = 6,
    /// `len(x)`
    Len = 7,
    /// `concat(…)`
    Concat = 8,
    /// `str(x)`
    Str = 9,
}

impl NativeId {
    /// From raw id.
    pub fn from_u16(id: u16) -> Option<Self> {
        Some(match id {
            0 => Self::Print,
            1 => Self::Abs,
            2 => Self::Min,
            3 => Self::Max,
            4 => Self::Floor,
            5 => Self::Ceil,
            6 => Self::Clamp,
            7 => Self::Len,
            8 => Self::Concat,
            9 => Self::Str,
            _ => return None,
        })
    }

    /// Raw id.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Script-visible name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Print => "print",
            Self::Abs => "abs",
            Self::Min => "min",
            Self::Max => "max",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Clamp => "clamp",
            Self::Len => "len",
            Self::Concat => "concat",
            Self::Str => "str",
        }
    }

    /// All natives in id order.
    pub fn all() -> &'static [NativeId] {
        &[
            Self::Print,
            Self::Abs,
            Self::Min,
            Self::Max,
            Self::Floor,
            Self::Ceil,
            Self::Clamp,
            Self::Len,
            Self::Concat,
            Self::Str,
        ]
    }
}

/// Resolve a script identifier to a native id.
pub fn lookup_native(name: &str) -> Option<NativeId> {
    match name {
        "print" => Some(NativeId::Print),
        "abs" => Some(NativeId::Abs),
        "min" => Some(NativeId::Min),
        "max" => Some(NativeId::Max),
        "floor" => Some(NativeId::Floor),
        "ceil" => Some(NativeId::Ceil),
        "clamp" => Some(NativeId::Clamp),
        "len" => Some(NativeId::Len),
        "concat" => Some(NativeId::Concat),
        "str" => Some(NativeId::Str),
        _ => None,
    }
}

/// Bytecode opcodes (u8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Op {
    /// Push constant by u16 index.
    Constant = 1,
    /// Push null.
    Null = 2,
    /// Push true.
    True = 3,
    /// Push false.
    False = 4,
    /// Pop top.
    Pop = 5,
    /// Define global from top (name index u16).
    DefineGlobal = 6,
    /// Get global (name index u16).
    GetGlobal = 7,
    /// Set global (name index u16).
    SetGlobal = 8,
    /// Get local slot u8.
    GetLocal = 9,
    /// Set local slot u8.
    SetLocal = 10,
    /// Add.
    Add = 11,
    /// Subtract.
    Sub = 12,
    /// Multiply.
    Mul = 13,
    /// Divide.
    Div = 14,
    /// Remainder.
    Rem = 15,
    /// Negate.
    Neg = 16,
    /// Not.
    Not = 17,
    /// Equal.
    Eq = 18,
    /// Not equal.
    Ne = 19,
    /// Less.
    Lt = 20,
    /// Less equal.
    Le = 21,
    /// Greater.
    Gt = 22,
    /// Greater equal.
    Ge = 23,
    /// Jump forward u16 bytes.
    Jump = 24,
    /// Jump if false u16, keeps value.
    JumpIfFalse = 25,
    /// Jump if true u16.
    JumpIfTrue = 26,
    /// Jump backward u16.
    Loop = 27,
    /// Call with u8 argc.
    Call = 28,
    /// Return top (or null).
    Return = 29,
    /// Build list with u16 count.
    MakeList = 30,
    /// Print top (debug host).
    Print = 31,
    /// Halt VM.
    Halt = 32,
    /// Duplicate top.
    Dup = 33,
    /// Call native host function: u16 native_id, u8 argc.
    NativeCall = 34,
    /// Yield top value to host / coroutine (cooperative).
    Yield = 35,
    /// Index get: stack `[.., list|map|str, index]` → value.
    GetIndex = 36,
    /// Index set: stack `[.., container, index, value]` → value (also stores).
    SetIndex = 37,
    /// Length of string or list (pops one, pushes int).
    Len = 38,
    /// Build map with u16 entry count (2*count stack values: k,v pairs).
    MakeMap = 39,
}

impl Op {
    /// From raw byte.
    pub fn from_u8(b: u8) -> Option<Self> {
        Some(match b {
            1 => Self::Constant,
            2 => Self::Null,
            3 => Self::True,
            4 => Self::False,
            5 => Self::Pop,
            6 => Self::DefineGlobal,
            7 => Self::GetGlobal,
            8 => Self::SetGlobal,
            9 => Self::GetLocal,
            10 => Self::SetLocal,
            11 => Self::Add,
            12 => Self::Sub,
            13 => Self::Mul,
            14 => Self::Div,
            15 => Self::Rem,
            16 => Self::Neg,
            17 => Self::Not,
            18 => Self::Eq,
            19 => Self::Ne,
            20 => Self::Lt,
            21 => Self::Le,
            22 => Self::Gt,
            23 => Self::Ge,
            24 => Self::Jump,
            25 => Self::JumpIfFalse,
            26 => Self::JumpIfTrue,
            27 => Self::Loop,
            28 => Self::Call,
            29 => Self::Return,
            30 => Self::MakeList,
            31 => Self::Print,
            32 => Self::Halt,
            33 => Self::Dup,
            34 => Self::NativeCall,
            35 => Self::Yield,
            36 => Self::GetIndex,
            37 => Self::SetIndex,
            38 => Self::Len,
            39 => Self::MakeMap,
            _ => return None,
        })
    }

    /// Encode as raw byte.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Mnemonic for disassembly.
    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Constant => "CONSTANT",
            Self::Null => "NULL",
            Self::True => "TRUE",
            Self::False => "FALSE",
            Self::Pop => "POP",
            Self::DefineGlobal => "DEFINE_GLOBAL",
            Self::GetGlobal => "GET_GLOBAL",
            Self::SetGlobal => "SET_GLOBAL",
            Self::GetLocal => "GET_LOCAL",
            Self::SetLocal => "SET_LOCAL",
            Self::Add => "ADD",
            Self::Sub => "SUB",
            Self::Mul => "MUL",
            Self::Div => "DIV",
            Self::Rem => "REM",
            Self::Neg => "NEG",
            Self::Not => "NOT",
            Self::Eq => "EQ",
            Self::Ne => "NE",
            Self::Lt => "LT",
            Self::Le => "LE",
            Self::Gt => "GT",
            Self::Ge => "GE",
            Self::Jump => "JUMP",
            Self::JumpIfFalse => "JUMP_IF_FALSE",
            Self::JumpIfTrue => "JUMP_IF_TRUE",
            Self::Loop => "LOOP",
            Self::Call => "CALL",
            Self::Return => "RETURN",
            Self::MakeList => "MAKE_LIST",
            Self::Print => "PRINT",
            Self::Halt => "HALT",
            Self::Dup => "DUP",
            Self::NativeCall => "NATIVE_CALL",
            Self::Yield => "YIELD",
            Self::GetIndex => "GET_INDEX",
            Self::SetIndex => "SET_INDEX",
            Self::Len => "LEN",
            Self::MakeMap => "MAKE_MAP",
        }
    }

    /// Operand size in bytes following the opcode.
    pub fn operand_len(self) -> usize {
        match self {
            Self::Constant
            | Self::DefineGlobal
            | Self::GetGlobal
            | Self::SetGlobal
            | Self::Jump
            | Self::JumpIfFalse
            | Self::JumpIfTrue
            | Self::Loop
            | Self::MakeList
            | Self::MakeMap => 2,
            Self::GetLocal | Self::SetLocal | Self::Call => 1,
            Self::NativeCall => 3, // u16 id + u8 argc
            _ => 0,
        }
    }
}

/// Constant pool values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Constant {
    /// Null.
    Null,
    /// Bool.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// String.
    String(String),
    /// Function index into module.functions.
    Function(u16),
    /// Native host function id.
    Native(u16),
}

/// Source mapping for a bytecode offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SourceMapEntry {
    /// Bytecode offset.
    pub offset: u32,
    /// Line.
    pub line: u32,
    /// Column.
    pub column: u32,
}

/// Compact line-table row: bytecode range mapped to one source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LineTableEntry {
    /// Inclusive start offset.
    pub start: u32,
    /// Exclusive end offset.
    pub end: u32,
    /// Source line (1-based).
    pub line: u32,
    /// Source column (1-based) at range start.
    pub column: u32,
}

/// Module-level metadata for tooling and versioning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModuleMetadata {
    /// Bytecode format version.
    pub version: u16,
    /// Optional source path.
    pub source_path: Option<String>,
    /// Optional content hash (FNV-1a 64 of source bytes when known).
    pub source_hash: Option<u64>,
    /// Free-form compiler / tool stamp.
    pub compiler: Option<String>,
}

impl ModuleMetadata {
    /// Metadata for the current bytecode format.
    pub fn current() -> Self {
        Self {
            version: BYTECODE_VERSION,
            source_path: None,
            source_hash: None,
            compiler: Some("velvet-script-compiler".into()),
        }
    }

    /// Attach source path and optional hash.
    pub fn with_source(mut self, path: impl Into<String>, hash: Option<u64>) -> Self {
        self.source_path = Some(path.into());
        self.source_hash = hash;
        self
    }
}

/// FNV-1a 64-bit hash of bytes (stable, no deps).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut h = OFFSET;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// A compiled function / chunk.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Chunk {
    /// Function name.
    pub name: String,
    /// Arity.
    pub arity: u8,
    /// Local slot count (including params).
    pub locals: u8,
    /// Bytecode.
    pub code: Vec<u8>,
    /// Constants.
    pub constants: Vec<Constant>,
    /// Source map (sparse).
    pub source_map: Vec<SourceMapEntry>,
}

impl Chunk {
    /// Create empty named chunk.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arity: 0,
            locals: 0,
            code: Vec::new(),
            constants: Vec::new(),
            source_map: Vec::new(),
        }
    }

    /// Emit opcode.
    pub fn emit_op(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    /// Emit u8.
    pub fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    /// Emit u16 little-endian.
    pub fn emit_u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// Current code length.
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// Add constant; return index.
    pub fn add_constant(&mut self, c: Constant) -> u16 {
        // Intern simple duplicates for strings/ints.
        for (i, existing) in self.constants.iter().enumerate() {
            if existing == &c {
                return i as u16;
            }
        }
        let idx = self.constants.len() as u16;
        self.constants.push(c);
        idx
    }

    /// Emit constant op.
    pub fn emit_constant(&mut self, c: Constant) {
        let idx = self.add_constant(c);
        self.emit_op(Op::Constant);
        self.emit_u16(idx);
    }

    /// Emit `NativeCall` with native id and argc.
    pub fn emit_native_call(&mut self, native_id: u16, argc: u8) {
        self.emit_op(Op::NativeCall);
        self.emit_u16(native_id);
        self.emit_u8(argc);
    }

    /// Patch u16 at offset.
    pub fn patch_u16(&mut self, offset: usize, value: u16) {
        let bytes = value.to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    /// Record source location at current offset.
    pub fn map_source(&mut self, line: u32, column: u32) {
        let offset = self.code.len() as u32;
        if let Some(last) = self.source_map.last() {
            if last.offset == offset {
                return;
            }
        }
        self.source_map.push(SourceMapEntry {
            offset,
            line,
            column,
        });
    }

    /// Lookup line/col for bytecode offset.
    pub fn location_at(&self, offset: usize) -> Option<(u32, u32)> {
        let mut best = None;
        for e in &self.source_map {
            if e.offset as usize <= offset {
                best = Some((e.line, e.column));
            } else {
                break;
            }
        }
        best
    }

    /// Build a compact line table covering the whole code buffer.
    ///
    /// Consecutive offsets that share the same line are merged.
    pub fn line_table(&self) -> Vec<LineTableEntry> {
        if self.code.is_empty() {
            return Vec::new();
        }
        let mut table = Vec::new();
        let mut cursor = 0u32;
        let mut map_i = 0usize;
        let mut cur_line = 1u32;
        let mut cur_col = 1u32;
        // Apply any entry at offset 0.
        while map_i < self.source_map.len() && self.source_map[map_i].offset == 0 {
            cur_line = self.source_map[map_i].line;
            cur_col = self.source_map[map_i].column;
            map_i += 1;
        }
        let code_len = self.code.len() as u32;
        while cursor < code_len {
            let next_change = self
                .source_map
                .get(map_i)
                .map(|e| e.offset)
                .unwrap_or(code_len);
            let end = next_change.min(code_len);
            if end > cursor {
                table.push(LineTableEntry {
                    start: cursor,
                    end,
                    line: cur_line,
                    column: cur_col,
                });
            }
            cursor = end;
            if map_i < self.source_map.len() && self.source_map[map_i].offset == cursor {
                cur_line = self.source_map[map_i].line;
                cur_col = self.source_map[map_i].column;
                map_i += 1;
            } else if cursor < code_len && next_change == cursor {
                // advance past duplicate
                map_i += 1;
            }
            // If stuck (no progress), advance one byte.
            if end == cursor && cursor < code_len {
                // consume remaining map entries at this offset
                while map_i < self.source_map.len() && self.source_map[map_i].offset == cursor {
                    cur_line = self.source_map[map_i].line;
                    cur_col = self.source_map[map_i].column;
                    map_i += 1;
                }
                let next = self
                    .source_map
                    .get(map_i)
                    .map(|e| e.offset)
                    .unwrap_or(code_len);
                if next > cursor {
                    table.push(LineTableEntry {
                        start: cursor,
                        end: next,
                        line: cur_line,
                        column: cur_col,
                    });
                    cursor = next;
                } else {
                    break;
                }
            }
        }
        // Merge adjacent same-line ranges.
        let mut merged: Vec<LineTableEntry> = Vec::new();
        for e in table {
            if let Some(last) = merged.last_mut() {
                if last.line == e.line && last.end == e.start {
                    last.end = e.end;
                    continue;
                }
            }
            merged.push(e);
        }
        merged
    }

    /// Line number for each bytecode byte (dense). Length equals `code.len()`.
    pub fn dense_line_table(&self) -> Vec<u32> {
        let mut lines = vec![1u32; self.code.len()];
        let table = self.line_table();
        for e in table {
            let start = e.start as usize;
            let end = (e.end as usize).min(lines.len());
            for slot in lines.iter_mut().take(end).skip(start) {
                *slot = e.line;
            }
        }
        lines
    }

    /// Encode chunk header + code + constants into a simple binary blob.
    ///
    /// Layout (little-endian):
    /// - magic `VCHK` (4)
    /// - arity u8, locals u8, name_len u16, name bytes
    /// - const_count u16, then each constant
    /// - code_len u32, code bytes
    /// - map_count u16, then (offset u32, line u32, column u32)*
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"VCHK");
        buf.push(self.arity);
        buf.push(self.locals);
        let name_bytes = self.name.as_bytes();
        buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&(self.constants.len() as u16).to_le_bytes());
        for c in &self.constants {
            encode_constant(c, &mut buf);
        }
        buf.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.code);
        buf.extend_from_slice(&(self.source_map.len() as u16).to_le_bytes());
        for e in &self.source_map {
            buf.extend_from_slice(&e.offset.to_le_bytes());
            buf.extend_from_slice(&e.line.to_le_bytes());
            buf.extend_from_slice(&e.column.to_le_bytes());
        }
        buf
    }

    /// Decode a chunk previously produced by [`Chunk::encode`].
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut i = 0usize;
        let read = |i: &mut usize, n: usize| -> Result<&[u8], String> {
            if *i + n > bytes.len() {
                return Err("truncated chunk blob".into());
            }
            let s = &bytes[*i..*i + n];
            *i += n;
            Ok(s)
        };
        if read(&mut i, 4)? != b"VCHK" {
            return Err("bad chunk magic".into());
        }
        let arity = read(&mut i, 1)?[0];
        let locals = read(&mut i, 1)?[0];
        let name_len = u16::from_le_bytes(read(&mut i, 2)?.try_into().unwrap()) as usize;
        let name = String::from_utf8(read(&mut i, name_len)?.to_vec())
            .map_err(|_| "invalid chunk name utf8".to_string())?;
        let const_count = u16::from_le_bytes(read(&mut i, 2)?.try_into().unwrap()) as usize;
        let mut constants = Vec::with_capacity(const_count);
        for _ in 0..const_count {
            constants.push(decode_constant(bytes, &mut i)?);
        }
        let code_len = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap()) as usize;
        let code = read(&mut i, code_len)?.to_vec();
        let map_count = u16::from_le_bytes(read(&mut i, 2)?.try_into().unwrap()) as usize;
        let mut source_map = Vec::with_capacity(map_count);
        for _ in 0..map_count {
            let offset = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap());
            let line = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap());
            let column = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap());
            source_map.push(SourceMapEntry {
                offset,
                line,
                column,
            });
        }
        Ok(Self {
            name,
            arity,
            locals,
            code,
            constants,
            source_map,
        })
    }
}

fn encode_constant(c: &Constant, buf: &mut Vec<u8>) {
    match c {
        Constant::Null => buf.push(0),
        Constant::Bool(b) => {
            buf.push(1);
            buf.push(u8::from(*b));
        }
        Constant::Int(i) => {
            buf.push(2);
            buf.extend_from_slice(&i.to_le_bytes());
        }
        Constant::Float(f) => {
            buf.push(3);
            buf.extend_from_slice(&f.to_bits().to_le_bytes());
        }
        Constant::String(s) => {
            buf.push(4);
            let b = s.as_bytes();
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        Constant::Function(idx) => {
            buf.push(5);
            buf.extend_from_slice(&idx.to_le_bytes());
        }
        Constant::Native(idx) => {
            buf.push(6);
            buf.extend_from_slice(&idx.to_le_bytes());
        }
    }
}

fn decode_constant(bytes: &[u8], i: &mut usize) -> Result<Constant, String> {
    if *i >= bytes.len() {
        return Err("truncated constant".into());
    }
    let tag = bytes[*i];
    *i += 1;
    match tag {
        0 => Ok(Constant::Null),
        1 => {
            if *i >= bytes.len() {
                return Err("truncated bool const".into());
            }
            let b = bytes[*i] != 0;
            *i += 1;
            Ok(Constant::Bool(b))
        }
        2 => {
            if *i + 8 > bytes.len() {
                return Err("truncated int const".into());
            }
            let v = i64::from_le_bytes(bytes[*i..*i + 8].try_into().unwrap());
            *i += 8;
            Ok(Constant::Int(v))
        }
        3 => {
            if *i + 8 > bytes.len() {
                return Err("truncated float const".into());
            }
            let bits = u64::from_le_bytes(bytes[*i..*i + 8].try_into().unwrap());
            *i += 8;
            Ok(Constant::Float(f64::from_bits(bits)))
        }
        4 => {
            if *i + 4 > bytes.len() {
                return Err("truncated string len".into());
            }
            let len = u32::from_le_bytes(bytes[*i..*i + 4].try_into().unwrap()) as usize;
            *i += 4;
            if *i + len > bytes.len() {
                return Err("truncated string bytes".into());
            }
            let s = String::from_utf8(bytes[*i..*i + len].to_vec())
                .map_err(|_| "invalid string utf8".to_string())?;
            *i += len;
            Ok(Constant::String(s))
        }
        5 => {
            if *i + 2 > bytes.len() {
                return Err("truncated function const".into());
            }
            let v = u16::from_le_bytes(bytes[*i..*i + 2].try_into().unwrap());
            *i += 2;
            Ok(Constant::Function(v))
        }
        6 => {
            if *i + 2 > bytes.len() {
                return Err("truncated native const".into());
            }
            let v = u16::from_le_bytes(bytes[*i..*i + 2].try_into().unwrap());
            *i += 2;
            Ok(Constant::Native(v))
        }
        other => Err(format!("unknown constant tag {other}")),
    }
}

/// Compiled module.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BytecodeModule {
    /// Optional file name.
    pub file: Option<String>,
    /// Module metadata (version, hash, tool stamp).
    pub metadata: ModuleMetadata,
    /// Functions (index 0 is often `<script>` main).
    pub functions: Vec<Chunk>,
    /// Global names in definition order.
    pub globals: Vec<String>,
    /// Name → function index for exports.
    pub exports: HashMap<String, u16>,
}

impl BytecodeModule {
    /// Create empty module.
    pub fn new() -> Self {
        Self {
            metadata: ModuleMetadata::current(),
            ..Default::default()
        }
    }

    /// Main script chunk index if present.
    pub fn main_index(&self) -> Option<u16> {
        self.exports
            .get("<script>")
            .copied()
            .or(if self.functions.is_empty() {
                None
            } else {
                Some(0)
            })
    }

    /// Lookup export by name.
    pub fn export(&self, name: &str) -> Option<u16> {
        self.exports.get(name).copied()
    }
}

/// Pretty-print a single chunk as a disassembly listing.
pub fn disassemble_chunk(chunk: &Chunk) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== {} (arity={}, locals={}) ==",
        chunk.name, chunk.arity, chunk.locals
    );
    if !chunk.constants.is_empty() {
        let _ = writeln!(out, "; constants:");
        for (i, c) in chunk.constants.iter().enumerate() {
            let _ = writeln!(out, ";   [{i}] {}", format_constant(c));
        }
    }
    let mut ip = 0usize;
    while ip < chunk.code.len() {
        let line_note = chunk
            .location_at(ip)
            .map(|(l, c)| format!(" ; L{l}:{c}"))
            .unwrap_or_default();
        let start = ip;
        let op_byte = chunk.code[ip];
        ip += 1;
        let Some(op) = Op::from_u8(op_byte) else {
            let _ = writeln!(out, "{start:04X}  ??? {op_byte:02X}{line_note}");
            continue;
        };
        match op {
            Op::Constant => {
                let idx = read_u16(&chunk.code, &mut ip);
                let pretty = chunk
                    .constants
                    .get(idx as usize)
                    .map(format_constant)
                    .unwrap_or_else(|| "<bad>".into());
                let _ = writeln!(
                    out,
                    "{start:04X}  {} {}  ({}){line_note}",
                    op.mnemonic(),
                    idx,
                    pretty
                );
            }
            Op::DefineGlobal | Op::GetGlobal | Op::SetGlobal => {
                let idx = read_u16(&chunk.code, &mut ip);
                let _ = writeln!(out, "{start:04X}  {} {}{line_note}", op.mnemonic(), idx);
            }
            Op::Jump | Op::JumpIfFalse | Op::JumpIfTrue | Op::Loop | Op::MakeList | Op::MakeMap => {
                let off = read_u16(&chunk.code, &mut ip);
                let _ = writeln!(out, "{start:04X}  {} {}{line_note}", op.mnemonic(), off);
            }
            Op::GetLocal | Op::SetLocal | Op::Call => {
                let slot = chunk.code.get(ip).copied().unwrap_or(0);
                ip += 1;
                let _ = writeln!(out, "{start:04X}  {} {}{line_note}", op.mnemonic(), slot);
            }
            Op::NativeCall => {
                let id = read_u16(&chunk.code, &mut ip);
                let argc = chunk.code.get(ip).copied().unwrap_or(0);
                ip += 1;
                let _ = writeln!(
                    out,
                    "{start:04X}  {} id={} argc={}{line_note}",
                    op.mnemonic(),
                    id,
                    argc
                );
            }
            _ => {
                let _ = writeln!(out, "{start:04X}  {}{line_note}", op.mnemonic());
            }
        }
    }
    out
}

/// Disassemble every function in a module.
pub fn disassemble_module(module: &BytecodeModule) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "; module file={:?} version={}",
        module.file, module.metadata.version
    );
    if let Some(h) = module.metadata.source_hash {
        let _ = writeln!(out, "; source_hash={h:#x}");
    }
    let _ = writeln!(out, "; globals: {:?}", module.globals);
    let _ = writeln!(
        out,
        "; exports: {:?}",
        module.exports.keys().collect::<Vec<_>>()
    );
    for (i, chunk) in module.functions.iter().enumerate() {
        let _ = writeln!(out, "\n; --- function [{i}] ---");
        out.push_str(&disassemble_chunk(chunk));
    }
    out
}

fn read_u16(code: &[u8], ip: &mut usize) -> u16 {
    let lo = code.get(*ip).copied().unwrap_or(0) as u16;
    let hi = code.get(*ip + 1).copied().unwrap_or(0) as u16;
    *ip += 2;
    lo | (hi << 8)
}

fn format_constant(c: &Constant) -> String {
    match c {
        Constant::Null => "null".into(),
        Constant::Bool(b) => b.to_string(),
        Constant::Int(i) => i.to_string(),
        Constant::Float(f) => f.to_string(),
        Constant::String(s) => format!("\"{s}\""),
        Constant::Function(i) => format!("fn#{i}"),
        Constant::Native(i) => format!("native#{i}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_and_patch() {
        let mut c = Chunk::new("test");
        c.emit_op(Op::Jump);
        let at = c.len();
        c.emit_u16(0);
        c.patch_u16(at, 10);
        assert_eq!(c.code[at], 10);
        assert_eq!(c.code[at + 1], 0);
    }

    #[test]
    fn constant_intern() {
        let mut c = Chunk::new("t");
        let a = c.add_constant(Constant::Int(1));
        let b = c.add_constant(Constant::Int(1));
        assert_eq!(a, b);
        assert_eq!(c.constants.len(), 1);
    }

    #[test]
    fn op_roundtrip_all() {
        for b in 1u8..=39 {
            let op = Op::from_u8(b).expect("op defined");
            assert_eq!(op.to_u8(), b);
            assert!(!op.mnemonic().is_empty());
        }
        assert!(Op::from_u8(0).is_none());
        assert!(Op::from_u8(40).is_none());
    }

    #[test]
    fn encode_decode_chunk() {
        let mut c = Chunk::new("fold");
        c.arity = 2;
        c.locals = 3;
        c.map_source(1, 1);
        c.emit_constant(Constant::Int(42));
        c.emit_constant(Constant::String("hi".into()));
        c.emit_constant(Constant::Native(3));
        c.emit_op(Op::Add);
        c.emit_native_call(1, 2);
        c.map_source(2, 4);
        c.emit_op(Op::Return);
        let bytes = c.encode();
        let d = Chunk::decode(&bytes).unwrap();
        assert_eq!(d.name, "fold");
        assert_eq!(d.arity, 2);
        assert_eq!(d.locals, 3);
        assert_eq!(d.constants, c.constants);
        assert_eq!(d.code, c.code);
        assert_eq!(d.source_map, c.source_map);
    }

    #[test]
    fn disassemble_contains_mnemonics() {
        let mut c = Chunk::new("main");
        c.emit_constant(Constant::Int(1));
        c.emit_op(Op::Print);
        c.emit_op(Op::Return);
        let text = disassemble_chunk(&c);
        assert!(text.contains("CONSTANT"));
        assert!(text.contains("PRINT"));
        assert!(text.contains("RETURN"));
        assert!(text.contains("== main"));
    }

    #[test]
    fn line_table_merges_ranges() {
        let mut c = Chunk::new("lt");
        c.map_source(10, 1);
        c.emit_op(Op::Null);
        c.emit_op(Op::Pop);
        c.map_source(11, 2);
        c.emit_op(Op::True);
        let table = c.line_table();
        assert!(!table.is_empty());
        assert!(table.iter().any(|e| e.line == 10));
        assert!(table.iter().any(|e| e.line == 11));
        let dense = c.dense_line_table();
        assert_eq!(dense.len(), c.code.len());
        assert_eq!(dense[0], 10);
    }

    #[test]
    fn module_metadata_and_disasm() {
        let mut m = BytecodeModule::new();
        m.file = Some("x.vel".into());
        m.metadata = ModuleMetadata::current().with_source("x.vel", Some(fnv1a64(b"hi")));
        let mut c = Chunk::new("<script>");
        c.emit_op(Op::Null);
        c.emit_op(Op::Return);
        m.functions.push(c);
        m.exports.insert("<script>".into(), 0);
        let text = disassemble_module(&m);
        assert!(text.contains("version="));
        assert!(text.contains("<script>"));
        assert_eq!(m.main_index(), Some(0));
    }

    #[test]
    fn fnv_stable() {
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_ne!(fnv1a64(b"a"), fnv1a64(b"b"));
    }

    #[test]
    fn operand_lens() {
        assert_eq!(Op::NativeCall.operand_len(), 3);
        assert_eq!(Op::Constant.operand_len(), 2);
        assert_eq!(Op::Call.operand_len(), 1);
        assert_eq!(Op::Yield.operand_len(), 0);
    }
}

/// VS2 opcode catalog.
pub mod opcodes_vs2;
