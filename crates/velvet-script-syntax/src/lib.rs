//! Velvet Script 2 syntax tables: keywords, operators, editions, diagnostics.

#![deny(missing_docs)]

/// Language edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    /// Original surface.
    V1 = 1,
    /// Typed rust-like surface.
    V2 = 2,
}
impl Edition {
    /// Parse.
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            _ => None,
        }
    }
    /// Latest.
    pub fn latest() -> Self {
        Self::V2
    }
    /// As number.
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// `as`
    AsKw,
    /// `async`
    AsyncKw,
    /// `await`
    AwaitKw,
    /// `break`
    BreakKw,
    /// `call`
    CallKw,
    /// `character`
    CharacterKw,
    /// `const`
    ConstKw,
    /// `continue`
    ContinueKw,
    /// `crate`
    CrateKw,
    /// `else`
    ElseKw,
    /// `enum`
    EnumKw,
    /// `extern`
    ExternKw,
    /// `false`
    FalseKw,
    /// `fn`
    FnKw,
    /// `for`
    ForKw,
    /// `function`
    FunctionKw,
    /// `if`
    IfKw,
    /// `impl`
    ImplKw,
    /// `import`
    ImportKw,
    /// `in`
    InKw,
    /// `jump`
    JumpKw,
    /// `let`
    LetKw,
    /// `loop`
    LoopKw,
    /// `match`
    MatchKw,
    /// `menu`
    MenuKw,
    /// `mod`
    ModKw,
    /// `move`
    MoveKw,
    /// `mut`
    MutKw,
    /// `pub`
    PubKw,
    /// `ref`
    RefKw,
    /// `return`
    ReturnKw,
    /// `scene`
    SceneKw,
    /// `screen`
    ScreenKw,
    /// `self`
    SelfValue,
    /// `Self`
    SelfType,
    /// `show`
    ShowKw,
    /// `hide`
    HideKw,
    /// `say`
    SayKw,
    /// `state`
    StateKw,
    /// `static`
    StaticKw,
    /// `struct`
    StructKw,
    /// `super`
    SuperKw,
    /// `trait`
    TraitKw,
    /// `true`
    TrueKw,
    /// `type`
    TypeKw,
    /// `use`
    UseKw,
    /// `where`
    WhereKw,
    /// `while`
    WhileKw,
    /// `with`
    WithKw,
    /// `transform`
    TransformKw,
    /// `layer`
    LayerKw,
    /// `background`
    BackgroundKw,
    /// `music`
    MusicKw,
    /// `choice`
    ChoiceKw,
    /// `option`
    OptionKw,
    /// `Ok`
    OkKw,
    /// `Err`
    ErrKw,
    /// `Some`
    SomeKw,
    /// `None`
    NoneKw,
    /// `Result`
    ResultKw,
    /// `Option`
    OptionTypeKw,
    /// `try`
    TryKw,
}

impl Keyword {
    /// Text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AsKw => "as",
            Self::AsyncKw => "async",
            Self::AwaitKw => "await",
            Self::BreakKw => "break",
            Self::CallKw => "call",
            Self::CharacterKw => "character",
            Self::ConstKw => "const",
            Self::ContinueKw => "continue",
            Self::CrateKw => "crate",
            Self::ElseKw => "else",
            Self::EnumKw => "enum",
            Self::ExternKw => "extern",
            Self::FalseKw => "false",
            Self::FnKw => "fn",
            Self::ForKw => "for",
            Self::FunctionKw => "function",
            Self::IfKw => "if",
            Self::ImplKw => "impl",
            Self::ImportKw => "import",
            Self::InKw => "in",
            Self::JumpKw => "jump",
            Self::LetKw => "let",
            Self::LoopKw => "loop",
            Self::MatchKw => "match",
            Self::MenuKw => "menu",
            Self::ModKw => "mod",
            Self::MoveKw => "move",
            Self::MutKw => "mut",
            Self::PubKw => "pub",
            Self::RefKw => "ref",
            Self::ReturnKw => "return",
            Self::SceneKw => "scene",
            Self::ScreenKw => "screen",
            Self::SelfValue => "self",
            Self::SelfType => "Self",
            Self::ShowKw => "show",
            Self::HideKw => "hide",
            Self::SayKw => "say",
            Self::StateKw => "state",
            Self::StaticKw => "static",
            Self::StructKw => "struct",
            Self::SuperKw => "super",
            Self::TraitKw => "trait",
            Self::TrueKw => "true",
            Self::TypeKw => "type",
            Self::UseKw => "use",
            Self::WhereKw => "where",
            Self::WhileKw => "while",
            Self::WithKw => "with",
            Self::TransformKw => "transform",
            Self::LayerKw => "layer",
            Self::BackgroundKw => "background",
            Self::MusicKw => "music",
            Self::ChoiceKw => "choice",
            Self::OptionKw => "option",
            Self::OkKw => "Ok",
            Self::ErrKw => "Err",
            Self::SomeKw => "Some",
            Self::NoneKw => "None",
            Self::ResultKw => "Result",
            Self::OptionTypeKw => "Option",
            Self::TryKw => "try",
        }
    }
    /// Lookup.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "as" => Some(Self::AsKw),
            "async" => Some(Self::AsyncKw),
            "await" => Some(Self::AwaitKw),
            "break" => Some(Self::BreakKw),
            "call" => Some(Self::CallKw),
            "character" => Some(Self::CharacterKw),
            "const" => Some(Self::ConstKw),
            "continue" => Some(Self::ContinueKw),
            "crate" => Some(Self::CrateKw),
            "else" => Some(Self::ElseKw),
            "enum" => Some(Self::EnumKw),
            "extern" => Some(Self::ExternKw),
            "false" => Some(Self::FalseKw),
            "fn" => Some(Self::FnKw),
            "for" => Some(Self::ForKw),
            "function" => Some(Self::FunctionKw),
            "if" => Some(Self::IfKw),
            "impl" => Some(Self::ImplKw),
            "import" => Some(Self::ImportKw),
            "in" => Some(Self::InKw),
            "jump" => Some(Self::JumpKw),
            "let" => Some(Self::LetKw),
            "loop" => Some(Self::LoopKw),
            "match" => Some(Self::MatchKw),
            "menu" => Some(Self::MenuKw),
            "mod" => Some(Self::ModKw),
            "move" => Some(Self::MoveKw),
            "mut" => Some(Self::MutKw),
            "pub" => Some(Self::PubKw),
            "ref" => Some(Self::RefKw),
            "return" => Some(Self::ReturnKw),
            "scene" => Some(Self::SceneKw),
            "screen" => Some(Self::ScreenKw),
            "self" => Some(Self::SelfValue),
            "Self" => Some(Self::SelfType),
            "show" => Some(Self::ShowKw),
            "hide" => Some(Self::HideKw),
            "say" => Some(Self::SayKw),
            "state" => Some(Self::StateKw),
            "static" => Some(Self::StaticKw),
            "struct" => Some(Self::StructKw),
            "super" => Some(Self::SuperKw),
            "trait" => Some(Self::TraitKw),
            "true" => Some(Self::TrueKw),
            "type" => Some(Self::TypeKw),
            "use" => Some(Self::UseKw),
            "where" => Some(Self::WhereKw),
            "while" => Some(Self::WhileKw),
            "with" => Some(Self::WithKw),
            "transform" => Some(Self::TransformKw),
            "layer" => Some(Self::LayerKw),
            "background" => Some(Self::BackgroundKw),
            "music" => Some(Self::MusicKw),
            "choice" => Some(Self::ChoiceKw),
            "option" => Some(Self::OptionKw),
            "Ok" => Some(Self::OkKw),
            "Err" => Some(Self::ErrKw),
            "Some" => Some(Self::SomeKw),
            "None" => Some(Self::NoneKw),
            "Result" => Some(Self::ResultKw),
            "Option" => Some(Self::OptionTypeKw),
            "try" => Some(Self::TryKw),
            _ => None,
        }
    }
    /// All.
    pub fn all() -> &'static [Self] {
        &[
            Self::AsKw,
            Self::AsyncKw,
            Self::AwaitKw,
            Self::BreakKw,
            Self::CallKw,
            Self::CharacterKw,
            Self::ConstKw,
            Self::ContinueKw,
            Self::CrateKw,
            Self::ElseKw,
            Self::EnumKw,
            Self::ExternKw,
            Self::FalseKw,
            Self::FnKw,
            Self::ForKw,
            Self::FunctionKw,
            Self::IfKw,
            Self::ImplKw,
            Self::ImportKw,
            Self::InKw,
            Self::JumpKw,
            Self::LetKw,
            Self::LoopKw,
            Self::MatchKw,
            Self::MenuKw,
            Self::ModKw,
            Self::MoveKw,
            Self::MutKw,
            Self::PubKw,
            Self::RefKw,
            Self::ReturnKw,
            Self::SceneKw,
            Self::ScreenKw,
            Self::SelfValue,
            Self::SelfType,
            Self::ShowKw,
            Self::HideKw,
            Self::SayKw,
            Self::StateKw,
            Self::StaticKw,
            Self::StructKw,
            Self::SuperKw,
            Self::TraitKw,
            Self::TrueKw,
            Self::TypeKw,
            Self::UseKw,
            Self::WhereKw,
            Self::WhileKw,
            Self::WithKw,
            Self::TransformKw,
            Self::LayerKw,
            Self::BackgroundKw,
            Self::MusicKw,
            Self::ChoiceKw,
            Self::OptionKw,
            Self::OkKw,
            Self::ErrKw,
            Self::SomeKw,
            Self::NoneKw,
            Self::ResultKw,
            Self::OptionTypeKw,
            Self::TryKw,
        ]
    }
}

/// Operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubAssign,
    /// `*=`
    MulAssign,
    /// `/=`
    DivAssign,
}
impl Op {
    /// Symbol.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::And => "&&",
            Self::Or => "||",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
        }
    }
    /// Precedence (higher binds tighter).
    pub fn precedence(self) -> u8 {
        match self {
            Self::Add => 40,
            Self::Sub => 40,
            Self::Mul => 50,
            Self::Div => 50,
            Self::Rem => 50,
            Self::Eq => 18,
            Self::Ne => 18,
            Self::Lt => 20,
            Self::Le => 20,
            Self::Gt => 20,
            Self::Ge => 20,
            Self::And => 15,
            Self::Or => 12,
            Self::BitAnd => 30,
            Self::BitOr => 26,
            Self::BitXor => 28,
            Self::Shl => 35,
            Self::Shr => 35,
            Self::Assign => 5,
            Self::AddAssign => 5,
            Self::SubAssign => 5,
            Self::MulAssign => 5,
            Self::DivAssign => 5,
        }
    }
}

/// Stable diagnostic codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum DiagCode {
    /// VS0001
    E0001 = 1,
    /// VS0002
    E0002 = 2,
    /// VS0003
    E0003 = 3,
    /// VS0004
    E0004 = 4,
    /// VS0005
    E0005 = 5,
    /// VS0006
    E0006 = 6,
    /// VS0007
    E0007 = 7,
    /// VS0008
    E0008 = 8,
    /// VS0009
    E0009 = 9,
    /// VS0010
    E0010 = 10,
    /// VS0011
    E0011 = 11,
    /// VS0012
    E0012 = 12,
    /// VS0013
    E0013 = 13,
    /// VS0014
    E0014 = 14,
    /// VS0015
    E0015 = 15,
    /// VS0016
    E0016 = 16,
    /// VS0017
    E0017 = 17,
    /// VS0018
    E0018 = 18,
    /// VS0019
    E0019 = 19,
    /// VS0020
    E0020 = 20,
    /// VS0021
    E0021 = 21,
    /// VS0022
    E0022 = 22,
    /// VS0023
    E0023 = 23,
    /// VS0024
    E0024 = 24,
    /// VS0025
    E0025 = 25,
    /// VS0026
    E0026 = 26,
    /// VS0027
    E0027 = 27,
    /// VS0028
    E0028 = 28,
    /// VS0029
    E0029 = 29,
    /// VS0030
    E0030 = 30,
    /// VS0031
    E0031 = 31,
    /// VS0032
    E0032 = 32,
    /// VS0033
    E0033 = 33,
    /// VS0034
    E0034 = 34,
    /// VS0035
    E0035 = 35,
    /// VS0036
    E0036 = 36,
    /// VS0037
    E0037 = 37,
    /// VS0038
    E0038 = 38,
    /// VS0039
    E0039 = 39,
    /// VS0040
    E0040 = 40,
    /// VS0041
    E0041 = 41,
    /// VS0042
    E0042 = 42,
    /// VS0043
    E0043 = 43,
    /// VS0044
    E0044 = 44,
    /// VS0045
    E0045 = 45,
    /// VS0046
    E0046 = 46,
    /// VS0047
    E0047 = 47,
    /// VS0048
    E0048 = 48,
    /// VS0049
    E0049 = 49,
    /// VS0050
    E0050 = 50,
    /// VS0051
    E0051 = 51,
    /// VS0052
    E0052 = 52,
    /// VS0053
    E0053 = 53,
    /// VS0054
    E0054 = 54,
    /// VS0055
    E0055 = 55,
    /// VS0056
    E0056 = 56,
    /// VS0057
    E0057 = 57,
    /// VS0058
    E0058 = 58,
    /// VS0059
    E0059 = 59,
    /// VS0060
    E0060 = 60,
    /// VS0061
    E0061 = 61,
    /// VS0062
    E0062 = 62,
    /// VS0063
    E0063 = 63,
    /// VS0064
    E0064 = 64,
    /// VS0065
    E0065 = 65,
    /// VS0066
    E0066 = 66,
    /// VS0067
    E0067 = 67,
    /// VS0068
    E0068 = 68,
    /// VS0069
    E0069 = 69,
    /// VS0070
    E0070 = 70,
    /// VS0071
    E0071 = 71,
    /// VS0072
    E0072 = 72,
    /// VS0073
    E0073 = 73,
    /// VS0074
    E0074 = 74,
    /// VS0075
    E0075 = 75,
    /// VS0076
    E0076 = 76,
    /// VS0077
    E0077 = 77,
    /// VS0078
    E0078 = 78,
    /// VS0079
    E0079 = 79,
    /// VS0080
    E0080 = 80,
    /// VS0081
    E0081 = 81,
    /// VS0082
    E0082 = 82,
    /// VS0083
    E0083 = 83,
    /// VS0084
    E0084 = 84,
    /// VS0085
    E0085 = 85,
    /// VS0086
    E0086 = 86,
    /// VS0087
    E0087 = 87,
    /// VS0088
    E0088 = 88,
    /// VS0089
    E0089 = 89,
    /// VS0090
    E0090 = 90,
    /// VS0091
    E0091 = 91,
    /// VS0092
    E0092 = 92,
    /// VS0093
    E0093 = 93,
    /// VS0094
    E0094 = 94,
    /// VS0095
    E0095 = 95,
    /// VS0096
    E0096 = 96,
    /// VS0097
    E0097 = 97,
    /// VS0098
    E0098 = 98,
    /// VS0099
    E0099 = 99,
    /// VS0100
    E0100 = 100,
    /// VS0101
    E0101 = 101,
    /// VS0102
    E0102 = 102,
    /// VS0103
    E0103 = 103,
    /// VS0104
    E0104 = 104,
    /// VS0105
    E0105 = 105,
    /// VS0106
    E0106 = 106,
    /// VS0107
    E0107 = 107,
    /// VS0108
    E0108 = 108,
    /// VS0109
    E0109 = 109,
    /// VS0110
    E0110 = 110,
    /// VS0111
    E0111 = 111,
    /// VS0112
    E0112 = 112,
    /// VS0113
    E0113 = 113,
    /// VS0114
    E0114 = 114,
    /// VS0115
    E0115 = 115,
    /// VS0116
    E0116 = 116,
    /// VS0117
    E0117 = 117,
    /// VS0118
    E0118 = 118,
    /// VS0119
    E0119 = 119,
    /// VS0120
    E0120 = 120,
    /// VS0121
    E0121 = 121,
    /// VS0122
    E0122 = 122,
    /// VS0123
    E0123 = 123,
    /// VS0124
    E0124 = 124,
    /// VS0125
    E0125 = 125,
    /// VS0126
    E0126 = 126,
    /// VS0127
    E0127 = 127,
    /// VS0128
    E0128 = 128,
    /// VS0129
    E0129 = 129,
    /// VS0130
    E0130 = 130,
    /// VS0131
    E0131 = 131,
    /// VS0132
    E0132 = 132,
    /// VS0133
    E0133 = 133,
    /// VS0134
    E0134 = 134,
    /// VS0135
    E0135 = 135,
    /// VS0136
    E0136 = 136,
    /// VS0137
    E0137 = 137,
    /// VS0138
    E0138 = 138,
    /// VS0139
    E0139 = 139,
    /// VS0140
    E0140 = 140,
    /// VS0141
    E0141 = 141,
    /// VS0142
    E0142 = 142,
    /// VS0143
    E0143 = 143,
    /// VS0144
    E0144 = 144,
    /// VS0145
    E0145 = 145,
    /// VS0146
    E0146 = 146,
    /// VS0147
    E0147 = 147,
    /// VS0148
    E0148 = 148,
    /// VS0149
    E0149 = 149,
    /// VS0150
    E0150 = 150,
    /// VS0151
    E0151 = 151,
    /// VS0152
    E0152 = 152,
    /// VS0153
    E0153 = 153,
    /// VS0154
    E0154 = 154,
    /// VS0155
    E0155 = 155,
    /// VS0156
    E0156 = 156,
    /// VS0157
    E0157 = 157,
    /// VS0158
    E0158 = 158,
    /// VS0159
    E0159 = 159,
    /// VS0160
    E0160 = 160,
    /// VS0161
    E0161 = 161,
    /// VS0162
    E0162 = 162,
    /// VS0163
    E0163 = 163,
    /// VS0164
    E0164 = 164,
    /// VS0165
    E0165 = 165,
    /// VS0166
    E0166 = 166,
    /// VS0167
    E0167 = 167,
    /// VS0168
    E0168 = 168,
    /// VS0169
    E0169 = 169,
    /// VS0170
    E0170 = 170,
    /// VS0171
    E0171 = 171,
    /// VS0172
    E0172 = 172,
    /// VS0173
    E0173 = 173,
    /// VS0174
    E0174 = 174,
    /// VS0175
    E0175 = 175,
    /// VS0176
    E0176 = 176,
    /// VS0177
    E0177 = 177,
    /// VS0178
    E0178 = 178,
    /// VS0179
    E0179 = 179,
    /// VS0180
    E0180 = 180,
    /// VS0181
    E0181 = 181,
    /// VS0182
    E0182 = 182,
    /// VS0183
    E0183 = 183,
    /// VS0184
    E0184 = 184,
    /// VS0185
    E0185 = 185,
    /// VS0186
    E0186 = 186,
    /// VS0187
    E0187 = 187,
    /// VS0188
    E0188 = 188,
    /// VS0189
    E0189 = 189,
    /// VS0190
    E0190 = 190,
    /// VS0191
    E0191 = 191,
    /// VS0192
    E0192 = 192,
    /// VS0193
    E0193 = 193,
    /// VS0194
    E0194 = 194,
    /// VS0195
    E0195 = 195,
    /// VS0196
    E0196 = 196,
    /// VS0197
    E0197 = 197,
    /// VS0198
    E0198 = 198,
    /// VS0199
    E0199 = 199,
    /// VS0200
    E0200 = 200,
    /// VS0201
    E0201 = 201,
    /// VS0202
    E0202 = 202,
    /// VS0203
    E0203 = 203,
    /// VS0204
    E0204 = 204,
    /// VS0205
    E0205 = 205,
    /// VS0206
    E0206 = 206,
    /// VS0207
    E0207 = 207,
    /// VS0208
    E0208 = 208,
    /// VS0209
    E0209 = 209,
    /// VS0210
    E0210 = 210,
    /// VS0211
    E0211 = 211,
    /// VS0212
    E0212 = 212,
    /// VS0213
    E0213 = 213,
    /// VS0214
    E0214 = 214,
    /// VS0215
    E0215 = 215,
    /// VS0216
    E0216 = 216,
    /// VS0217
    E0217 = 217,
    /// VS0218
    E0218 = 218,
    /// VS0219
    E0219 = 219,
    /// VS0220
    E0220 = 220,
    /// VS0221
    E0221 = 221,
    /// VS0222
    E0222 = 222,
    /// VS0223
    E0223 = 223,
    /// VS0224
    E0224 = 224,
    /// VS0225
    E0225 = 225,
    /// VS0226
    E0226 = 226,
    /// VS0227
    E0227 = 227,
    /// VS0228
    E0228 = 228,
    /// VS0229
    E0229 = 229,
    /// VS0230
    E0230 = 230,
    /// VS0231
    E0231 = 231,
    /// VS0232
    E0232 = 232,
    /// VS0233
    E0233 = 233,
    /// VS0234
    E0234 = 234,
    /// VS0235
    E0235 = 235,
    /// VS0236
    E0236 = 236,
    /// VS0237
    E0237 = 237,
    /// VS0238
    E0238 = 238,
    /// VS0239
    E0239 = 239,
    /// VS0240
    E0240 = 240,
    /// VS0241
    E0241 = 241,
    /// VS0242
    E0242 = 242,
    /// VS0243
    E0243 = 243,
    /// VS0244
    E0244 = 244,
    /// VS0245
    E0245 = 245,
    /// VS0246
    E0246 = 246,
    /// VS0247
    E0247 = 247,
    /// VS0248
    E0248 = 248,
    /// VS0249
    E0249 = 249,
    /// VS0250
    E0250 = 250,
    /// VS0251
    E0251 = 251,
    /// VS0252
    E0252 = 252,
    /// VS0253
    E0253 = 253,
    /// VS0254
    E0254 = 254,
    /// VS0255
    E0255 = 255,
    /// VS0256
    E0256 = 256,
    /// VS0257
    E0257 = 257,
    /// VS0258
    E0258 = 258,
    /// VS0259
    E0259 = 259,
    /// VS0260
    E0260 = 260,
    /// VS0261
    E0261 = 261,
    /// VS0262
    E0262 = 262,
    /// VS0263
    E0263 = 263,
    /// VS0264
    E0264 = 264,
    /// VS0265
    E0265 = 265,
    /// VS0266
    E0266 = 266,
    /// VS0267
    E0267 = 267,
    /// VS0268
    E0268 = 268,
    /// VS0269
    E0269 = 269,
    /// VS0270
    E0270 = 270,
    /// VS0271
    E0271 = 271,
    /// VS0272
    E0272 = 272,
    /// VS0273
    E0273 = 273,
    /// VS0274
    E0274 = 274,
    /// VS0275
    E0275 = 275,
    /// VS0276
    E0276 = 276,
    /// VS0277
    E0277 = 277,
    /// VS0278
    E0278 = 278,
    /// VS0279
    E0279 = 279,
    /// VS0280
    E0280 = 280,
    /// VS0281
    E0281 = 281,
    /// VS0282
    E0282 = 282,
    /// VS0283
    E0283 = 283,
    /// VS0284
    E0284 = 284,
    /// VS0285
    E0285 = 285,
    /// VS0286
    E0286 = 286,
    /// VS0287
    E0287 = 287,
    /// VS0288
    E0288 = 288,
    /// VS0289
    E0289 = 289,
    /// VS0290
    E0290 = 290,
    /// VS0291
    E0291 = 291,
    /// VS0292
    E0292 = 292,
    /// VS0293
    E0293 = 293,
    /// VS0294
    E0294 = 294,
    /// VS0295
    E0295 = 295,
    /// VS0296
    E0296 = 296,
    /// VS0297
    E0297 = 297,
    /// VS0298
    E0298 = 298,
    /// VS0299
    E0299 = 299,
    /// VS0300
    E0300 = 300,
    /// VS0301
    E0301 = 301,
    /// VS0302
    E0302 = 302,
    /// VS0303
    E0303 = 303,
    /// VS0304
    E0304 = 304,
    /// VS0305
    E0305 = 305,
    /// VS0306
    E0306 = 306,
    /// VS0307
    E0307 = 307,
    /// VS0308
    E0308 = 308,
    /// VS0309
    E0309 = 309,
    /// VS0310
    E0310 = 310,
    /// VS0311
    E0311 = 311,
    /// VS0312
    E0312 = 312,
    /// VS0313
    E0313 = 313,
    /// VS0314
    E0314 = 314,
    /// VS0315
    E0315 = 315,
    /// VS0316
    E0316 = 316,
    /// VS0317
    E0317 = 317,
    /// VS0318
    E0318 = 318,
    /// VS0319
    E0319 = 319,
    /// VS0320
    E0320 = 320,
    /// VS0321
    E0321 = 321,
    /// VS0322
    E0322 = 322,
    /// VS0323
    E0323 = 323,
    /// VS0324
    E0324 = 324,
    /// VS0325
    E0325 = 325,
    /// VS0326
    E0326 = 326,
    /// VS0327
    E0327 = 327,
    /// VS0328
    E0328 = 328,
    /// VS0329
    E0329 = 329,
    /// VS0330
    E0330 = 330,
    /// VS0331
    E0331 = 331,
    /// VS0332
    E0332 = 332,
    /// VS0333
    E0333 = 333,
    /// VS0334
    E0334 = 334,
    /// VS0335
    E0335 = 335,
    /// VS0336
    E0336 = 336,
    /// VS0337
    E0337 = 337,
    /// VS0338
    E0338 = 338,
    /// VS0339
    E0339 = 339,
    /// VS0340
    E0340 = 340,
    /// VS0341
    E0341 = 341,
    /// VS0342
    E0342 = 342,
    /// VS0343
    E0343 = 343,
    /// VS0344
    E0344 = 344,
    /// VS0345
    E0345 = 345,
    /// VS0346
    E0346 = 346,
    /// VS0347
    E0347 = 347,
    /// VS0348
    E0348 = 348,
    /// VS0349
    E0349 = 349,
    /// VS0350
    E0350 = 350,
    /// VS0351
    E0351 = 351,
    /// VS0352
    E0352 = 352,
    /// VS0353
    E0353 = 353,
    /// VS0354
    E0354 = 354,
    /// VS0355
    E0355 = 355,
    /// VS0356
    E0356 = 356,
    /// VS0357
    E0357 = 357,
    /// VS0358
    E0358 = 358,
    /// VS0359
    E0359 = 359,
    /// VS0360
    E0360 = 360,
    /// VS0361
    E0361 = 361,
    /// VS0362
    E0362 = 362,
    /// VS0363
    E0363 = 363,
    /// VS0364
    E0364 = 364,
    /// VS0365
    E0365 = 365,
    /// VS0366
    E0366 = 366,
    /// VS0367
    E0367 = 367,
    /// VS0368
    E0368 = 368,
    /// VS0369
    E0369 = 369,
    /// VS0370
    E0370 = 370,
    /// VS0371
    E0371 = 371,
    /// VS0372
    E0372 = 372,
    /// VS0373
    E0373 = 373,
    /// VS0374
    E0374 = 374,
    /// VS0375
    E0375 = 375,
    /// VS0376
    E0376 = 376,
    /// VS0377
    E0377 = 377,
    /// VS0378
    E0378 = 378,
    /// VS0379
    E0379 = 379,
    /// VS0380
    E0380 = 380,
    /// VS0381
    E0381 = 381,
    /// VS0382
    E0382 = 382,
    /// VS0383
    E0383 = 383,
    /// VS0384
    E0384 = 384,
    /// VS0385
    E0385 = 385,
    /// VS0386
    E0386 = 386,
    /// VS0387
    E0387 = 387,
    /// VS0388
    E0388 = 388,
    /// VS0389
    E0389 = 389,
    /// VS0390
    E0390 = 390,
    /// VS0391
    E0391 = 391,
    /// VS0392
    E0392 = 392,
    /// VS0393
    E0393 = 393,
    /// VS0394
    E0394 = 394,
    /// VS0395
    E0395 = 395,
    /// VS0396
    E0396 = 396,
    /// VS0397
    E0397 = 397,
    /// VS0398
    E0398 = 398,
    /// VS0399
    E0399 = 399,
    /// VS0400
    E0400 = 400,
    /// VS0401
    E0401 = 401,
    /// VS0402
    E0402 = 402,
    /// VS0403
    E0403 = 403,
    /// VS0404
    E0404 = 404,
    /// VS0405
    E0405 = 405,
    /// VS0406
    E0406 = 406,
    /// VS0407
    E0407 = 407,
    /// VS0408
    E0408 = 408,
    /// VS0409
    E0409 = 409,
    /// VS0410
    E0410 = 410,
    /// VS0411
    E0411 = 411,
    /// VS0412
    E0412 = 412,
    /// VS0413
    E0413 = 413,
    /// VS0414
    E0414 = 414,
    /// VS0415
    E0415 = 415,
    /// VS0416
    E0416 = 416,
    /// VS0417
    E0417 = 417,
    /// VS0418
    E0418 = 418,
    /// VS0419
    E0419 = 419,
    /// VS0420
    E0420 = 420,
    /// VS0421
    E0421 = 421,
    /// VS0422
    E0422 = 422,
    /// VS0423
    E0423 = 423,
    /// VS0424
    E0424 = 424,
    /// VS0425
    E0425 = 425,
    /// VS0426
    E0426 = 426,
    /// VS0427
    E0427 = 427,
    /// VS0428
    E0428 = 428,
    /// VS0429
    E0429 = 429,
    /// VS0430
    E0430 = 430,
    /// VS0431
    E0431 = 431,
    /// VS0432
    E0432 = 432,
    /// VS0433
    E0433 = 433,
    /// VS0434
    E0434 = 434,
    /// VS0435
    E0435 = 435,
    /// VS0436
    E0436 = 436,
    /// VS0437
    E0437 = 437,
    /// VS0438
    E0438 = 438,
    /// VS0439
    E0439 = 439,
    /// VS0440
    E0440 = 440,
    /// VS0441
    E0441 = 441,
    /// VS0442
    E0442 = 442,
    /// VS0443
    E0443 = 443,
    /// VS0444
    E0444 = 444,
    /// VS0445
    E0445 = 445,
    /// VS0446
    E0446 = 446,
    /// VS0447
    E0447 = 447,
    /// VS0448
    E0448 = 448,
    /// VS0449
    E0449 = 449,
    /// VS0450
    E0450 = 450,
    /// VS0451
    E0451 = 451,
    /// VS0452
    E0452 = 452,
    /// VS0453
    E0453 = 453,
    /// VS0454
    E0454 = 454,
    /// VS0455
    E0455 = 455,
    /// VS0456
    E0456 = 456,
    /// VS0457
    E0457 = 457,
    /// VS0458
    E0458 = 458,
    /// VS0459
    E0459 = 459,
    /// VS0460
    E0460 = 460,
    /// VS0461
    E0461 = 461,
    /// VS0462
    E0462 = 462,
    /// VS0463
    E0463 = 463,
    /// VS0464
    E0464 = 464,
    /// VS0465
    E0465 = 465,
    /// VS0466
    E0466 = 466,
    /// VS0467
    E0467 = 467,
    /// VS0468
    E0468 = 468,
    /// VS0469
    E0469 = 469,
    /// VS0470
    E0470 = 470,
    /// VS0471
    E0471 = 471,
    /// VS0472
    E0472 = 472,
    /// VS0473
    E0473 = 473,
    /// VS0474
    E0474 = 474,
    /// VS0475
    E0475 = 475,
    /// VS0476
    E0476 = 476,
    /// VS0477
    E0477 = 477,
    /// VS0478
    E0478 = 478,
    /// VS0479
    E0479 = 479,
    /// VS0480
    E0480 = 480,
    /// VS0481
    E0481 = 481,
    /// VS0482
    E0482 = 482,
    /// VS0483
    E0483 = 483,
    /// VS0484
    E0484 = 484,
    /// VS0485
    E0485 = 485,
    /// VS0486
    E0486 = 486,
    /// VS0487
    E0487 = 487,
    /// VS0488
    E0488 = 488,
    /// VS0489
    E0489 = 489,
    /// VS0490
    E0490 = 490,
    /// VS0491
    E0491 = 491,
    /// VS0492
    E0492 = 492,
    /// VS0493
    E0493 = 493,
    /// VS0494
    E0494 = 494,
    /// VS0495
    E0495 = 495,
    /// VS0496
    E0496 = 496,
    /// VS0497
    E0497 = 497,
    /// VS0498
    E0498 = 498,
    /// VS0499
    E0499 = 499,
    /// VS0500
    E0500 = 500,
}
impl DiagCode {
    /// Numeric code.
    pub fn code(self) -> u16 { self as u16 }
    /// Label.
    pub fn label(self) -> String { format!("VS{:04}", self.code() as u32) }
    /// Iterate all.
    pub fn all() -> impl Iterator<Item = Self> {
        (1u16..=500).map(|n| match n {
            1 => Self::E0001,
            2 => Self::E0002,
            3 => Self::E0003,
            4 => Self::E0004,
            5 => Self::E0005,
            6 => Self::E0006,
            7 => Self::E0007,
            8 => Self::E0008,
            9 => Self::E0009,
            10 => Self::E0010,
            11 => Self::E0011,
            12 => Self::E0012,
            13 => Self::E0013,
            14 => Self::E0014,
            15 => Self::E0015,
            16 => Self::E0016,
            17 => Self::E0017,
            18 => Self::E0018,
            19 => Self::E0019,
            20 => Self::E0020,
            21 => Self::E0021,
            22 => Self::E0022,
            23 => Self::E0023,
            24 => Self::E0024,
            25 => Self::E0025,
            26 => Self::E0026,
            27 => Self::E0027,
            28 => Self::E0028,
            29 => Self::E0029,
            30 => Self::E0030,
            31 => Self::E0031,
            32 => Self::E0032,
            33 => Self::E0033,
            34 => Self::E0034,
            35 => Self::E0035,
            36 => Self::E0036,
            37 => Self::E0037,
            38 => Self::E0038,
            39 => Self::E0039,
            40 => Self::E0040,
            41 => Self::E0041,
            42 => Self::E0042,
            43 => Self::E0043,
            44 => Self::E0044,
            45 => Self::E0045,
            46 => Self::E0046,
            47 => Self::E0047,
            48 => Self::E0048,
            49 => Self::E0049,
            50 => Self::E0050,
            51 => Self::E0051,
            52 => Self::E0052,
            53 => Self::E0053,
            54 => Self::E0054,
            55 => Self::E0055,
            56 => Self::E0056,
            57 => Self::E0057,
            58 => Self::E0058,
            59 => Self::E0059,
            60 => Self::E0060,
            61 => Self::E0061,
            62 => Self::E0062,
            63 => Self::E0063,
            64 => Self::E0064,
            65 => Self::E0065,
            66 => Self::E0066,
            67 => Self::E0067,
            68 => Self::E0068,
            69 => Self::E0069,
            70 => Self::E0070,
            71 => Self::E0071,
            72 => Self::E0072,
            73 => Self::E0073,
            74 => Self::E0074,
            75 => Self::E0075,
            76 => Self::E0076,
            77 => Self::E0077,
            78 => Self::E0078,
            79 => Self::E0079,
            80 => Self::E0080,
            81 => Self::E0081,
            82 => Self::E0082,
            83 => Self::E0083,
            84 => Self::E0084,
            85 => Self::E0085,
            86 => Self::E0086,
            87 => Self::E0087,
            88 => Self::E0088,
            89 => Self::E0089,
            90 => Self::E0090,
            91 => Self::E0091,
            92 => Self::E0092,
            93 => Self::E0093,
            94 => Self::E0094,
            95 => Self::E0095,
            96 => Self::E0096,
            97 => Self::E0097,
            98 => Self::E0098,
            99 => Self::E0099,
            100 => Self::E0100,
            101 => Self::E0101,
            102 => Self::E0102,
            103 => Self::E0103,
            104 => Self::E0104,
            105 => Self::E0105,
            106 => Self::E0106,
            107 => Self::E0107,
            108 => Self::E0108,
            109 => Self::E0109,
            110 => Self::E0110,
            111 => Self::E0111,
            112 => Self::E0112,
            113 => Self::E0113,
            114 => Self::E0114,
            115 => Self::E0115,
            116 => Self::E0116,
            117 => Self::E0117,
            118 => Self::E0118,
            119 => Self::E0119,
            120 => Self::E0120,
            121 => Self::E0121,
            122 => Self::E0122,
            123 => Self::E0123,
            124 => Self::E0124,
            125 => Self::E0125,
            126 => Self::E0126,
            127 => Self::E0127,
            128 => Self::E0128,
            129 => Self::E0129,
            130 => Self::E0130,
            131 => Self::E0131,
            132 => Self::E0132,
            133 => Self::E0133,
            134 => Self::E0134,
            135 => Self::E0135,
            136 => Self::E0136,
            137 => Self::E0137,
            138 => Self::E0138,
            139 => Self::E0139,
            140 => Self::E0140,
            141 => Self::E0141,
            142 => Self::E0142,
            143 => Self::E0143,
            144 => Self::E0144,
            145 => Self::E0145,
            146 => Self::E0146,
            147 => Self::E0147,
            148 => Self::E0148,
            149 => Self::E0149,
            150 => Self::E0150,
            151 => Self::E0151,
            152 => Self::E0152,
            153 => Self::E0153,
            154 => Self::E0154,
            155 => Self::E0155,
            156 => Self::E0156,
            157 => Self::E0157,
            158 => Self::E0158,
            159 => Self::E0159,
            160 => Self::E0160,
            161 => Self::E0161,
            162 => Self::E0162,
            163 => Self::E0163,
            164 => Self::E0164,
            165 => Self::E0165,
            166 => Self::E0166,
            167 => Self::E0167,
            168 => Self::E0168,
            169 => Self::E0169,
            170 => Self::E0170,
            171 => Self::E0171,
            172 => Self::E0172,
            173 => Self::E0173,
            174 => Self::E0174,
            175 => Self::E0175,
            176 => Self::E0176,
            177 => Self::E0177,
            178 => Self::E0178,
            179 => Self::E0179,
            180 => Self::E0180,
            181 => Self::E0181,
            182 => Self::E0182,
            183 => Self::E0183,
            184 => Self::E0184,
            185 => Self::E0185,
            186 => Self::E0186,
            187 => Self::E0187,
            188 => Self::E0188,
            189 => Self::E0189,
            190 => Self::E0190,
            191 => Self::E0191,
            192 => Self::E0192,
            193 => Self::E0193,
            194 => Self::E0194,
            195 => Self::E0195,
            196 => Self::E0196,
            197 => Self::E0197,
            198 => Self::E0198,
            199 => Self::E0199,
            200 => Self::E0200,
            201 => Self::E0201,
            202 => Self::E0202,
            203 => Self::E0203,
            204 => Self::E0204,
            205 => Self::E0205,
            206 => Self::E0206,
            207 => Self::E0207,
            208 => Self::E0208,
            209 => Self::E0209,
            210 => Self::E0210,
            211 => Self::E0211,
            212 => Self::E0212,
            213 => Self::E0213,
            214 => Self::E0214,
            215 => Self::E0215,
            216 => Self::E0216,
            217 => Self::E0217,
            218 => Self::E0218,
            219 => Self::E0219,
            220 => Self::E0220,
            221 => Self::E0221,
            222 => Self::E0222,
            223 => Self::E0223,
            224 => Self::E0224,
            225 => Self::E0225,
            226 => Self::E0226,
            227 => Self::E0227,
            228 => Self::E0228,
            229 => Self::E0229,
            230 => Self::E0230,
            231 => Self::E0231,
            232 => Self::E0232,
            233 => Self::E0233,
            234 => Self::E0234,
            235 => Self::E0235,
            236 => Self::E0236,
            237 => Self::E0237,
            238 => Self::E0238,
            239 => Self::E0239,
            240 => Self::E0240,
            241 => Self::E0241,
            242 => Self::E0242,
            243 => Self::E0243,
            244 => Self::E0244,
            245 => Self::E0245,
            246 => Self::E0246,
            247 => Self::E0247,
            248 => Self::E0248,
            249 => Self::E0249,
            250 => Self::E0250,
            251 => Self::E0251,
            252 => Self::E0252,
            253 => Self::E0253,
            254 => Self::E0254,
            255 => Self::E0255,
            256 => Self::E0256,
            257 => Self::E0257,
            258 => Self::E0258,
            259 => Self::E0259,
            260 => Self::E0260,
            261 => Self::E0261,
            262 => Self::E0262,
            263 => Self::E0263,
            264 => Self::E0264,
            265 => Self::E0265,
            266 => Self::E0266,
            267 => Self::E0267,
            268 => Self::E0268,
            269 => Self::E0269,
            270 => Self::E0270,
            271 => Self::E0271,
            272 => Self::E0272,
            273 => Self::E0273,
            274 => Self::E0274,
            275 => Self::E0275,
            276 => Self::E0276,
            277 => Self::E0277,
            278 => Self::E0278,
            279 => Self::E0279,
            280 => Self::E0280,
            281 => Self::E0281,
            282 => Self::E0282,
            283 => Self::E0283,
            284 => Self::E0284,
            285 => Self::E0285,
            286 => Self::E0286,
            287 => Self::E0287,
            288 => Self::E0288,
            289 => Self::E0289,
            290 => Self::E0290,
            291 => Self::E0291,
            292 => Self::E0292,
            293 => Self::E0293,
            294 => Self::E0294,
            295 => Self::E0295,
            296 => Self::E0296,
            297 => Self::E0297,
            298 => Self::E0298,
            299 => Self::E0299,
            300 => Self::E0300,
            301 => Self::E0301,
            302 => Self::E0302,
            303 => Self::E0303,
            304 => Self::E0304,
            305 => Self::E0305,
            306 => Self::E0306,
            307 => Self::E0307,
            308 => Self::E0308,
            309 => Self::E0309,
            310 => Self::E0310,
            311 => Self::E0311,
            312 => Self::E0312,
            313 => Self::E0313,
            314 => Self::E0314,
            315 => Self::E0315,
            316 => Self::E0316,
            317 => Self::E0317,
            318 => Self::E0318,
            319 => Self::E0319,
            320 => Self::E0320,
            321 => Self::E0321,
            322 => Self::E0322,
            323 => Self::E0323,
            324 => Self::E0324,
            325 => Self::E0325,
            326 => Self::E0326,
            327 => Self::E0327,
            328 => Self::E0328,
            329 => Self::E0329,
            330 => Self::E0330,
            331 => Self::E0331,
            332 => Self::E0332,
            333 => Self::E0333,
            334 => Self::E0334,
            335 => Self::E0335,
            336 => Self::E0336,
            337 => Self::E0337,
            338 => Self::E0338,
            339 => Self::E0339,
            340 => Self::E0340,
            341 => Self::E0341,
            342 => Self::E0342,
            343 => Self::E0343,
            344 => Self::E0344,
            345 => Self::E0345,
            346 => Self::E0346,
            347 => Self::E0347,
            348 => Self::E0348,
            349 => Self::E0349,
            350 => Self::E0350,
            351 => Self::E0351,
            352 => Self::E0352,
            353 => Self::E0353,
            354 => Self::E0354,
            355 => Self::E0355,
            356 => Self::E0356,
            357 => Self::E0357,
            358 => Self::E0358,
            359 => Self::E0359,
            360 => Self::E0360,
            361 => Self::E0361,
            362 => Self::E0362,
            363 => Self::E0363,
            364 => Self::E0364,
            365 => Self::E0365,
            366 => Self::E0366,
            367 => Self::E0367,
            368 => Self::E0368,
            369 => Self::E0369,
            370 => Self::E0370,
            371 => Self::E0371,
            372 => Self::E0372,
            373 => Self::E0373,
            374 => Self::E0374,
            375 => Self::E0375,
            376 => Self::E0376,
            377 => Self::E0377,
            378 => Self::E0378,
            379 => Self::E0379,
            380 => Self::E0380,
            381 => Self::E0381,
            382 => Self::E0382,
            383 => Self::E0383,
            384 => Self::E0384,
            385 => Self::E0385,
            386 => Self::E0386,
            387 => Self::E0387,
            388 => Self::E0388,
            389 => Self::E0389,
            390 => Self::E0390,
            391 => Self::E0391,
            392 => Self::E0392,
            393 => Self::E0393,
            394 => Self::E0394,
            395 => Self::E0395,
            396 => Self::E0396,
            397 => Self::E0397,
            398 => Self::E0398,
            399 => Self::E0399,
            400 => Self::E0400,
            401 => Self::E0401,
            402 => Self::E0402,
            403 => Self::E0403,
            404 => Self::E0404,
            405 => Self::E0405,
            406 => Self::E0406,
            407 => Self::E0407,
            408 => Self::E0408,
            409 => Self::E0409,
            410 => Self::E0410,
            411 => Self::E0411,
            412 => Self::E0412,
            413 => Self::E0413,
            414 => Self::E0414,
            415 => Self::E0415,
            416 => Self::E0416,
            417 => Self::E0417,
            418 => Self::E0418,
            419 => Self::E0419,
            420 => Self::E0420,
            421 => Self::E0421,
            422 => Self::E0422,
            423 => Self::E0423,
            424 => Self::E0424,
            425 => Self::E0425,
            426 => Self::E0426,
            427 => Self::E0427,
            428 => Self::E0428,
            429 => Self::E0429,
            430 => Self::E0430,
            431 => Self::E0431,
            432 => Self::E0432,
            433 => Self::E0433,
            434 => Self::E0434,
            435 => Self::E0435,
            436 => Self::E0436,
            437 => Self::E0437,
            438 => Self::E0438,
            439 => Self::E0439,
            440 => Self::E0440,
            441 => Self::E0441,
            442 => Self::E0442,
            443 => Self::E0443,
            444 => Self::E0444,
            445 => Self::E0445,
            446 => Self::E0446,
            447 => Self::E0447,
            448 => Self::E0448,
            449 => Self::E0449,
            450 => Self::E0450,
            451 => Self::E0451,
            452 => Self::E0452,
            453 => Self::E0453,
            454 => Self::E0454,
            455 => Self::E0455,
            456 => Self::E0456,
            457 => Self::E0457,
            458 => Self::E0458,
            459 => Self::E0459,
            460 => Self::E0460,
            461 => Self::E0461,
            462 => Self::E0462,
            463 => Self::E0463,
            464 => Self::E0464,
            465 => Self::E0465,
            466 => Self::E0466,
            467 => Self::E0467,
            468 => Self::E0468,
            469 => Self::E0469,
            470 => Self::E0470,
            471 => Self::E0471,
            472 => Self::E0472,
            473 => Self::E0473,
            474 => Self::E0474,
            475 => Self::E0475,
            476 => Self::E0476,
            477 => Self::E0477,
            478 => Self::E0478,
            479 => Self::E0479,
            480 => Self::E0480,
            481 => Self::E0481,
            482 => Self::E0482,
            483 => Self::E0483,
            484 => Self::E0484,
            485 => Self::E0485,
            486 => Self::E0486,
            487 => Self::E0487,
            488 => Self::E0488,
            489 => Self::E0489,
            490 => Self::E0490,
            491 => Self::E0491,
            492 => Self::E0492,
            493 => Self::E0493,
            494 => Self::E0494,
            495 => Self::E0495,
            496 => Self::E0496,
            497 => Self::E0497,
            498 => Self::E0498,
            499 => Self::E0499,
            500 => Self::E0500,
            _ => unreachable!(),
        })
    }
}

/// Builtin type names (edition 2).
pub const BUILTIN_TYPES: &[&str] = &[
    "i32",
    "i64",
    "u32",
    "u64",
    "f32",
    "f64",
    "bool",
    "str",
    "String",
    "()",
    "MsgId",
    "SceneId",
    "LayerId",
    "ImageHandle",
    "AudioHandle",
    "EntityId",
    "Result",
    "Option",
    "Array",
    "Map",
    "Duration",
    "Color",
    "Vec2",
    "Transform",
    "Transition",
    "Anchor",
    "Channel",
    "ScriptError",
    "Action",
    "StyleId",
];

/// Builtin type check.
pub fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains(&name)
}

/// Crate version.
pub fn crate_version() -> &'static str { env!("CARGO_PKG_VERSION") }
/// Crate name.
pub fn crate_name() -> &'static str { env!("CARGO_PKG_NAME") }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keywords_roundtrip() {
        for k in Keyword::all() {
            assert_eq!(Keyword::from_str(k.as_str()), Some(*k));
        }
    }
    #[test]
    fn diag_count() {
        assert_eq!(DiagCode::all().count(), 500);
    }
    #[test]
    fn edition() {
        assert_eq!(Edition::from_u32(2), Some(Edition::V2));
    }
    #[test]
    fn kw_0_askw() { assert_eq!(Keyword::AsKw.as_str(), "as"); }
    #[test]
    fn kw_1_asynckw() { assert_eq!(Keyword::AsyncKw.as_str(), "async"); }
    #[test]
    fn kw_2_awaitkw() { assert_eq!(Keyword::AwaitKw.as_str(), "await"); }
    #[test]
    fn kw_3_breakkw() { assert_eq!(Keyword::BreakKw.as_str(), "break"); }
    #[test]
    fn kw_4_callkw() { assert_eq!(Keyword::CallKw.as_str(), "call"); }
    #[test]
    fn kw_5_characterkw() { assert_eq!(Keyword::CharacterKw.as_str(), "character"); }
    #[test]
    fn kw_6_constkw() { assert_eq!(Keyword::ConstKw.as_str(), "const"); }
    #[test]
    fn kw_7_continuekw() { assert_eq!(Keyword::ContinueKw.as_str(), "continue"); }
    #[test]
    fn kw_8_cratekw() { assert_eq!(Keyword::CrateKw.as_str(), "crate"); }
    #[test]
    fn kw_9_elsekw() { assert_eq!(Keyword::ElseKw.as_str(), "else"); }
    #[test]
    fn kw_10_enumkw() { assert_eq!(Keyword::EnumKw.as_str(), "enum"); }
    #[test]
    fn kw_11_externkw() { assert_eq!(Keyword::ExternKw.as_str(), "extern"); }
    #[test]
    fn kw_12_falsekw() { assert_eq!(Keyword::FalseKw.as_str(), "false"); }
    #[test]
    fn kw_13_fnkw() { assert_eq!(Keyword::FnKw.as_str(), "fn"); }
    #[test]
    fn kw_14_forkw() { assert_eq!(Keyword::ForKw.as_str(), "for"); }
    #[test]
    fn kw_15_functionkw() { assert_eq!(Keyword::FunctionKw.as_str(), "function"); }
    #[test]
    fn kw_16_ifkw() { assert_eq!(Keyword::IfKw.as_str(), "if"); }
    #[test]
    fn kw_17_implkw() { assert_eq!(Keyword::ImplKw.as_str(), "impl"); }
    #[test]
    fn kw_18_importkw() { assert_eq!(Keyword::ImportKw.as_str(), "import"); }
    #[test]
    fn kw_19_inkw() { assert_eq!(Keyword::InKw.as_str(), "in"); }
    #[test]
    fn kw_20_jumpkw() { assert_eq!(Keyword::JumpKw.as_str(), "jump"); }
    #[test]
    fn kw_21_letkw() { assert_eq!(Keyword::LetKw.as_str(), "let"); }
    #[test]
    fn kw_22_loopkw() { assert_eq!(Keyword::LoopKw.as_str(), "loop"); }
    #[test]
    fn kw_23_matchkw() { assert_eq!(Keyword::MatchKw.as_str(), "match"); }
    #[test]
    fn kw_24_menukw() { assert_eq!(Keyword::MenuKw.as_str(), "menu"); }
    #[test]
    fn kw_25_modkw() { assert_eq!(Keyword::ModKw.as_str(), "mod"); }
    #[test]
    fn kw_26_movekw() { assert_eq!(Keyword::MoveKw.as_str(), "move"); }
    #[test]
    fn kw_27_mutkw() { assert_eq!(Keyword::MutKw.as_str(), "mut"); }
    #[test]
    fn kw_28_pubkw() { assert_eq!(Keyword::PubKw.as_str(), "pub"); }
    #[test]
    fn kw_29_refkw() { assert_eq!(Keyword::RefKw.as_str(), "ref"); }
    #[test]
    fn kw_30_returnkw() { assert_eq!(Keyword::ReturnKw.as_str(), "return"); }
    #[test]
    fn kw_31_scenekw() { assert_eq!(Keyword::SceneKw.as_str(), "scene"); }
    #[test]
    fn kw_32_screenkw() { assert_eq!(Keyword::ScreenKw.as_str(), "screen"); }
    #[test]
    fn kw_33_selfvalue() { assert_eq!(Keyword::SelfValue.as_str(), "self"); }
    #[test]
    fn kw_34_selftype() { assert_eq!(Keyword::SelfType.as_str(), "Self"); }
    #[test]
    fn kw_35_showkw() { assert_eq!(Keyword::ShowKw.as_str(), "show"); }
    #[test]
    fn kw_36_hidekw() { assert_eq!(Keyword::HideKw.as_str(), "hide"); }
    #[test]
    fn kw_37_saykw() { assert_eq!(Keyword::SayKw.as_str(), "say"); }
    #[test]
    fn kw_38_statekw() { assert_eq!(Keyword::StateKw.as_str(), "state"); }
    #[test]
    fn kw_39_statickw() { assert_eq!(Keyword::StaticKw.as_str(), "static"); }
    #[test]
    fn kw_40_structkw() { assert_eq!(Keyword::StructKw.as_str(), "struct"); }
    #[test]
    fn kw_41_superkw() { assert_eq!(Keyword::SuperKw.as_str(), "super"); }
    #[test]
    fn kw_42_traitkw() { assert_eq!(Keyword::TraitKw.as_str(), "trait"); }
    #[test]
    fn kw_43_truekw() { assert_eq!(Keyword::TrueKw.as_str(), "true"); }
    #[test]
    fn kw_44_typekw() { assert_eq!(Keyword::TypeKw.as_str(), "type"); }
    #[test]
    fn kw_45_usekw() { assert_eq!(Keyword::UseKw.as_str(), "use"); }
    #[test]
    fn kw_46_wherekw() { assert_eq!(Keyword::WhereKw.as_str(), "where"); }
    #[test]
    fn kw_47_whilekw() { assert_eq!(Keyword::WhileKw.as_str(), "while"); }
    #[test]
    fn kw_48_withkw() { assert_eq!(Keyword::WithKw.as_str(), "with"); }
    #[test]
    fn kw_49_transformkw() { assert_eq!(Keyword::TransformKw.as_str(), "transform"); }
    #[test]
    fn kw_50_layerkw() { assert_eq!(Keyword::LayerKw.as_str(), "layer"); }
    #[test]
    fn kw_51_backgroundkw() { assert_eq!(Keyword::BackgroundKw.as_str(), "background"); }
    #[test]
    fn kw_52_musickw() { assert_eq!(Keyword::MusicKw.as_str(), "music"); }
    #[test]
    fn kw_53_choicekw() { assert_eq!(Keyword::ChoiceKw.as_str(), "choice"); }
    #[test]
    fn kw_54_optionkw() { assert_eq!(Keyword::OptionKw.as_str(), "option"); }
    #[test]
    fn kw_55_okkw() { assert_eq!(Keyword::OkKw.as_str(), "Ok"); }
    #[test]
    fn kw_56_errkw() { assert_eq!(Keyword::ErrKw.as_str(), "Err"); }
    #[test]
    fn kw_57_somekw() { assert_eq!(Keyword::SomeKw.as_str(), "Some"); }
    #[test]
    fn kw_58_nonekw() { assert_eq!(Keyword::NoneKw.as_str(), "None"); }
    #[test]
    fn kw_59_resultkw() { assert_eq!(Keyword::ResultKw.as_str(), "Result"); }
    #[test]
    fn kw_60_optiontypekw() { assert_eq!(Keyword::OptionTypeKw.as_str(), "Option"); }
    #[test]
    fn kw_61_trykw() { assert_eq!(Keyword::TryKw.as_str(), "try"); }
    #[test]
    fn diag_e0001() { assert_eq!(DiagCode::E0001.code(), 1); }
    #[test]
    fn diag_e0002() { assert_eq!(DiagCode::E0002.code(), 2); }
    #[test]
    fn diag_e0003() { assert_eq!(DiagCode::E0003.code(), 3); }
    #[test]
    fn diag_e0004() { assert_eq!(DiagCode::E0004.code(), 4); }
    #[test]
    fn diag_e0005() { assert_eq!(DiagCode::E0005.code(), 5); }
    #[test]
    fn diag_e0006() { assert_eq!(DiagCode::E0006.code(), 6); }
    #[test]
    fn diag_e0007() { assert_eq!(DiagCode::E0007.code(), 7); }
    #[test]
    fn diag_e0008() { assert_eq!(DiagCode::E0008.code(), 8); }
    #[test]
    fn diag_e0009() { assert_eq!(DiagCode::E0009.code(), 9); }
    #[test]
    fn diag_e0010() { assert_eq!(DiagCode::E0010.code(), 10); }
    #[test]
    fn diag_e0011() { assert_eq!(DiagCode::E0011.code(), 11); }
    #[test]
    fn diag_e0012() { assert_eq!(DiagCode::E0012.code(), 12); }
    #[test]
    fn diag_e0013() { assert_eq!(DiagCode::E0013.code(), 13); }
    #[test]
    fn diag_e0014() { assert_eq!(DiagCode::E0014.code(), 14); }
    #[test]
    fn diag_e0015() { assert_eq!(DiagCode::E0015.code(), 15); }
    #[test]
    fn diag_e0016() { assert_eq!(DiagCode::E0016.code(), 16); }
    #[test]
    fn diag_e0017() { assert_eq!(DiagCode::E0017.code(), 17); }
    #[test]
    fn diag_e0018() { assert_eq!(DiagCode::E0018.code(), 18); }
    #[test]
    fn diag_e0019() { assert_eq!(DiagCode::E0019.code(), 19); }
    #[test]
    fn diag_e0020() { assert_eq!(DiagCode::E0020.code(), 20); }
    #[test]
    fn diag_e0021() { assert_eq!(DiagCode::E0021.code(), 21); }
    #[test]
    fn diag_e0022() { assert_eq!(DiagCode::E0022.code(), 22); }
    #[test]
    fn diag_e0023() { assert_eq!(DiagCode::E0023.code(), 23); }
    #[test]
    fn diag_e0024() { assert_eq!(DiagCode::E0024.code(), 24); }
    #[test]
    fn diag_e0025() { assert_eq!(DiagCode::E0025.code(), 25); }
    #[test]
    fn diag_e0026() { assert_eq!(DiagCode::E0026.code(), 26); }
    #[test]
    fn diag_e0027() { assert_eq!(DiagCode::E0027.code(), 27); }
    #[test]
    fn diag_e0028() { assert_eq!(DiagCode::E0028.code(), 28); }
    #[test]
    fn diag_e0029() { assert_eq!(DiagCode::E0029.code(), 29); }
    #[test]
    fn diag_e0030() { assert_eq!(DiagCode::E0030.code(), 30); }
    #[test]
    fn diag_e0031() { assert_eq!(DiagCode::E0031.code(), 31); }
    #[test]
    fn diag_e0032() { assert_eq!(DiagCode::E0032.code(), 32); }
    #[test]
    fn diag_e0033() { assert_eq!(DiagCode::E0033.code(), 33); }
    #[test]
    fn diag_e0034() { assert_eq!(DiagCode::E0034.code(), 34); }
    #[test]
    fn diag_e0035() { assert_eq!(DiagCode::E0035.code(), 35); }
    #[test]
    fn diag_e0036() { assert_eq!(DiagCode::E0036.code(), 36); }
    #[test]
    fn diag_e0037() { assert_eq!(DiagCode::E0037.code(), 37); }
    #[test]
    fn diag_e0038() { assert_eq!(DiagCode::E0038.code(), 38); }
    #[test]
    fn diag_e0039() { assert_eq!(DiagCode::E0039.code(), 39); }
    #[test]
    fn diag_e0040() { assert_eq!(DiagCode::E0040.code(), 40); }
    #[test]
    fn diag_e0041() { assert_eq!(DiagCode::E0041.code(), 41); }
    #[test]
    fn diag_e0042() { assert_eq!(DiagCode::E0042.code(), 42); }
    #[test]
    fn diag_e0043() { assert_eq!(DiagCode::E0043.code(), 43); }
    #[test]
    fn diag_e0044() { assert_eq!(DiagCode::E0044.code(), 44); }
    #[test]
    fn diag_e0045() { assert_eq!(DiagCode::E0045.code(), 45); }
    #[test]
    fn diag_e0046() { assert_eq!(DiagCode::E0046.code(), 46); }
    #[test]
    fn diag_e0047() { assert_eq!(DiagCode::E0047.code(), 47); }
    #[test]
    fn diag_e0048() { assert_eq!(DiagCode::E0048.code(), 48); }
    #[test]
    fn diag_e0049() { assert_eq!(DiagCode::E0049.code(), 49); }
    #[test]
    fn diag_e0050() { assert_eq!(DiagCode::E0050.code(), 50); }
    #[test]
    fn diag_e0051() { assert_eq!(DiagCode::E0051.code(), 51); }
    #[test]
    fn diag_e0052() { assert_eq!(DiagCode::E0052.code(), 52); }
    #[test]
    fn diag_e0053() { assert_eq!(DiagCode::E0053.code(), 53); }
    #[test]
    fn diag_e0054() { assert_eq!(DiagCode::E0054.code(), 54); }
    #[test]
    fn diag_e0055() { assert_eq!(DiagCode::E0055.code(), 55); }
    #[test]
    fn diag_e0056() { assert_eq!(DiagCode::E0056.code(), 56); }
    #[test]
    fn diag_e0057() { assert_eq!(DiagCode::E0057.code(), 57); }
    #[test]
    fn diag_e0058() { assert_eq!(DiagCode::E0058.code(), 58); }
    #[test]
    fn diag_e0059() { assert_eq!(DiagCode::E0059.code(), 59); }
    #[test]
    fn diag_e0060() { assert_eq!(DiagCode::E0060.code(), 60); }
    #[test]
    fn diag_e0061() { assert_eq!(DiagCode::E0061.code(), 61); }
    #[test]
    fn diag_e0062() { assert_eq!(DiagCode::E0062.code(), 62); }
    #[test]
    fn diag_e0063() { assert_eq!(DiagCode::E0063.code(), 63); }
    #[test]
    fn diag_e0064() { assert_eq!(DiagCode::E0064.code(), 64); }
    #[test]
    fn diag_e0065() { assert_eq!(DiagCode::E0065.code(), 65); }
    #[test]
    fn diag_e0066() { assert_eq!(DiagCode::E0066.code(), 66); }
    #[test]
    fn diag_e0067() { assert_eq!(DiagCode::E0067.code(), 67); }
    #[test]
    fn diag_e0068() { assert_eq!(DiagCode::E0068.code(), 68); }
    #[test]
    fn diag_e0069() { assert_eq!(DiagCode::E0069.code(), 69); }
    #[test]
    fn diag_e0070() { assert_eq!(DiagCode::E0070.code(), 70); }
    #[test]
    fn diag_e0071() { assert_eq!(DiagCode::E0071.code(), 71); }
    #[test]
    fn diag_e0072() { assert_eq!(DiagCode::E0072.code(), 72); }
    #[test]
    fn diag_e0073() { assert_eq!(DiagCode::E0073.code(), 73); }
    #[test]
    fn diag_e0074() { assert_eq!(DiagCode::E0074.code(), 74); }
    #[test]
    fn diag_e0075() { assert_eq!(DiagCode::E0075.code(), 75); }
    #[test]
    fn diag_e0076() { assert_eq!(DiagCode::E0076.code(), 76); }
    #[test]
    fn diag_e0077() { assert_eq!(DiagCode::E0077.code(), 77); }
    #[test]
    fn diag_e0078() { assert_eq!(DiagCode::E0078.code(), 78); }
    #[test]
    fn diag_e0079() { assert_eq!(DiagCode::E0079.code(), 79); }
    #[test]
    fn diag_e0080() { assert_eq!(DiagCode::E0080.code(), 80); }
    #[test]
    fn diag_e0081() { assert_eq!(DiagCode::E0081.code(), 81); }
    #[test]
    fn diag_e0082() { assert_eq!(DiagCode::E0082.code(), 82); }
    #[test]
    fn diag_e0083() { assert_eq!(DiagCode::E0083.code(), 83); }
    #[test]
    fn diag_e0084() { assert_eq!(DiagCode::E0084.code(), 84); }
    #[test]
    fn diag_e0085() { assert_eq!(DiagCode::E0085.code(), 85); }
    #[test]
    fn diag_e0086() { assert_eq!(DiagCode::E0086.code(), 86); }
    #[test]
    fn diag_e0087() { assert_eq!(DiagCode::E0087.code(), 87); }
    #[test]
    fn diag_e0088() { assert_eq!(DiagCode::E0088.code(), 88); }
    #[test]
    fn diag_e0089() { assert_eq!(DiagCode::E0089.code(), 89); }
    #[test]
    fn diag_e0090() { assert_eq!(DiagCode::E0090.code(), 90); }
    #[test]
    fn diag_e0091() { assert_eq!(DiagCode::E0091.code(), 91); }
    #[test]
    fn diag_e0092() { assert_eq!(DiagCode::E0092.code(), 92); }
    #[test]
    fn diag_e0093() { assert_eq!(DiagCode::E0093.code(), 93); }
    #[test]
    fn diag_e0094() { assert_eq!(DiagCode::E0094.code(), 94); }
    #[test]
    fn diag_e0095() { assert_eq!(DiagCode::E0095.code(), 95); }
    #[test]
    fn diag_e0096() { assert_eq!(DiagCode::E0096.code(), 96); }
    #[test]
    fn diag_e0097() { assert_eq!(DiagCode::E0097.code(), 97); }
    #[test]
    fn diag_e0098() { assert_eq!(DiagCode::E0098.code(), 98); }
    #[test]
    fn diag_e0099() { assert_eq!(DiagCode::E0099.code(), 99); }
    #[test]
    fn diag_e0100() { assert_eq!(DiagCode::E0100.code(), 100); }
    #[test]
    fn diag_e0101() { assert_eq!(DiagCode::E0101.code(), 101); }
    #[test]
    fn diag_e0102() { assert_eq!(DiagCode::E0102.code(), 102); }
    #[test]
    fn diag_e0103() { assert_eq!(DiagCode::E0103.code(), 103); }
    #[test]
    fn diag_e0104() { assert_eq!(DiagCode::E0104.code(), 104); }
    #[test]
    fn diag_e0105() { assert_eq!(DiagCode::E0105.code(), 105); }
    #[test]
    fn diag_e0106() { assert_eq!(DiagCode::E0106.code(), 106); }
    #[test]
    fn diag_e0107() { assert_eq!(DiagCode::E0107.code(), 107); }
    #[test]
    fn diag_e0108() { assert_eq!(DiagCode::E0108.code(), 108); }
    #[test]
    fn diag_e0109() { assert_eq!(DiagCode::E0109.code(), 109); }
    #[test]
    fn diag_e0110() { assert_eq!(DiagCode::E0110.code(), 110); }
    #[test]
    fn diag_e0111() { assert_eq!(DiagCode::E0111.code(), 111); }
    #[test]
    fn diag_e0112() { assert_eq!(DiagCode::E0112.code(), 112); }
    #[test]
    fn diag_e0113() { assert_eq!(DiagCode::E0113.code(), 113); }
    #[test]
    fn diag_e0114() { assert_eq!(DiagCode::E0114.code(), 114); }
    #[test]
    fn diag_e0115() { assert_eq!(DiagCode::E0115.code(), 115); }
    #[test]
    fn diag_e0116() { assert_eq!(DiagCode::E0116.code(), 116); }
    #[test]
    fn diag_e0117() { assert_eq!(DiagCode::E0117.code(), 117); }
    #[test]
    fn diag_e0118() { assert_eq!(DiagCode::E0118.code(), 118); }
    #[test]
    fn diag_e0119() { assert_eq!(DiagCode::E0119.code(), 119); }
    #[test]
    fn diag_e0120() { assert_eq!(DiagCode::E0120.code(), 120); }
    #[test]
    fn diag_e0121() { assert_eq!(DiagCode::E0121.code(), 121); }
    #[test]
    fn diag_e0122() { assert_eq!(DiagCode::E0122.code(), 122); }
    #[test]
    fn diag_e0123() { assert_eq!(DiagCode::E0123.code(), 123); }
    #[test]
    fn diag_e0124() { assert_eq!(DiagCode::E0124.code(), 124); }
    #[test]
    fn diag_e0125() { assert_eq!(DiagCode::E0125.code(), 125); }
    #[test]
    fn diag_e0126() { assert_eq!(DiagCode::E0126.code(), 126); }
    #[test]
    fn diag_e0127() { assert_eq!(DiagCode::E0127.code(), 127); }
    #[test]
    fn diag_e0128() { assert_eq!(DiagCode::E0128.code(), 128); }
    #[test]
    fn diag_e0129() { assert_eq!(DiagCode::E0129.code(), 129); }
    #[test]
    fn diag_e0130() { assert_eq!(DiagCode::E0130.code(), 130); }
    #[test]
    fn diag_e0131() { assert_eq!(DiagCode::E0131.code(), 131); }
    #[test]
    fn diag_e0132() { assert_eq!(DiagCode::E0132.code(), 132); }
    #[test]
    fn diag_e0133() { assert_eq!(DiagCode::E0133.code(), 133); }
    #[test]
    fn diag_e0134() { assert_eq!(DiagCode::E0134.code(), 134); }
    #[test]
    fn diag_e0135() { assert_eq!(DiagCode::E0135.code(), 135); }
    #[test]
    fn diag_e0136() { assert_eq!(DiagCode::E0136.code(), 136); }
    #[test]
    fn diag_e0137() { assert_eq!(DiagCode::E0137.code(), 137); }
    #[test]
    fn diag_e0138() { assert_eq!(DiagCode::E0138.code(), 138); }
    #[test]
    fn diag_e0139() { assert_eq!(DiagCode::E0139.code(), 139); }
    #[test]
    fn diag_e0140() { assert_eq!(DiagCode::E0140.code(), 140); }
    #[test]
    fn diag_e0141() { assert_eq!(DiagCode::E0141.code(), 141); }
    #[test]
    fn diag_e0142() { assert_eq!(DiagCode::E0142.code(), 142); }
    #[test]
    fn diag_e0143() { assert_eq!(DiagCode::E0143.code(), 143); }
    #[test]
    fn diag_e0144() { assert_eq!(DiagCode::E0144.code(), 144); }
    #[test]
    fn diag_e0145() { assert_eq!(DiagCode::E0145.code(), 145); }
    #[test]
    fn diag_e0146() { assert_eq!(DiagCode::E0146.code(), 146); }
    #[test]
    fn diag_e0147() { assert_eq!(DiagCode::E0147.code(), 147); }
    #[test]
    fn diag_e0148() { assert_eq!(DiagCode::E0148.code(), 148); }
    #[test]
    fn diag_e0149() { assert_eq!(DiagCode::E0149.code(), 149); }
    #[test]
    fn diag_e0150() { assert_eq!(DiagCode::E0150.code(), 150); }
    #[test]
    fn diag_e0151() { assert_eq!(DiagCode::E0151.code(), 151); }
    #[test]
    fn diag_e0152() { assert_eq!(DiagCode::E0152.code(), 152); }
    #[test]
    fn diag_e0153() { assert_eq!(DiagCode::E0153.code(), 153); }
    #[test]
    fn diag_e0154() { assert_eq!(DiagCode::E0154.code(), 154); }
    #[test]
    fn diag_e0155() { assert_eq!(DiagCode::E0155.code(), 155); }
    #[test]
    fn diag_e0156() { assert_eq!(DiagCode::E0156.code(), 156); }
    #[test]
    fn diag_e0157() { assert_eq!(DiagCode::E0157.code(), 157); }
    #[test]
    fn diag_e0158() { assert_eq!(DiagCode::E0158.code(), 158); }
    #[test]
    fn diag_e0159() { assert_eq!(DiagCode::E0159.code(), 159); }
    #[test]
    fn diag_e0160() { assert_eq!(DiagCode::E0160.code(), 160); }
    #[test]
    fn diag_e0161() { assert_eq!(DiagCode::E0161.code(), 161); }
    #[test]
    fn diag_e0162() { assert_eq!(DiagCode::E0162.code(), 162); }
    #[test]
    fn diag_e0163() { assert_eq!(DiagCode::E0163.code(), 163); }
    #[test]
    fn diag_e0164() { assert_eq!(DiagCode::E0164.code(), 164); }
    #[test]
    fn diag_e0165() { assert_eq!(DiagCode::E0165.code(), 165); }
    #[test]
    fn diag_e0166() { assert_eq!(DiagCode::E0166.code(), 166); }
    #[test]
    fn diag_e0167() { assert_eq!(DiagCode::E0167.code(), 167); }
    #[test]
    fn diag_e0168() { assert_eq!(DiagCode::E0168.code(), 168); }
    #[test]
    fn diag_e0169() { assert_eq!(DiagCode::E0169.code(), 169); }
    #[test]
    fn diag_e0170() { assert_eq!(DiagCode::E0170.code(), 170); }
    #[test]
    fn diag_e0171() { assert_eq!(DiagCode::E0171.code(), 171); }
    #[test]
    fn diag_e0172() { assert_eq!(DiagCode::E0172.code(), 172); }
    #[test]
    fn diag_e0173() { assert_eq!(DiagCode::E0173.code(), 173); }
    #[test]
    fn diag_e0174() { assert_eq!(DiagCode::E0174.code(), 174); }
    #[test]
    fn diag_e0175() { assert_eq!(DiagCode::E0175.code(), 175); }
    #[test]
    fn diag_e0176() { assert_eq!(DiagCode::E0176.code(), 176); }
    #[test]
    fn diag_e0177() { assert_eq!(DiagCode::E0177.code(), 177); }
    #[test]
    fn diag_e0178() { assert_eq!(DiagCode::E0178.code(), 178); }
    #[test]
    fn diag_e0179() { assert_eq!(DiagCode::E0179.code(), 179); }
    #[test]
    fn diag_e0180() { assert_eq!(DiagCode::E0180.code(), 180); }
    #[test]
    fn diag_e0181() { assert_eq!(DiagCode::E0181.code(), 181); }
    #[test]
    fn diag_e0182() { assert_eq!(DiagCode::E0182.code(), 182); }
    #[test]
    fn diag_e0183() { assert_eq!(DiagCode::E0183.code(), 183); }
    #[test]
    fn diag_e0184() { assert_eq!(DiagCode::E0184.code(), 184); }
    #[test]
    fn diag_e0185() { assert_eq!(DiagCode::E0185.code(), 185); }
    #[test]
    fn diag_e0186() { assert_eq!(DiagCode::E0186.code(), 186); }
    #[test]
    fn diag_e0187() { assert_eq!(DiagCode::E0187.code(), 187); }
    #[test]
    fn diag_e0188() { assert_eq!(DiagCode::E0188.code(), 188); }
    #[test]
    fn diag_e0189() { assert_eq!(DiagCode::E0189.code(), 189); }
    #[test]
    fn diag_e0190() { assert_eq!(DiagCode::E0190.code(), 190); }
    #[test]
    fn diag_e0191() { assert_eq!(DiagCode::E0191.code(), 191); }
    #[test]
    fn diag_e0192() { assert_eq!(DiagCode::E0192.code(), 192); }
    #[test]
    fn diag_e0193() { assert_eq!(DiagCode::E0193.code(), 193); }
    #[test]
    fn diag_e0194() { assert_eq!(DiagCode::E0194.code(), 194); }
    #[test]
    fn diag_e0195() { assert_eq!(DiagCode::E0195.code(), 195); }
    #[test]
    fn diag_e0196() { assert_eq!(DiagCode::E0196.code(), 196); }
    #[test]
    fn diag_e0197() { assert_eq!(DiagCode::E0197.code(), 197); }
    #[test]
    fn diag_e0198() { assert_eq!(DiagCode::E0198.code(), 198); }
    #[test]
    fn diag_e0199() { assert_eq!(DiagCode::E0199.code(), 199); }
    #[test]
    fn diag_e0200() { assert_eq!(DiagCode::E0200.code(), 200); }
}
