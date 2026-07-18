//! Velvet Script 2 opcode catalog (real ops only — no ReservedN padding).

#![allow(missing_docs)]

/// Extended opcode id for VS2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum OpVs2 {
    Nop = 0,
    LoadConst = 1,
    LoadLocal = 2,
    StoreLocal = 3,
    Add = 4,
    Sub = 5,
    Mul = 6,
    Div = 7,
    Rem = 8,
    Eq = 9,
    Ne = 10,
    Lt = 11,
    Le = 12,
    Gt = 13,
    Ge = 14,
    And = 15,
    Or = 16,
    Not = 17,
    Jump = 18,
    JumpIf = 19,
    Call = 20,
    Ret = 21,
    Print = 22,
    Pop = 23,
    Dup = 24,
    Say = 25,
    Menu = 26,
    Choice = 27,
    JumpScene = 28,
    CallScene = 29,
    ShowChar = 30,
    HideChar = 31,
    Background = 32,
    Music = 33,
    PushLayer = 34,
    PopLayer = 35,
    ShowLayer = 36,
    HideLayer = 37,
    SetLayerZ = 38,
    Translate = 39,
    Await = 40,
    Yield = 41,
    LoadMsg = 42,
    StoreState = 43,
    LoadState = 44,
    MakeArray = 45,
    IndexGet = 46,
    IndexSet = 47,
    MakeMap = 48,
    MapGet = 49,
    MapSet = 50,
    Ok = 51,
    Err = 52,
    Some = 53,
    None_ = 54,
    Try = 55,
    IsOk = 56,
    Unwrap = 57,
    CastI32 = 58,
    CastF64 = 59,
    Concat = 60,
    Len = 61,
    TransformApply = 62,
    TransitionPlay = 63,
    ActionFire = 64,
    ScreenOpen = 65,
    ScreenClose = 66,
    BindButton = 67,
    PlaySfx = 68,
    PlayVoice = 69,
    StopBgm = 70,
    SetVolume = 71,
}

/// Last defined opcode discriminant + 1.
pub const OP_VS2_COUNT: u16 = 72;

impl OpVs2 {
    /// Raw u16.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Stable name for disassembly.
    pub fn name(self) -> &'static str {
        match self {
            Self::Nop => "Nop",
            Self::LoadConst => "LoadConst",
            Self::LoadLocal => "LoadLocal",
            Self::StoreLocal => "StoreLocal",
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Mul => "Mul",
            Self::Div => "Div",
            Self::Rem => "Rem",
            Self::Eq => "Eq",
            Self::Ne => "Ne",
            Self::Lt => "Lt",
            Self::Le => "Le",
            Self::Gt => "Gt",
            Self::Ge => "Ge",
            Self::And => "And",
            Self::Or => "Or",
            Self::Not => "Not",
            Self::Jump => "Jump",
            Self::JumpIf => "JumpIf",
            Self::Call => "Call",
            Self::Ret => "Ret",
            Self::Print => "Print",
            Self::Pop => "Pop",
            Self::Dup => "Dup",
            Self::Say => "Say",
            Self::Menu => "Menu",
            Self::Choice => "Choice",
            Self::JumpScene => "JumpScene",
            Self::CallScene => "CallScene",
            Self::ShowChar => "ShowChar",
            Self::HideChar => "HideChar",
            Self::Background => "Background",
            Self::Music => "Music",
            Self::PushLayer => "PushLayer",
            Self::PopLayer => "PopLayer",
            Self::ShowLayer => "ShowLayer",
            Self::HideLayer => "HideLayer",
            Self::SetLayerZ => "SetLayerZ",
            Self::Translate => "Translate",
            Self::Await => "Await",
            Self::Yield => "Yield",
            Self::LoadMsg => "LoadMsg",
            Self::StoreState => "StoreState",
            Self::LoadState => "LoadState",
            Self::MakeArray => "MakeArray",
            Self::IndexGet => "IndexGet",
            Self::IndexSet => "IndexSet",
            Self::MakeMap => "MakeMap",
            Self::MapGet => "MapGet",
            Self::MapSet => "MapSet",
            Self::Ok => "Ok",
            Self::Err => "Err",
            Self::Some => "Some",
            Self::None_ => "None_",
            Self::Try => "Try",
            Self::IsOk => "IsOk",
            Self::Unwrap => "Unwrap",
            Self::CastI32 => "CastI32",
            Self::CastF64 => "CastF64",
            Self::Concat => "Concat",
            Self::Len => "Len",
            Self::TransformApply => "TransformApply",
            Self::TransitionPlay => "TransitionPlay",
            Self::ActionFire => "ActionFire",
            Self::ScreenOpen => "ScreenOpen",
            Self::ScreenClose => "ScreenClose",
            Self::BindButton => "BindButton",
            Self::PlaySfx => "PlaySfx",
            Self::PlayVoice => "PlayVoice",
            Self::StopBgm => "StopBgm",
            Self::SetVolume => "SetVolume",
        }
    }

    /// Decode from raw id; unknown ids are `None` (no reserved slots).
    pub fn from_u16(v: u16) -> Option<Self> {
        if v >= OP_VS2_COUNT {
            return None;
        }
        // Safety: discriminants 0..71 are dense and match the enum layout.
        Some(match v {
            0 => Self::Nop,
            1 => Self::LoadConst,
            2 => Self::LoadLocal,
            3 => Self::StoreLocal,
            4 => Self::Add,
            5 => Self::Sub,
            6 => Self::Mul,
            7 => Self::Div,
            8 => Self::Rem,
            9 => Self::Eq,
            10 => Self::Ne,
            11 => Self::Lt,
            12 => Self::Le,
            13 => Self::Gt,
            14 => Self::Ge,
            15 => Self::And,
            16 => Self::Or,
            17 => Self::Not,
            18 => Self::Jump,
            19 => Self::JumpIf,
            20 => Self::Call,
            21 => Self::Ret,
            22 => Self::Print,
            23 => Self::Pop,
            24 => Self::Dup,
            25 => Self::Say,
            26 => Self::Menu,
            27 => Self::Choice,
            28 => Self::JumpScene,
            29 => Self::CallScene,
            30 => Self::ShowChar,
            31 => Self::HideChar,
            32 => Self::Background,
            33 => Self::Music,
            34 => Self::PushLayer,
            35 => Self::PopLayer,
            36 => Self::ShowLayer,
            37 => Self::HideLayer,
            38 => Self::SetLayerZ,
            39 => Self::Translate,
            40 => Self::Await,
            41 => Self::Yield,
            42 => Self::LoadMsg,
            43 => Self::StoreState,
            44 => Self::LoadState,
            45 => Self::MakeArray,
            46 => Self::IndexGet,
            47 => Self::IndexSet,
            48 => Self::MakeMap,
            49 => Self::MapGet,
            50 => Self::MapSet,
            51 => Self::Ok,
            52 => Self::Err,
            53 => Self::Some,
            54 => Self::None_,
            55 => Self::Try,
            56 => Self::IsOk,
            57 => Self::Unwrap,
            58 => Self::CastI32,
            59 => Self::CastF64,
            60 => Self::Concat,
            61 => Self::Len,
            62 => Self::TransformApply,
            63 => Self::TransitionPlay,
            64 => Self::ActionFire,
            65 => Self::ScreenOpen,
            66 => Self::ScreenClose,
            67 => Self::BindButton,
            68 => Self::PlaySfx,
            69 => Self::PlayVoice,
            70 => Self::StopBgm,
            71 => Self::SetVolume,
            _ => return None,
        })
    }

    /// All defined opcodes in id order.
    pub fn all() -> &'static [OpVs2] {
        &[
            Self::Nop,
            Self::LoadConst,
            Self::LoadLocal,
            Self::StoreLocal,
            Self::Add,
            Self::Sub,
            Self::Mul,
            Self::Div,
            Self::Rem,
            Self::Eq,
            Self::Ne,
            Self::Lt,
            Self::Le,
            Self::Gt,
            Self::Ge,
            Self::And,
            Self::Or,
            Self::Not,
            Self::Jump,
            Self::JumpIf,
            Self::Call,
            Self::Ret,
            Self::Print,
            Self::Pop,
            Self::Dup,
            Self::Say,
            Self::Menu,
            Self::Choice,
            Self::JumpScene,
            Self::CallScene,
            Self::ShowChar,
            Self::HideChar,
            Self::Background,
            Self::Music,
            Self::PushLayer,
            Self::PopLayer,
            Self::ShowLayer,
            Self::HideLayer,
            Self::SetLayerZ,
            Self::Translate,
            Self::Await,
            Self::Yield,
            Self::LoadMsg,
            Self::StoreState,
            Self::LoadState,
            Self::MakeArray,
            Self::IndexGet,
            Self::IndexSet,
            Self::MakeMap,
            Self::MapGet,
            Self::MapSet,
            Self::Ok,
            Self::Err,
            Self::Some,
            Self::None_,
            Self::Try,
            Self::IsOk,
            Self::Unwrap,
            Self::CastI32,
            Self::CastF64,
            Self::Concat,
            Self::Len,
            Self::TransformApply,
            Self::TransitionPlay,
            Self::ActionFire,
            Self::ScreenOpen,
            Self::ScreenClose,
            Self::BindButton,
            Self::PlaySfx,
            Self::PlayVoice,
            Self::StopBgm,
            Self::SetVolume,
        ]
    }
}

/// Approximate stack effect `(pops, pushes)` for analysis.
pub fn stack_effect(op: OpVs2) -> (i8, i8) {
    match op {
        OpVs2::Nop | OpVs2::Jump | OpVs2::Ret | OpVs2::Await | OpVs2::Yield => (0, 0),
        OpVs2::LoadConst
        | OpVs2::LoadLocal
        | OpVs2::LoadMsg
        | OpVs2::LoadState
        | OpVs2::Dup
        | OpVs2::None_ => (0, 1),
        OpVs2::StoreLocal | OpVs2::StoreState | OpVs2::Pop | OpVs2::Print | OpVs2::JumpIf => (1, 0),
        OpVs2::Add
        | OpVs2::Sub
        | OpVs2::Mul
        | OpVs2::Div
        | OpVs2::Rem
        | OpVs2::Eq
        | OpVs2::Ne
        | OpVs2::Lt
        | OpVs2::Le
        | OpVs2::Gt
        | OpVs2::Ge
        | OpVs2::And
        | OpVs2::Or
        | OpVs2::IndexSet
        | OpVs2::MapSet => (2, 1),
        OpVs2::Not
        | OpVs2::IsOk
        | OpVs2::Unwrap
        | OpVs2::CastI32
        | OpVs2::CastF64
        | OpVs2::Len
        | OpVs2::Ok
        | OpVs2::Err
        | OpVs2::Some
        | OpVs2::IndexGet
        | OpVs2::MapGet
        | OpVs2::Try => (1, 1),
        OpVs2::Call => (0, 1),
        OpVs2::Say
        | OpVs2::Menu
        | OpVs2::Choice
        | OpVs2::JumpScene
        | OpVs2::CallScene
        | OpVs2::ShowChar
        | OpVs2::HideChar
        | OpVs2::Background
        | OpVs2::Music
        | OpVs2::PushLayer
        | OpVs2::PopLayer
        | OpVs2::ShowLayer
        | OpVs2::HideLayer
        | OpVs2::SetLayerZ
        | OpVs2::Translate
        | OpVs2::MakeArray
        | OpVs2::MakeMap
        | OpVs2::TransformApply
        | OpVs2::TransitionPlay
        | OpVs2::ActionFire
        | OpVs2::ScreenOpen
        | OpVs2::ScreenClose
        | OpVs2::BindButton
        | OpVs2::PlaySfx
        | OpVs2::PlayVoice
        | OpVs2::StopBgm
        | OpVs2::SetVolume
        | OpVs2::Concat => (0, 0),
    }
}

/// Alias used by older codegen helpers.
pub fn op_name(op: OpVs2) -> &'static str {
    op.name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_count_no_reserved() {
        assert_eq!(OP_VS2_COUNT, 72);
        assert_eq!(OpVs2::all().len(), 72);
        assert!(OpVs2::from_u16(72).is_none());
        assert!(OpVs2::from_u16(399).is_none());
        for op in OpVs2::all() {
            assert!(!op.name().starts_with("Reserved"));
            assert_eq!(OpVs2::from_u16(op.as_u16()), Some(*op));
        }
    }

    #[test]
    fn core_names() {
        assert_eq!(OpVs2::Nop.name(), "Nop");
        assert_eq!(OpVs2::Say.name(), "Say");
        assert_eq!(OpVs2::PushLayer.name(), "PushLayer");
        assert_eq!(OpVs2::SetVolume.name(), "SetVolume");
    }

    #[test]
    fn stack_effect_add() {
        assert_eq!(stack_effect(OpVs2::Add), (2, 1));
        assert_eq!(stack_effect(OpVs2::LoadConst), (0, 1));
    }
}
