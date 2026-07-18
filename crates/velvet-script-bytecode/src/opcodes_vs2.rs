//! Velvet Script 2 extended opcode catalog and metadata.

#![allow(missing_docs)]

/// Extended opcode id for VS2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum OpVs2 {
    /// Nop
    Nop = 0,
    /// LoadConst
    LoadConst = 1,
    /// LoadLocal
    LoadLocal = 2,
    /// StoreLocal
    StoreLocal = 3,
    /// Add
    Add = 4,
    /// Sub
    Sub = 5,
    /// Mul
    Mul = 6,
    /// Div
    Div = 7,
    /// Rem
    Rem = 8,
    /// Eq
    Eq = 9,
    /// Ne
    Ne = 10,
    /// Lt
    Lt = 11,
    /// Le
    Le = 12,
    /// Gt
    Gt = 13,
    /// Ge
    Ge = 14,
    /// And
    And = 15,
    /// Or
    Or = 16,
    /// Not
    Not = 17,
    /// Jump
    Jump = 18,
    /// JumpIf
    JumpIf = 19,
    /// Call
    Call = 20,
    /// Ret
    Ret = 21,
    /// Print
    Print = 22,
    /// Pop
    Pop = 23,
    /// Dup
    Dup = 24,
    /// Say
    Say = 25,
    /// Menu
    Menu = 26,
    /// Choice
    Choice = 27,
    /// JumpScene
    JumpScene = 28,
    /// CallScene
    CallScene = 29,
    /// ShowChar
    ShowChar = 30,
    /// HideChar
    HideChar = 31,
    /// Background
    Background = 32,
    /// Music
    Music = 33,
    /// PushLayer
    PushLayer = 34,
    /// PopLayer
    PopLayer = 35,
    /// ShowLayer
    ShowLayer = 36,
    /// HideLayer
    HideLayer = 37,
    /// SetLayerZ
    SetLayerZ = 38,
    /// Translate
    Translate = 39,
    /// Await
    Await = 40,
    /// Yield
    Yield = 41,
    /// LoadMsg
    LoadMsg = 42,
    /// StoreState
    StoreState = 43,
    /// LoadState
    LoadState = 44,
    /// MakeArray
    MakeArray = 45,
    /// IndexGet
    IndexGet = 46,
    /// IndexSet
    IndexSet = 47,
    /// MakeMap
    MakeMap = 48,
    /// MapGet
    MapGet = 49,
    /// MapSet
    MapSet = 50,
    /// Ok
    Ok = 51,
    /// Err
    Err = 52,
    /// Some
    Some = 53,
    /// None_
    None_ = 54,
    /// Try
    Try = 55,
    /// IsOk
    IsOk = 56,
    /// Unwrap
    Unwrap = 57,
    /// CastI32
    CastI32 = 58,
    /// CastF64
    CastF64 = 59,
    /// Concat
    Concat = 60,
    /// Len
    Len = 61,
    /// TransformApply
    TransformApply = 62,
    /// TransitionPlay
    TransitionPlay = 63,
    /// ActionFire
    ActionFire = 64,
    /// ScreenOpen
    ScreenOpen = 65,
    /// ScreenClose
    ScreenClose = 66,
    /// BindButton
    BindButton = 67,
    /// PlaySfx
    PlaySfx = 68,
    /// PlayVoice
    PlayVoice = 69,
    /// StopBgm
    StopBgm = 70,
    /// SetVolume
    SetVolume = 71,
    /// Reserved slot 72
    Reserved72 = 72,
    /// Reserved slot 73
    Reserved73 = 73,
    /// Reserved slot 74
    Reserved74 = 74,
    /// Reserved slot 75
    Reserved75 = 75,
    /// Reserved slot 76
    Reserved76 = 76,
    /// Reserved slot 77
    Reserved77 = 77,
    /// Reserved slot 78
    Reserved78 = 78,
    /// Reserved slot 79
    Reserved79 = 79,
    /// Reserved slot 80
    Reserved80 = 80,
    /// Reserved slot 81
    Reserved81 = 81,
    /// Reserved slot 82
    Reserved82 = 82,
    /// Reserved slot 83
    Reserved83 = 83,
    /// Reserved slot 84
    Reserved84 = 84,
    /// Reserved slot 85
    Reserved85 = 85,
    /// Reserved slot 86
    Reserved86 = 86,
    /// Reserved slot 87
    Reserved87 = 87,
    /// Reserved slot 88
    Reserved88 = 88,
    /// Reserved slot 89
    Reserved89 = 89,
    /// Reserved slot 90
    Reserved90 = 90,
    /// Reserved slot 91
    Reserved91 = 91,
    /// Reserved slot 92
    Reserved92 = 92,
    /// Reserved slot 93
    Reserved93 = 93,
    /// Reserved slot 94
    Reserved94 = 94,
    /// Reserved slot 95
    Reserved95 = 95,
    /// Reserved slot 96
    Reserved96 = 96,
    /// Reserved slot 97
    Reserved97 = 97,
    /// Reserved slot 98
    Reserved98 = 98,
    /// Reserved slot 99
    Reserved99 = 99,
    /// Reserved slot 100
    Reserved100 = 100,
    /// Reserved slot 101
    Reserved101 = 101,
    /// Reserved slot 102
    Reserved102 = 102,
    /// Reserved slot 103
    Reserved103 = 103,
    /// Reserved slot 104
    Reserved104 = 104,
    /// Reserved slot 105
    Reserved105 = 105,
    /// Reserved slot 106
    Reserved106 = 106,
    /// Reserved slot 107
    Reserved107 = 107,
    /// Reserved slot 108
    Reserved108 = 108,
    /// Reserved slot 109
    Reserved109 = 109,
    /// Reserved slot 110
    Reserved110 = 110,
    /// Reserved slot 111
    Reserved111 = 111,
    /// Reserved slot 112
    Reserved112 = 112,
    /// Reserved slot 113
    Reserved113 = 113,
    /// Reserved slot 114
    Reserved114 = 114,
    /// Reserved slot 115
    Reserved115 = 115,
    /// Reserved slot 116
    Reserved116 = 116,
    /// Reserved slot 117
    Reserved117 = 117,
    /// Reserved slot 118
    Reserved118 = 118,
    /// Reserved slot 119
    Reserved119 = 119,
    /// Reserved slot 120
    Reserved120 = 120,
    /// Reserved slot 121
    Reserved121 = 121,
    /// Reserved slot 122
    Reserved122 = 122,
    /// Reserved slot 123
    Reserved123 = 123,
    /// Reserved slot 124
    Reserved124 = 124,
    /// Reserved slot 125
    Reserved125 = 125,
    /// Reserved slot 126
    Reserved126 = 126,
    /// Reserved slot 127
    Reserved127 = 127,
    /// Reserved slot 128
    Reserved128 = 128,
    /// Reserved slot 129
    Reserved129 = 129,
    /// Reserved slot 130
    Reserved130 = 130,
    /// Reserved slot 131
    Reserved131 = 131,
    /// Reserved slot 132
    Reserved132 = 132,
    /// Reserved slot 133
    Reserved133 = 133,
    /// Reserved slot 134
    Reserved134 = 134,
    /// Reserved slot 135
    Reserved135 = 135,
    /// Reserved slot 136
    Reserved136 = 136,
    /// Reserved slot 137
    Reserved137 = 137,
    /// Reserved slot 138
    Reserved138 = 138,
    /// Reserved slot 139
    Reserved139 = 139,
    /// Reserved slot 140
    Reserved140 = 140,
    /// Reserved slot 141
    Reserved141 = 141,
    /// Reserved slot 142
    Reserved142 = 142,
    /// Reserved slot 143
    Reserved143 = 143,
    /// Reserved slot 144
    Reserved144 = 144,
    /// Reserved slot 145
    Reserved145 = 145,
    /// Reserved slot 146
    Reserved146 = 146,
    /// Reserved slot 147
    Reserved147 = 147,
    /// Reserved slot 148
    Reserved148 = 148,
    /// Reserved slot 149
    Reserved149 = 149,
    /// Reserved slot 150
    Reserved150 = 150,
    /// Reserved slot 151
    Reserved151 = 151,
    /// Reserved slot 152
    Reserved152 = 152,
    /// Reserved slot 153
    Reserved153 = 153,
    /// Reserved slot 154
    Reserved154 = 154,
    /// Reserved slot 155
    Reserved155 = 155,
    /// Reserved slot 156
    Reserved156 = 156,
    /// Reserved slot 157
    Reserved157 = 157,
    /// Reserved slot 158
    Reserved158 = 158,
    /// Reserved slot 159
    Reserved159 = 159,
    /// Reserved slot 160
    Reserved160 = 160,
    /// Reserved slot 161
    Reserved161 = 161,
    /// Reserved slot 162
    Reserved162 = 162,
    /// Reserved slot 163
    Reserved163 = 163,
    /// Reserved slot 164
    Reserved164 = 164,
    /// Reserved slot 165
    Reserved165 = 165,
    /// Reserved slot 166
    Reserved166 = 166,
    /// Reserved slot 167
    Reserved167 = 167,
    /// Reserved slot 168
    Reserved168 = 168,
    /// Reserved slot 169
    Reserved169 = 169,
    /// Reserved slot 170
    Reserved170 = 170,
    /// Reserved slot 171
    Reserved171 = 171,
    /// Reserved slot 172
    Reserved172 = 172,
    /// Reserved slot 173
    Reserved173 = 173,
    /// Reserved slot 174
    Reserved174 = 174,
    /// Reserved slot 175
    Reserved175 = 175,
    /// Reserved slot 176
    Reserved176 = 176,
    /// Reserved slot 177
    Reserved177 = 177,
    /// Reserved slot 178
    Reserved178 = 178,
    /// Reserved slot 179
    Reserved179 = 179,
    /// Reserved slot 180
    Reserved180 = 180,
    /// Reserved slot 181
    Reserved181 = 181,
    /// Reserved slot 182
    Reserved182 = 182,
    /// Reserved slot 183
    Reserved183 = 183,
    /// Reserved slot 184
    Reserved184 = 184,
    /// Reserved slot 185
    Reserved185 = 185,
    /// Reserved slot 186
    Reserved186 = 186,
    /// Reserved slot 187
    Reserved187 = 187,
    /// Reserved slot 188
    Reserved188 = 188,
    /// Reserved slot 189
    Reserved189 = 189,
    /// Reserved slot 190
    Reserved190 = 190,
    /// Reserved slot 191
    Reserved191 = 191,
    /// Reserved slot 192
    Reserved192 = 192,
    /// Reserved slot 193
    Reserved193 = 193,
    /// Reserved slot 194
    Reserved194 = 194,
    /// Reserved slot 195
    Reserved195 = 195,
    /// Reserved slot 196
    Reserved196 = 196,
    /// Reserved slot 197
    Reserved197 = 197,
    /// Reserved slot 198
    Reserved198 = 198,
    /// Reserved slot 199
    Reserved199 = 199,
    /// Reserved slot 200
    Reserved200 = 200,
    /// Reserved slot 201
    Reserved201 = 201,
    /// Reserved slot 202
    Reserved202 = 202,
    /// Reserved slot 203
    Reserved203 = 203,
    /// Reserved slot 204
    Reserved204 = 204,
    /// Reserved slot 205
    Reserved205 = 205,
    /// Reserved slot 206
    Reserved206 = 206,
    /// Reserved slot 207
    Reserved207 = 207,
    /// Reserved slot 208
    Reserved208 = 208,
    /// Reserved slot 209
    Reserved209 = 209,
    /// Reserved slot 210
    Reserved210 = 210,
    /// Reserved slot 211
    Reserved211 = 211,
    /// Reserved slot 212
    Reserved212 = 212,
    /// Reserved slot 213
    Reserved213 = 213,
    /// Reserved slot 214
    Reserved214 = 214,
    /// Reserved slot 215
    Reserved215 = 215,
    /// Reserved slot 216
    Reserved216 = 216,
    /// Reserved slot 217
    Reserved217 = 217,
    /// Reserved slot 218
    Reserved218 = 218,
    /// Reserved slot 219
    Reserved219 = 219,
    /// Reserved slot 220
    Reserved220 = 220,
    /// Reserved slot 221
    Reserved221 = 221,
    /// Reserved slot 222
    Reserved222 = 222,
    /// Reserved slot 223
    Reserved223 = 223,
    /// Reserved slot 224
    Reserved224 = 224,
    /// Reserved slot 225
    Reserved225 = 225,
    /// Reserved slot 226
    Reserved226 = 226,
    /// Reserved slot 227
    Reserved227 = 227,
    /// Reserved slot 228
    Reserved228 = 228,
    /// Reserved slot 229
    Reserved229 = 229,
    /// Reserved slot 230
    Reserved230 = 230,
    /// Reserved slot 231
    Reserved231 = 231,
    /// Reserved slot 232
    Reserved232 = 232,
    /// Reserved slot 233
    Reserved233 = 233,
    /// Reserved slot 234
    Reserved234 = 234,
    /// Reserved slot 235
    Reserved235 = 235,
    /// Reserved slot 236
    Reserved236 = 236,
    /// Reserved slot 237
    Reserved237 = 237,
    /// Reserved slot 238
    Reserved238 = 238,
    /// Reserved slot 239
    Reserved239 = 239,
    /// Reserved slot 240
    Reserved240 = 240,
    /// Reserved slot 241
    Reserved241 = 241,
    /// Reserved slot 242
    Reserved242 = 242,
    /// Reserved slot 243
    Reserved243 = 243,
    /// Reserved slot 244
    Reserved244 = 244,
    /// Reserved slot 245
    Reserved245 = 245,
    /// Reserved slot 246
    Reserved246 = 246,
    /// Reserved slot 247
    Reserved247 = 247,
    /// Reserved slot 248
    Reserved248 = 248,
    /// Reserved slot 249
    Reserved249 = 249,
    /// Reserved slot 250
    Reserved250 = 250,
    /// Reserved slot 251
    Reserved251 = 251,
    /// Reserved slot 252
    Reserved252 = 252,
    /// Reserved slot 253
    Reserved253 = 253,
    /// Reserved slot 254
    Reserved254 = 254,
    /// Reserved slot 255
    Reserved255 = 255,
    /// Reserved slot 256
    Reserved256 = 256,
    /// Reserved slot 257
    Reserved257 = 257,
    /// Reserved slot 258
    Reserved258 = 258,
    /// Reserved slot 259
    Reserved259 = 259,
    /// Reserved slot 260
    Reserved260 = 260,
    /// Reserved slot 261
    Reserved261 = 261,
    /// Reserved slot 262
    Reserved262 = 262,
    /// Reserved slot 263
    Reserved263 = 263,
    /// Reserved slot 264
    Reserved264 = 264,
    /// Reserved slot 265
    Reserved265 = 265,
    /// Reserved slot 266
    Reserved266 = 266,
    /// Reserved slot 267
    Reserved267 = 267,
    /// Reserved slot 268
    Reserved268 = 268,
    /// Reserved slot 269
    Reserved269 = 269,
    /// Reserved slot 270
    Reserved270 = 270,
    /// Reserved slot 271
    Reserved271 = 271,
    /// Reserved slot 272
    Reserved272 = 272,
    /// Reserved slot 273
    Reserved273 = 273,
    /// Reserved slot 274
    Reserved274 = 274,
    /// Reserved slot 275
    Reserved275 = 275,
    /// Reserved slot 276
    Reserved276 = 276,
    /// Reserved slot 277
    Reserved277 = 277,
    /// Reserved slot 278
    Reserved278 = 278,
    /// Reserved slot 279
    Reserved279 = 279,
    /// Reserved slot 280
    Reserved280 = 280,
    /// Reserved slot 281
    Reserved281 = 281,
    /// Reserved slot 282
    Reserved282 = 282,
    /// Reserved slot 283
    Reserved283 = 283,
    /// Reserved slot 284
    Reserved284 = 284,
    /// Reserved slot 285
    Reserved285 = 285,
    /// Reserved slot 286
    Reserved286 = 286,
    /// Reserved slot 287
    Reserved287 = 287,
    /// Reserved slot 288
    Reserved288 = 288,
    /// Reserved slot 289
    Reserved289 = 289,
    /// Reserved slot 290
    Reserved290 = 290,
    /// Reserved slot 291
    Reserved291 = 291,
    /// Reserved slot 292
    Reserved292 = 292,
    /// Reserved slot 293
    Reserved293 = 293,
    /// Reserved slot 294
    Reserved294 = 294,
    /// Reserved slot 295
    Reserved295 = 295,
    /// Reserved slot 296
    Reserved296 = 296,
    /// Reserved slot 297
    Reserved297 = 297,
    /// Reserved slot 298
    Reserved298 = 298,
    /// Reserved slot 299
    Reserved299 = 299,
    /// Reserved slot 300
    Reserved300 = 300,
    /// Reserved slot 301
    Reserved301 = 301,
    /// Reserved slot 302
    Reserved302 = 302,
    /// Reserved slot 303
    Reserved303 = 303,
    /// Reserved slot 304
    Reserved304 = 304,
    /// Reserved slot 305
    Reserved305 = 305,
    /// Reserved slot 306
    Reserved306 = 306,
    /// Reserved slot 307
    Reserved307 = 307,
    /// Reserved slot 308
    Reserved308 = 308,
    /// Reserved slot 309
    Reserved309 = 309,
    /// Reserved slot 310
    Reserved310 = 310,
    /// Reserved slot 311
    Reserved311 = 311,
    /// Reserved slot 312
    Reserved312 = 312,
    /// Reserved slot 313
    Reserved313 = 313,
    /// Reserved slot 314
    Reserved314 = 314,
    /// Reserved slot 315
    Reserved315 = 315,
    /// Reserved slot 316
    Reserved316 = 316,
    /// Reserved slot 317
    Reserved317 = 317,
    /// Reserved slot 318
    Reserved318 = 318,
    /// Reserved slot 319
    Reserved319 = 319,
    /// Reserved slot 320
    Reserved320 = 320,
    /// Reserved slot 321
    Reserved321 = 321,
    /// Reserved slot 322
    Reserved322 = 322,
    /// Reserved slot 323
    Reserved323 = 323,
    /// Reserved slot 324
    Reserved324 = 324,
    /// Reserved slot 325
    Reserved325 = 325,
    /// Reserved slot 326
    Reserved326 = 326,
    /// Reserved slot 327
    Reserved327 = 327,
    /// Reserved slot 328
    Reserved328 = 328,
    /// Reserved slot 329
    Reserved329 = 329,
    /// Reserved slot 330
    Reserved330 = 330,
    /// Reserved slot 331
    Reserved331 = 331,
    /// Reserved slot 332
    Reserved332 = 332,
    /// Reserved slot 333
    Reserved333 = 333,
    /// Reserved slot 334
    Reserved334 = 334,
    /// Reserved slot 335
    Reserved335 = 335,
    /// Reserved slot 336
    Reserved336 = 336,
    /// Reserved slot 337
    Reserved337 = 337,
    /// Reserved slot 338
    Reserved338 = 338,
    /// Reserved slot 339
    Reserved339 = 339,
    /// Reserved slot 340
    Reserved340 = 340,
    /// Reserved slot 341
    Reserved341 = 341,
    /// Reserved slot 342
    Reserved342 = 342,
    /// Reserved slot 343
    Reserved343 = 343,
    /// Reserved slot 344
    Reserved344 = 344,
    /// Reserved slot 345
    Reserved345 = 345,
    /// Reserved slot 346
    Reserved346 = 346,
    /// Reserved slot 347
    Reserved347 = 347,
    /// Reserved slot 348
    Reserved348 = 348,
    /// Reserved slot 349
    Reserved349 = 349,
    /// Reserved slot 350
    Reserved350 = 350,
    /// Reserved slot 351
    Reserved351 = 351,
    /// Reserved slot 352
    Reserved352 = 352,
    /// Reserved slot 353
    Reserved353 = 353,
    /// Reserved slot 354
    Reserved354 = 354,
    /// Reserved slot 355
    Reserved355 = 355,
    /// Reserved slot 356
    Reserved356 = 356,
    /// Reserved slot 357
    Reserved357 = 357,
    /// Reserved slot 358
    Reserved358 = 358,
    /// Reserved slot 359
    Reserved359 = 359,
    /// Reserved slot 360
    Reserved360 = 360,
    /// Reserved slot 361
    Reserved361 = 361,
    /// Reserved slot 362
    Reserved362 = 362,
    /// Reserved slot 363
    Reserved363 = 363,
    /// Reserved slot 364
    Reserved364 = 364,
    /// Reserved slot 365
    Reserved365 = 365,
    /// Reserved slot 366
    Reserved366 = 366,
    /// Reserved slot 367
    Reserved367 = 367,
    /// Reserved slot 368
    Reserved368 = 368,
    /// Reserved slot 369
    Reserved369 = 369,
    /// Reserved slot 370
    Reserved370 = 370,
    /// Reserved slot 371
    Reserved371 = 371,
    /// Reserved slot 372
    Reserved372 = 372,
    /// Reserved slot 373
    Reserved373 = 373,
    /// Reserved slot 374
    Reserved374 = 374,
    /// Reserved slot 375
    Reserved375 = 375,
    /// Reserved slot 376
    Reserved376 = 376,
    /// Reserved slot 377
    Reserved377 = 377,
    /// Reserved slot 378
    Reserved378 = 378,
    /// Reserved slot 379
    Reserved379 = 379,
    /// Reserved slot 380
    Reserved380 = 380,
    /// Reserved slot 381
    Reserved381 = 381,
    /// Reserved slot 382
    Reserved382 = 382,
    /// Reserved slot 383
    Reserved383 = 383,
    /// Reserved slot 384
    Reserved384 = 384,
    /// Reserved slot 385
    Reserved385 = 385,
    /// Reserved slot 386
    Reserved386 = 386,
    /// Reserved slot 387
    Reserved387 = 387,
    /// Reserved slot 388
    Reserved388 = 388,
    /// Reserved slot 389
    Reserved389 = 389,
    /// Reserved slot 390
    Reserved390 = 390,
    /// Reserved slot 391
    Reserved391 = 391,
    /// Reserved slot 392
    Reserved392 = 392,
    /// Reserved slot 393
    Reserved393 = 393,
    /// Reserved slot 394
    Reserved394 = 394,
    /// Reserved slot 395
    Reserved395 = 395,
    /// Reserved slot 396
    Reserved396 = 396,
    /// Reserved slot 397
    Reserved397 = 397,
    /// Reserved slot 398
    Reserved398 = 398,
    /// Reserved slot 399
    Reserved399 = 399,
}

impl OpVs2 {
    pub fn as_u16(self) -> u16 { self as u16 }
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
            Self::Reserved72 => "Reserved72",
            Self::Reserved73 => "Reserved73",
            Self::Reserved74 => "Reserved74",
            Self::Reserved75 => "Reserved75",
            Self::Reserved76 => "Reserved76",
            Self::Reserved77 => "Reserved77",
            Self::Reserved78 => "Reserved78",
            Self::Reserved79 => "Reserved79",
            Self::Reserved80 => "Reserved80",
            Self::Reserved81 => "Reserved81",
            Self::Reserved82 => "Reserved82",
            Self::Reserved83 => "Reserved83",
            Self::Reserved84 => "Reserved84",
            Self::Reserved85 => "Reserved85",
            Self::Reserved86 => "Reserved86",
            Self::Reserved87 => "Reserved87",
            Self::Reserved88 => "Reserved88",
            Self::Reserved89 => "Reserved89",
            Self::Reserved90 => "Reserved90",
            Self::Reserved91 => "Reserved91",
            Self::Reserved92 => "Reserved92",
            Self::Reserved93 => "Reserved93",
            Self::Reserved94 => "Reserved94",
            Self::Reserved95 => "Reserved95",
            Self::Reserved96 => "Reserved96",
            Self::Reserved97 => "Reserved97",
            Self::Reserved98 => "Reserved98",
            Self::Reserved99 => "Reserved99",
            Self::Reserved100 => "Reserved100",
            Self::Reserved101 => "Reserved101",
            Self::Reserved102 => "Reserved102",
            Self::Reserved103 => "Reserved103",
            Self::Reserved104 => "Reserved104",
            Self::Reserved105 => "Reserved105",
            Self::Reserved106 => "Reserved106",
            Self::Reserved107 => "Reserved107",
            Self::Reserved108 => "Reserved108",
            Self::Reserved109 => "Reserved109",
            Self::Reserved110 => "Reserved110",
            Self::Reserved111 => "Reserved111",
            Self::Reserved112 => "Reserved112",
            Self::Reserved113 => "Reserved113",
            Self::Reserved114 => "Reserved114",
            Self::Reserved115 => "Reserved115",
            Self::Reserved116 => "Reserved116",
            Self::Reserved117 => "Reserved117",
            Self::Reserved118 => "Reserved118",
            Self::Reserved119 => "Reserved119",
            Self::Reserved120 => "Reserved120",
            Self::Reserved121 => "Reserved121",
            Self::Reserved122 => "Reserved122",
            Self::Reserved123 => "Reserved123",
            Self::Reserved124 => "Reserved124",
            Self::Reserved125 => "Reserved125",
            Self::Reserved126 => "Reserved126",
            Self::Reserved127 => "Reserved127",
            Self::Reserved128 => "Reserved128",
            Self::Reserved129 => "Reserved129",
            Self::Reserved130 => "Reserved130",
            Self::Reserved131 => "Reserved131",
            Self::Reserved132 => "Reserved132",
            Self::Reserved133 => "Reserved133",
            Self::Reserved134 => "Reserved134",
            Self::Reserved135 => "Reserved135",
            Self::Reserved136 => "Reserved136",
            Self::Reserved137 => "Reserved137",
            Self::Reserved138 => "Reserved138",
            Self::Reserved139 => "Reserved139",
            Self::Reserved140 => "Reserved140",
            Self::Reserved141 => "Reserved141",
            Self::Reserved142 => "Reserved142",
            Self::Reserved143 => "Reserved143",
            Self::Reserved144 => "Reserved144",
            Self::Reserved145 => "Reserved145",
            Self::Reserved146 => "Reserved146",
            Self::Reserved147 => "Reserved147",
            Self::Reserved148 => "Reserved148",
            Self::Reserved149 => "Reserved149",
            Self::Reserved150 => "Reserved150",
            Self::Reserved151 => "Reserved151",
            Self::Reserved152 => "Reserved152",
            Self::Reserved153 => "Reserved153",
            Self::Reserved154 => "Reserved154",
            Self::Reserved155 => "Reserved155",
            Self::Reserved156 => "Reserved156",
            Self::Reserved157 => "Reserved157",
            Self::Reserved158 => "Reserved158",
            Self::Reserved159 => "Reserved159",
            Self::Reserved160 => "Reserved160",
            Self::Reserved161 => "Reserved161",
            Self::Reserved162 => "Reserved162",
            Self::Reserved163 => "Reserved163",
            Self::Reserved164 => "Reserved164",
            Self::Reserved165 => "Reserved165",
            Self::Reserved166 => "Reserved166",
            Self::Reserved167 => "Reserved167",
            Self::Reserved168 => "Reserved168",
            Self::Reserved169 => "Reserved169",
            Self::Reserved170 => "Reserved170",
            Self::Reserved171 => "Reserved171",
            Self::Reserved172 => "Reserved172",
            Self::Reserved173 => "Reserved173",
            Self::Reserved174 => "Reserved174",
            Self::Reserved175 => "Reserved175",
            Self::Reserved176 => "Reserved176",
            Self::Reserved177 => "Reserved177",
            Self::Reserved178 => "Reserved178",
            Self::Reserved179 => "Reserved179",
            Self::Reserved180 => "Reserved180",
            Self::Reserved181 => "Reserved181",
            Self::Reserved182 => "Reserved182",
            Self::Reserved183 => "Reserved183",
            Self::Reserved184 => "Reserved184",
            Self::Reserved185 => "Reserved185",
            Self::Reserved186 => "Reserved186",
            Self::Reserved187 => "Reserved187",
            Self::Reserved188 => "Reserved188",
            Self::Reserved189 => "Reserved189",
            Self::Reserved190 => "Reserved190",
            Self::Reserved191 => "Reserved191",
            Self::Reserved192 => "Reserved192",
            Self::Reserved193 => "Reserved193",
            Self::Reserved194 => "Reserved194",
            Self::Reserved195 => "Reserved195",
            Self::Reserved196 => "Reserved196",
            Self::Reserved197 => "Reserved197",
            Self::Reserved198 => "Reserved198",
            Self::Reserved199 => "Reserved199",
            Self::Reserved200 => "Reserved200",
            Self::Reserved201 => "Reserved201",
            Self::Reserved202 => "Reserved202",
            Self::Reserved203 => "Reserved203",
            Self::Reserved204 => "Reserved204",
            Self::Reserved205 => "Reserved205",
            Self::Reserved206 => "Reserved206",
            Self::Reserved207 => "Reserved207",
            Self::Reserved208 => "Reserved208",
            Self::Reserved209 => "Reserved209",
            Self::Reserved210 => "Reserved210",
            Self::Reserved211 => "Reserved211",
            Self::Reserved212 => "Reserved212",
            Self::Reserved213 => "Reserved213",
            Self::Reserved214 => "Reserved214",
            Self::Reserved215 => "Reserved215",
            Self::Reserved216 => "Reserved216",
            Self::Reserved217 => "Reserved217",
            Self::Reserved218 => "Reserved218",
            Self::Reserved219 => "Reserved219",
            Self::Reserved220 => "Reserved220",
            Self::Reserved221 => "Reserved221",
            Self::Reserved222 => "Reserved222",
            Self::Reserved223 => "Reserved223",
            Self::Reserved224 => "Reserved224",
            Self::Reserved225 => "Reserved225",
            Self::Reserved226 => "Reserved226",
            Self::Reserved227 => "Reserved227",
            Self::Reserved228 => "Reserved228",
            Self::Reserved229 => "Reserved229",
            Self::Reserved230 => "Reserved230",
            Self::Reserved231 => "Reserved231",
            Self::Reserved232 => "Reserved232",
            Self::Reserved233 => "Reserved233",
            Self::Reserved234 => "Reserved234",
            Self::Reserved235 => "Reserved235",
            Self::Reserved236 => "Reserved236",
            Self::Reserved237 => "Reserved237",
            Self::Reserved238 => "Reserved238",
            Self::Reserved239 => "Reserved239",
            Self::Reserved240 => "Reserved240",
            Self::Reserved241 => "Reserved241",
            Self::Reserved242 => "Reserved242",
            Self::Reserved243 => "Reserved243",
            Self::Reserved244 => "Reserved244",
            Self::Reserved245 => "Reserved245",
            Self::Reserved246 => "Reserved246",
            Self::Reserved247 => "Reserved247",
            Self::Reserved248 => "Reserved248",
            Self::Reserved249 => "Reserved249",
            Self::Reserved250 => "Reserved250",
            Self::Reserved251 => "Reserved251",
            Self::Reserved252 => "Reserved252",
            Self::Reserved253 => "Reserved253",
            Self::Reserved254 => "Reserved254",
            Self::Reserved255 => "Reserved255",
            Self::Reserved256 => "Reserved256",
            Self::Reserved257 => "Reserved257",
            Self::Reserved258 => "Reserved258",
            Self::Reserved259 => "Reserved259",
            Self::Reserved260 => "Reserved260",
            Self::Reserved261 => "Reserved261",
            Self::Reserved262 => "Reserved262",
            Self::Reserved263 => "Reserved263",
            Self::Reserved264 => "Reserved264",
            Self::Reserved265 => "Reserved265",
            Self::Reserved266 => "Reserved266",
            Self::Reserved267 => "Reserved267",
            Self::Reserved268 => "Reserved268",
            Self::Reserved269 => "Reserved269",
            Self::Reserved270 => "Reserved270",
            Self::Reserved271 => "Reserved271",
            Self::Reserved272 => "Reserved272",
            Self::Reserved273 => "Reserved273",
            Self::Reserved274 => "Reserved274",
            Self::Reserved275 => "Reserved275",
            Self::Reserved276 => "Reserved276",
            Self::Reserved277 => "Reserved277",
            Self::Reserved278 => "Reserved278",
            Self::Reserved279 => "Reserved279",
            Self::Reserved280 => "Reserved280",
            Self::Reserved281 => "Reserved281",
            Self::Reserved282 => "Reserved282",
            Self::Reserved283 => "Reserved283",
            Self::Reserved284 => "Reserved284",
            Self::Reserved285 => "Reserved285",
            Self::Reserved286 => "Reserved286",
            Self::Reserved287 => "Reserved287",
            Self::Reserved288 => "Reserved288",
            Self::Reserved289 => "Reserved289",
            Self::Reserved290 => "Reserved290",
            Self::Reserved291 => "Reserved291",
            Self::Reserved292 => "Reserved292",
            Self::Reserved293 => "Reserved293",
            Self::Reserved294 => "Reserved294",
            Self::Reserved295 => "Reserved295",
            Self::Reserved296 => "Reserved296",
            Self::Reserved297 => "Reserved297",
            Self::Reserved298 => "Reserved298",
            Self::Reserved299 => "Reserved299",
            Self::Reserved300 => "Reserved300",
            Self::Reserved301 => "Reserved301",
            Self::Reserved302 => "Reserved302",
            Self::Reserved303 => "Reserved303",
            Self::Reserved304 => "Reserved304",
            Self::Reserved305 => "Reserved305",
            Self::Reserved306 => "Reserved306",
            Self::Reserved307 => "Reserved307",
            Self::Reserved308 => "Reserved308",
            Self::Reserved309 => "Reserved309",
            Self::Reserved310 => "Reserved310",
            Self::Reserved311 => "Reserved311",
            Self::Reserved312 => "Reserved312",
            Self::Reserved313 => "Reserved313",
            Self::Reserved314 => "Reserved314",
            Self::Reserved315 => "Reserved315",
            Self::Reserved316 => "Reserved316",
            Self::Reserved317 => "Reserved317",
            Self::Reserved318 => "Reserved318",
            Self::Reserved319 => "Reserved319",
            Self::Reserved320 => "Reserved320",
            Self::Reserved321 => "Reserved321",
            Self::Reserved322 => "Reserved322",
            Self::Reserved323 => "Reserved323",
            Self::Reserved324 => "Reserved324",
            Self::Reserved325 => "Reserved325",
            Self::Reserved326 => "Reserved326",
            Self::Reserved327 => "Reserved327",
            Self::Reserved328 => "Reserved328",
            Self::Reserved329 => "Reserved329",
            Self::Reserved330 => "Reserved330",
            Self::Reserved331 => "Reserved331",
            Self::Reserved332 => "Reserved332",
            Self::Reserved333 => "Reserved333",
            Self::Reserved334 => "Reserved334",
            Self::Reserved335 => "Reserved335",
            Self::Reserved336 => "Reserved336",
            Self::Reserved337 => "Reserved337",
            Self::Reserved338 => "Reserved338",
            Self::Reserved339 => "Reserved339",
            Self::Reserved340 => "Reserved340",
            Self::Reserved341 => "Reserved341",
            Self::Reserved342 => "Reserved342",
            Self::Reserved343 => "Reserved343",
            Self::Reserved344 => "Reserved344",
            Self::Reserved345 => "Reserved345",
            Self::Reserved346 => "Reserved346",
            Self::Reserved347 => "Reserved347",
            Self::Reserved348 => "Reserved348",
            Self::Reserved349 => "Reserved349",
            Self::Reserved350 => "Reserved350",
            Self::Reserved351 => "Reserved351",
            Self::Reserved352 => "Reserved352",
            Self::Reserved353 => "Reserved353",
            Self::Reserved354 => "Reserved354",
            Self::Reserved355 => "Reserved355",
            Self::Reserved356 => "Reserved356",
            Self::Reserved357 => "Reserved357",
            Self::Reserved358 => "Reserved358",
            Self::Reserved359 => "Reserved359",
            Self::Reserved360 => "Reserved360",
            Self::Reserved361 => "Reserved361",
            Self::Reserved362 => "Reserved362",
            Self::Reserved363 => "Reserved363",
            Self::Reserved364 => "Reserved364",
            Self::Reserved365 => "Reserved365",
            Self::Reserved366 => "Reserved366",
            Self::Reserved367 => "Reserved367",
            Self::Reserved368 => "Reserved368",
            Self::Reserved369 => "Reserved369",
            Self::Reserved370 => "Reserved370",
            Self::Reserved371 => "Reserved371",
            Self::Reserved372 => "Reserved372",
            Self::Reserved373 => "Reserved373",
            Self::Reserved374 => "Reserved374",
            Self::Reserved375 => "Reserved375",
            Self::Reserved376 => "Reserved376",
            Self::Reserved377 => "Reserved377",
            Self::Reserved378 => "Reserved378",
            Self::Reserved379 => "Reserved379",
            Self::Reserved380 => "Reserved380",
            Self::Reserved381 => "Reserved381",
            Self::Reserved382 => "Reserved382",
            Self::Reserved383 => "Reserved383",
            Self::Reserved384 => "Reserved384",
            Self::Reserved385 => "Reserved385",
            Self::Reserved386 => "Reserved386",
            Self::Reserved387 => "Reserved387",
            Self::Reserved388 => "Reserved388",
            Self::Reserved389 => "Reserved389",
            Self::Reserved390 => "Reserved390",
            Self::Reserved391 => "Reserved391",
            Self::Reserved392 => "Reserved392",
            Self::Reserved393 => "Reserved393",
            Self::Reserved394 => "Reserved394",
            Self::Reserved395 => "Reserved395",
            Self::Reserved396 => "Reserved396",
            Self::Reserved397 => "Reserved397",
            Self::Reserved398 => "Reserved398",
            Self::Reserved399 => "Reserved399",
        }
    }
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0 => Some(Self::Nop),
            1 => Some(Self::LoadConst),
            2 => Some(Self::LoadLocal),
            3 => Some(Self::StoreLocal),
            4 => Some(Self::Add),
            5 => Some(Self::Sub),
            6 => Some(Self::Mul),
            7 => Some(Self::Div),
            8 => Some(Self::Rem),
            9 => Some(Self::Eq),
            10 => Some(Self::Ne),
            11 => Some(Self::Lt),
            12 => Some(Self::Le),
            13 => Some(Self::Gt),
            14 => Some(Self::Ge),
            15 => Some(Self::And),
            16 => Some(Self::Or),
            17 => Some(Self::Not),
            18 => Some(Self::Jump),
            19 => Some(Self::JumpIf),
            20 => Some(Self::Call),
            21 => Some(Self::Ret),
            22 => Some(Self::Print),
            23 => Some(Self::Pop),
            24 => Some(Self::Dup),
            25 => Some(Self::Say),
            26 => Some(Self::Menu),
            27 => Some(Self::Choice),
            28 => Some(Self::JumpScene),
            29 => Some(Self::CallScene),
            30 => Some(Self::ShowChar),
            31 => Some(Self::HideChar),
            32 => Some(Self::Background),
            33 => Some(Self::Music),
            34 => Some(Self::PushLayer),
            35 => Some(Self::PopLayer),
            36 => Some(Self::ShowLayer),
            37 => Some(Self::HideLayer),
            38 => Some(Self::SetLayerZ),
            39 => Some(Self::Translate),
            40 => Some(Self::Await),
            41 => Some(Self::Yield),
            42 => Some(Self::LoadMsg),
            43 => Some(Self::StoreState),
            44 => Some(Self::LoadState),
            45 => Some(Self::MakeArray),
            46 => Some(Self::IndexGet),
            47 => Some(Self::IndexSet),
            48 => Some(Self::MakeMap),
            49 => Some(Self::MapGet),
            50 => Some(Self::MapSet),
            51 => Some(Self::Ok),
            52 => Some(Self::Err),
            53 => Some(Self::Some),
            54 => Some(Self::None_),
            55 => Some(Self::Try),
            56 => Some(Self::IsOk),
            57 => Some(Self::Unwrap),
            58 => Some(Self::CastI32),
            59 => Some(Self::CastF64),
            60 => Some(Self::Concat),
            61 => Some(Self::Len),
            62 => Some(Self::TransformApply),
            63 => Some(Self::TransitionPlay),
            64 => Some(Self::ActionFire),
            65 => Some(Self::ScreenOpen),
            66 => Some(Self::ScreenClose),
            67 => Some(Self::BindButton),
            68 => Some(Self::PlaySfx),
            69 => Some(Self::PlayVoice),
            70 => Some(Self::StopBgm),
            71 => Some(Self::SetVolume),
            72 => Some(Self::Reserved72),
            73 => Some(Self::Reserved73),
            74 => Some(Self::Reserved74),
            75 => Some(Self::Reserved75),
            76 => Some(Self::Reserved76),
            77 => Some(Self::Reserved77),
            78 => Some(Self::Reserved78),
            79 => Some(Self::Reserved79),
            80 => Some(Self::Reserved80),
            81 => Some(Self::Reserved81),
            82 => Some(Self::Reserved82),
            83 => Some(Self::Reserved83),
            84 => Some(Self::Reserved84),
            85 => Some(Self::Reserved85),
            86 => Some(Self::Reserved86),
            87 => Some(Self::Reserved87),
            88 => Some(Self::Reserved88),
            89 => Some(Self::Reserved89),
            90 => Some(Self::Reserved90),
            91 => Some(Self::Reserved91),
            92 => Some(Self::Reserved92),
            93 => Some(Self::Reserved93),
            94 => Some(Self::Reserved94),
            95 => Some(Self::Reserved95),
            96 => Some(Self::Reserved96),
            97 => Some(Self::Reserved97),
            98 => Some(Self::Reserved98),
            99 => Some(Self::Reserved99),
            100 => Some(Self::Reserved100),
            101 => Some(Self::Reserved101),
            102 => Some(Self::Reserved102),
            103 => Some(Self::Reserved103),
            104 => Some(Self::Reserved104),
            105 => Some(Self::Reserved105),
            106 => Some(Self::Reserved106),
            107 => Some(Self::Reserved107),
            108 => Some(Self::Reserved108),
            109 => Some(Self::Reserved109),
            110 => Some(Self::Reserved110),
            111 => Some(Self::Reserved111),
            112 => Some(Self::Reserved112),
            113 => Some(Self::Reserved113),
            114 => Some(Self::Reserved114),
            115 => Some(Self::Reserved115),
            116 => Some(Self::Reserved116),
            117 => Some(Self::Reserved117),
            118 => Some(Self::Reserved118),
            119 => Some(Self::Reserved119),
            120 => Some(Self::Reserved120),
            121 => Some(Self::Reserved121),
            122 => Some(Self::Reserved122),
            123 => Some(Self::Reserved123),
            124 => Some(Self::Reserved124),
            125 => Some(Self::Reserved125),
            126 => Some(Self::Reserved126),
            127 => Some(Self::Reserved127),
            128 => Some(Self::Reserved128),
            129 => Some(Self::Reserved129),
            130 => Some(Self::Reserved130),
            131 => Some(Self::Reserved131),
            132 => Some(Self::Reserved132),
            133 => Some(Self::Reserved133),
            134 => Some(Self::Reserved134),
            135 => Some(Self::Reserved135),
            136 => Some(Self::Reserved136),
            137 => Some(Self::Reserved137),
            138 => Some(Self::Reserved138),
            139 => Some(Self::Reserved139),
            140 => Some(Self::Reserved140),
            141 => Some(Self::Reserved141),
            142 => Some(Self::Reserved142),
            143 => Some(Self::Reserved143),
            144 => Some(Self::Reserved144),
            145 => Some(Self::Reserved145),
            146 => Some(Self::Reserved146),
            147 => Some(Self::Reserved147),
            148 => Some(Self::Reserved148),
            149 => Some(Self::Reserved149),
            150 => Some(Self::Reserved150),
            151 => Some(Self::Reserved151),
            152 => Some(Self::Reserved152),
            153 => Some(Self::Reserved153),
            154 => Some(Self::Reserved154),
            155 => Some(Self::Reserved155),
            156 => Some(Self::Reserved156),
            157 => Some(Self::Reserved157),
            158 => Some(Self::Reserved158),
            159 => Some(Self::Reserved159),
            160 => Some(Self::Reserved160),
            161 => Some(Self::Reserved161),
            162 => Some(Self::Reserved162),
            163 => Some(Self::Reserved163),
            164 => Some(Self::Reserved164),
            165 => Some(Self::Reserved165),
            166 => Some(Self::Reserved166),
            167 => Some(Self::Reserved167),
            168 => Some(Self::Reserved168),
            169 => Some(Self::Reserved169),
            170 => Some(Self::Reserved170),
            171 => Some(Self::Reserved171),
            172 => Some(Self::Reserved172),
            173 => Some(Self::Reserved173),
            174 => Some(Self::Reserved174),
            175 => Some(Self::Reserved175),
            176 => Some(Self::Reserved176),
            177 => Some(Self::Reserved177),
            178 => Some(Self::Reserved178),
            179 => Some(Self::Reserved179),
            180 => Some(Self::Reserved180),
            181 => Some(Self::Reserved181),
            182 => Some(Self::Reserved182),
            183 => Some(Self::Reserved183),
            184 => Some(Self::Reserved184),
            185 => Some(Self::Reserved185),
            186 => Some(Self::Reserved186),
            187 => Some(Self::Reserved187),
            188 => Some(Self::Reserved188),
            189 => Some(Self::Reserved189),
            190 => Some(Self::Reserved190),
            191 => Some(Self::Reserved191),
            192 => Some(Self::Reserved192),
            193 => Some(Self::Reserved193),
            194 => Some(Self::Reserved194),
            195 => Some(Self::Reserved195),
            196 => Some(Self::Reserved196),
            197 => Some(Self::Reserved197),
            198 => Some(Self::Reserved198),
            199 => Some(Self::Reserved199),
            200 => Some(Self::Reserved200),
            201 => Some(Self::Reserved201),
            202 => Some(Self::Reserved202),
            203 => Some(Self::Reserved203),
            204 => Some(Self::Reserved204),
            205 => Some(Self::Reserved205),
            206 => Some(Self::Reserved206),
            207 => Some(Self::Reserved207),
            208 => Some(Self::Reserved208),
            209 => Some(Self::Reserved209),
            210 => Some(Self::Reserved210),
            211 => Some(Self::Reserved211),
            212 => Some(Self::Reserved212),
            213 => Some(Self::Reserved213),
            214 => Some(Self::Reserved214),
            215 => Some(Self::Reserved215),
            216 => Some(Self::Reserved216),
            217 => Some(Self::Reserved217),
            218 => Some(Self::Reserved218),
            219 => Some(Self::Reserved219),
            220 => Some(Self::Reserved220),
            221 => Some(Self::Reserved221),
            222 => Some(Self::Reserved222),
            223 => Some(Self::Reserved223),
            224 => Some(Self::Reserved224),
            225 => Some(Self::Reserved225),
            226 => Some(Self::Reserved226),
            227 => Some(Self::Reserved227),
            228 => Some(Self::Reserved228),
            229 => Some(Self::Reserved229),
            230 => Some(Self::Reserved230),
            231 => Some(Self::Reserved231),
            232 => Some(Self::Reserved232),
            233 => Some(Self::Reserved233),
            234 => Some(Self::Reserved234),
            235 => Some(Self::Reserved235),
            236 => Some(Self::Reserved236),
            237 => Some(Self::Reserved237),
            238 => Some(Self::Reserved238),
            239 => Some(Self::Reserved239),
            240 => Some(Self::Reserved240),
            241 => Some(Self::Reserved241),
            242 => Some(Self::Reserved242),
            243 => Some(Self::Reserved243),
            244 => Some(Self::Reserved244),
            245 => Some(Self::Reserved245),
            246 => Some(Self::Reserved246),
            247 => Some(Self::Reserved247),
            248 => Some(Self::Reserved248),
            249 => Some(Self::Reserved249),
            250 => Some(Self::Reserved250),
            251 => Some(Self::Reserved251),
            252 => Some(Self::Reserved252),
            253 => Some(Self::Reserved253),
            254 => Some(Self::Reserved254),
            255 => Some(Self::Reserved255),
            256 => Some(Self::Reserved256),
            257 => Some(Self::Reserved257),
            258 => Some(Self::Reserved258),
            259 => Some(Self::Reserved259),
            260 => Some(Self::Reserved260),
            261 => Some(Self::Reserved261),
            262 => Some(Self::Reserved262),
            263 => Some(Self::Reserved263),
            264 => Some(Self::Reserved264),
            265 => Some(Self::Reserved265),
            266 => Some(Self::Reserved266),
            267 => Some(Self::Reserved267),
            268 => Some(Self::Reserved268),
            269 => Some(Self::Reserved269),
            270 => Some(Self::Reserved270),
            271 => Some(Self::Reserved271),
            272 => Some(Self::Reserved272),
            273 => Some(Self::Reserved273),
            274 => Some(Self::Reserved274),
            275 => Some(Self::Reserved275),
            276 => Some(Self::Reserved276),
            277 => Some(Self::Reserved277),
            278 => Some(Self::Reserved278),
            279 => Some(Self::Reserved279),
            280 => Some(Self::Reserved280),
            281 => Some(Self::Reserved281),
            282 => Some(Self::Reserved282),
            283 => Some(Self::Reserved283),
            284 => Some(Self::Reserved284),
            285 => Some(Self::Reserved285),
            286 => Some(Self::Reserved286),
            287 => Some(Self::Reserved287),
            288 => Some(Self::Reserved288),
            289 => Some(Self::Reserved289),
            290 => Some(Self::Reserved290),
            291 => Some(Self::Reserved291),
            292 => Some(Self::Reserved292),
            293 => Some(Self::Reserved293),
            294 => Some(Self::Reserved294),
            295 => Some(Self::Reserved295),
            296 => Some(Self::Reserved296),
            297 => Some(Self::Reserved297),
            298 => Some(Self::Reserved298),
            299 => Some(Self::Reserved299),
            300 => Some(Self::Reserved300),
            301 => Some(Self::Reserved301),
            302 => Some(Self::Reserved302),
            303 => Some(Self::Reserved303),
            304 => Some(Self::Reserved304),
            305 => Some(Self::Reserved305),
            306 => Some(Self::Reserved306),
            307 => Some(Self::Reserved307),
            308 => Some(Self::Reserved308),
            309 => Some(Self::Reserved309),
            310 => Some(Self::Reserved310),
            311 => Some(Self::Reserved311),
            312 => Some(Self::Reserved312),
            313 => Some(Self::Reserved313),
            314 => Some(Self::Reserved314),
            315 => Some(Self::Reserved315),
            316 => Some(Self::Reserved316),
            317 => Some(Self::Reserved317),
            318 => Some(Self::Reserved318),
            319 => Some(Self::Reserved319),
            320 => Some(Self::Reserved320),
            321 => Some(Self::Reserved321),
            322 => Some(Self::Reserved322),
            323 => Some(Self::Reserved323),
            324 => Some(Self::Reserved324),
            325 => Some(Self::Reserved325),
            326 => Some(Self::Reserved326),
            327 => Some(Self::Reserved327),
            328 => Some(Self::Reserved328),
            329 => Some(Self::Reserved329),
            330 => Some(Self::Reserved330),
            331 => Some(Self::Reserved331),
            332 => Some(Self::Reserved332),
            333 => Some(Self::Reserved333),
            334 => Some(Self::Reserved334),
            335 => Some(Self::Reserved335),
            336 => Some(Self::Reserved336),
            337 => Some(Self::Reserved337),
            338 => Some(Self::Reserved338),
            339 => Some(Self::Reserved339),
            340 => Some(Self::Reserved340),
            341 => Some(Self::Reserved341),
            342 => Some(Self::Reserved342),
            343 => Some(Self::Reserved343),
            344 => Some(Self::Reserved344),
            345 => Some(Self::Reserved345),
            346 => Some(Self::Reserved346),
            347 => Some(Self::Reserved347),
            348 => Some(Self::Reserved348),
            349 => Some(Self::Reserved349),
            350 => Some(Self::Reserved350),
            351 => Some(Self::Reserved351),
            352 => Some(Self::Reserved352),
            353 => Some(Self::Reserved353),
            354 => Some(Self::Reserved354),
            355 => Some(Self::Reserved355),
            356 => Some(Self::Reserved356),
            357 => Some(Self::Reserved357),
            358 => Some(Self::Reserved358),
            359 => Some(Self::Reserved359),
            360 => Some(Self::Reserved360),
            361 => Some(Self::Reserved361),
            362 => Some(Self::Reserved362),
            363 => Some(Self::Reserved363),
            364 => Some(Self::Reserved364),
            365 => Some(Self::Reserved365),
            366 => Some(Self::Reserved366),
            367 => Some(Self::Reserved367),
            368 => Some(Self::Reserved368),
            369 => Some(Self::Reserved369),
            370 => Some(Self::Reserved370),
            371 => Some(Self::Reserved371),
            372 => Some(Self::Reserved372),
            373 => Some(Self::Reserved373),
            374 => Some(Self::Reserved374),
            375 => Some(Self::Reserved375),
            376 => Some(Self::Reserved376),
            377 => Some(Self::Reserved377),
            378 => Some(Self::Reserved378),
            379 => Some(Self::Reserved379),
            380 => Some(Self::Reserved380),
            381 => Some(Self::Reserved381),
            382 => Some(Self::Reserved382),
            383 => Some(Self::Reserved383),
            384 => Some(Self::Reserved384),
            385 => Some(Self::Reserved385),
            386 => Some(Self::Reserved386),
            387 => Some(Self::Reserved387),
            388 => Some(Self::Reserved388),
            389 => Some(Self::Reserved389),
            390 => Some(Self::Reserved390),
            391 => Some(Self::Reserved391),
            392 => Some(Self::Reserved392),
            393 => Some(Self::Reserved393),
            394 => Some(Self::Reserved394),
            395 => Some(Self::Reserved395),
            396 => Some(Self::Reserved396),
            397 => Some(Self::Reserved397),
            398 => Some(Self::Reserved398),
            399 => Some(Self::Reserved399),
            _ => None,
        }
    }
}

/// Opcode stack effect (pops, pushes) approximate.
pub fn stack_effect(op: OpVs2) -> (i8, i8) {
    match op {
        OpVs2::Nop => (0, 0),
        OpVs2::LoadConst => (0, 0),
        OpVs2::LoadLocal => (0, 0),
        OpVs2::StoreLocal => (0, 0),
        OpVs2::Add => (2, 1),
        OpVs2::Sub => (2, 1),
        OpVs2::Mul => (2, 1),
        OpVs2::Div => (2, 1),
        OpVs2::Rem => (0, 0),
        OpVs2::Eq => (2, 1),
        OpVs2::Ne => (0, 0),
        OpVs2::Lt => (0, 0),
        OpVs2::Le => (0, 0),
        OpVs2::Gt => (0, 0),
        OpVs2::Ge => (0, 0),
        OpVs2::And => (2, 1),
        OpVs2::Or => (2, 1),
        OpVs2::Not => (0, 0),
        OpVs2::Jump => (0, 0),
        OpVs2::JumpIf => (0, 0),
        OpVs2::Call => (0, 0),
        OpVs2::Ret => (0, 0),
        OpVs2::Print => (0, 0),
        OpVs2::Pop => (0, 0),
        OpVs2::Dup => (0, 0),
        OpVs2::Say => (0, 0),
        OpVs2::Menu => (0, 0),
        OpVs2::Choice => (0, 0),
        OpVs2::JumpScene => (0, 0),
        OpVs2::CallScene => (0, 0),
        OpVs2::ShowChar => (0, 0),
        OpVs2::HideChar => (0, 0),
        OpVs2::Background => (0, 0),
        OpVs2::Music => (0, 0),
        OpVs2::PushLayer => (0, 0),
        OpVs2::PopLayer => (0, 0),
        OpVs2::ShowLayer => (0, 0),
        OpVs2::HideLayer => (0, 0),
        OpVs2::SetLayerZ => (0, 0),
        OpVs2::Translate => (0, 0),
        OpVs2::Await => (0, 0),
        OpVs2::Yield => (0, 0),
        OpVs2::LoadMsg => (0, 0),
        OpVs2::StoreState => (0, 0),
        OpVs2::LoadState => (0, 0),
        OpVs2::MakeArray => (0, 0),
        OpVs2::IndexGet => (0, 0),
        OpVs2::IndexSet => (0, 0),
        OpVs2::MakeMap => (0, 0),
        OpVs2::MapGet => (0, 0),
        OpVs2::MapSet => (0, 0),
        OpVs2::Ok => (0, 0),
        OpVs2::Err => (0, 0),
        OpVs2::Some => (0, 0),
        OpVs2::None_ => (0, 0),
        OpVs2::Try => (0, 0),
        OpVs2::IsOk => (0, 0),
        OpVs2::Unwrap => (0, 0),
        OpVs2::CastI32 => (0, 0),
        OpVs2::CastF64 => (0, 0),
        OpVs2::Concat => (0, 0),
        OpVs2::Len => (0, 0),
        OpVs2::TransformApply => (0, 0),
        OpVs2::TransitionPlay => (0, 0),
        OpVs2::ActionFire => (0, 0),
        OpVs2::ScreenOpen => (0, 0),
        OpVs2::ScreenClose => (0, 0),
        OpVs2::BindButton => (0, 0),
        OpVs2::PlaySfx => (0, 0),
        OpVs2::PlayVoice => (0, 0),
        OpVs2::StopBgm => (0, 0),
        OpVs2::SetVolume => (0, 0),
        OpVs2::Reserved72 => (0, 0),
        OpVs2::Reserved73 => (0, 0),
        OpVs2::Reserved74 => (0, 0),
        OpVs2::Reserved75 => (0, 0),
        OpVs2::Reserved76 => (0, 0),
        OpVs2::Reserved77 => (0, 0),
        OpVs2::Reserved78 => (0, 0),
        OpVs2::Reserved79 => (0, 0),
        OpVs2::Reserved80 => (0, 0),
        OpVs2::Reserved81 => (0, 0),
        OpVs2::Reserved82 => (0, 0),
        OpVs2::Reserved83 => (0, 0),
        OpVs2::Reserved84 => (0, 0),
        OpVs2::Reserved85 => (0, 0),
        OpVs2::Reserved86 => (0, 0),
        OpVs2::Reserved87 => (0, 0),
        OpVs2::Reserved88 => (0, 0),
        OpVs2::Reserved89 => (0, 0),
        OpVs2::Reserved90 => (0, 0),
        OpVs2::Reserved91 => (0, 0),
        OpVs2::Reserved92 => (0, 0),
        OpVs2::Reserved93 => (0, 0),
        OpVs2::Reserved94 => (0, 0),
        OpVs2::Reserved95 => (0, 0),
        OpVs2::Reserved96 => (0, 0),
        OpVs2::Reserved97 => (0, 0),
        OpVs2::Reserved98 => (0, 0),
        OpVs2::Reserved99 => (0, 0),
        OpVs2::Reserved100 => (0, 0),
        OpVs2::Reserved101 => (0, 0),
        OpVs2::Reserved102 => (0, 0),
        OpVs2::Reserved103 => (0, 0),
        OpVs2::Reserved104 => (0, 0),
        OpVs2::Reserved105 => (0, 0),
        OpVs2::Reserved106 => (0, 0),
        OpVs2::Reserved107 => (0, 0),
        OpVs2::Reserved108 => (0, 0),
        OpVs2::Reserved109 => (0, 0),
        OpVs2::Reserved110 => (0, 0),
        OpVs2::Reserved111 => (0, 0),
        OpVs2::Reserved112 => (0, 0),
        OpVs2::Reserved113 => (0, 0),
        OpVs2::Reserved114 => (0, 0),
        OpVs2::Reserved115 => (0, 0),
        OpVs2::Reserved116 => (0, 0),
        OpVs2::Reserved117 => (0, 0),
        OpVs2::Reserved118 => (0, 0),
        OpVs2::Reserved119 => (0, 0),
        OpVs2::Reserved120 => (0, 0),
        OpVs2::Reserved121 => (0, 0),
        OpVs2::Reserved122 => (0, 0),
        OpVs2::Reserved123 => (0, 0),
        OpVs2::Reserved124 => (0, 0),
        OpVs2::Reserved125 => (0, 0),
        OpVs2::Reserved126 => (0, 0),
        OpVs2::Reserved127 => (0, 0),
        OpVs2::Reserved128 => (0, 0),
        OpVs2::Reserved129 => (0, 0),
        OpVs2::Reserved130 => (0, 0),
        OpVs2::Reserved131 => (0, 0),
        OpVs2::Reserved132 => (0, 0),
        OpVs2::Reserved133 => (0, 0),
        OpVs2::Reserved134 => (0, 0),
        OpVs2::Reserved135 => (0, 0),
        OpVs2::Reserved136 => (0, 0),
        OpVs2::Reserved137 => (0, 0),
        OpVs2::Reserved138 => (0, 0),
        OpVs2::Reserved139 => (0, 0),
        OpVs2::Reserved140 => (0, 0),
        OpVs2::Reserved141 => (0, 0),
        OpVs2::Reserved142 => (0, 0),
        OpVs2::Reserved143 => (0, 0),
        OpVs2::Reserved144 => (0, 0),
        OpVs2::Reserved145 => (0, 0),
        OpVs2::Reserved146 => (0, 0),
        OpVs2::Reserved147 => (0, 0),
        OpVs2::Reserved148 => (0, 0),
        OpVs2::Reserved149 => (0, 0),
        OpVs2::Reserved150 => (0, 0),
        OpVs2::Reserved151 => (0, 0),
        OpVs2::Reserved152 => (0, 0),
        OpVs2::Reserved153 => (0, 0),
        OpVs2::Reserved154 => (0, 0),
        OpVs2::Reserved155 => (0, 0),
        OpVs2::Reserved156 => (0, 0),
        OpVs2::Reserved157 => (0, 0),
        OpVs2::Reserved158 => (0, 0),
        OpVs2::Reserved159 => (0, 0),
        OpVs2::Reserved160 => (0, 0),
        OpVs2::Reserved161 => (0, 0),
        OpVs2::Reserved162 => (0, 0),
        OpVs2::Reserved163 => (0, 0),
        OpVs2::Reserved164 => (0, 0),
        OpVs2::Reserved165 => (0, 0),
        OpVs2::Reserved166 => (0, 0),
        OpVs2::Reserved167 => (0, 0),
        OpVs2::Reserved168 => (0, 0),
        OpVs2::Reserved169 => (0, 0),
        OpVs2::Reserved170 => (0, 0),
        OpVs2::Reserved171 => (0, 0),
        OpVs2::Reserved172 => (0, 0),
        OpVs2::Reserved173 => (0, 0),
        OpVs2::Reserved174 => (0, 0),
        OpVs2::Reserved175 => (0, 0),
        OpVs2::Reserved176 => (0, 0),
        OpVs2::Reserved177 => (0, 0),
        OpVs2::Reserved178 => (0, 0),
        OpVs2::Reserved179 => (0, 0),
        OpVs2::Reserved180 => (0, 0),
        OpVs2::Reserved181 => (0, 0),
        OpVs2::Reserved182 => (0, 0),
        OpVs2::Reserved183 => (0, 0),
        OpVs2::Reserved184 => (0, 0),
        OpVs2::Reserved185 => (0, 0),
        OpVs2::Reserved186 => (0, 0),
        OpVs2::Reserved187 => (0, 0),
        OpVs2::Reserved188 => (0, 0),
        OpVs2::Reserved189 => (0, 0),
        OpVs2::Reserved190 => (0, 0),
        OpVs2::Reserved191 => (0, 0),
        OpVs2::Reserved192 => (0, 0),
        OpVs2::Reserved193 => (0, 0),
        OpVs2::Reserved194 => (0, 0),
        OpVs2::Reserved195 => (0, 0),
        OpVs2::Reserved196 => (0, 0),
        OpVs2::Reserved197 => (0, 0),
        OpVs2::Reserved198 => (0, 0),
        OpVs2::Reserved199 => (0, 0),
        OpVs2::Reserved200 => (0, 0),
        OpVs2::Reserved201 => (0, 0),
        OpVs2::Reserved202 => (0, 0),
        OpVs2::Reserved203 => (0, 0),
        OpVs2::Reserved204 => (0, 0),
        OpVs2::Reserved205 => (0, 0),
        OpVs2::Reserved206 => (0, 0),
        OpVs2::Reserved207 => (0, 0),
        OpVs2::Reserved208 => (0, 0),
        OpVs2::Reserved209 => (0, 0),
        OpVs2::Reserved210 => (0, 0),
        OpVs2::Reserved211 => (0, 0),
        OpVs2::Reserved212 => (0, 0),
        OpVs2::Reserved213 => (0, 0),
        OpVs2::Reserved214 => (0, 0),
        OpVs2::Reserved215 => (0, 0),
        OpVs2::Reserved216 => (0, 0),
        OpVs2::Reserved217 => (0, 0),
        OpVs2::Reserved218 => (0, 0),
        OpVs2::Reserved219 => (0, 0),
        OpVs2::Reserved220 => (0, 0),
        OpVs2::Reserved221 => (0, 0),
        OpVs2::Reserved222 => (0, 0),
        OpVs2::Reserved223 => (0, 0),
        OpVs2::Reserved224 => (0, 0),
        OpVs2::Reserved225 => (0, 0),
        OpVs2::Reserved226 => (0, 0),
        OpVs2::Reserved227 => (0, 0),
        OpVs2::Reserved228 => (0, 0),
        OpVs2::Reserved229 => (0, 0),
        OpVs2::Reserved230 => (0, 0),
        OpVs2::Reserved231 => (0, 0),
        OpVs2::Reserved232 => (0, 0),
        OpVs2::Reserved233 => (0, 0),
        OpVs2::Reserved234 => (0, 0),
        OpVs2::Reserved235 => (0, 0),
        OpVs2::Reserved236 => (0, 0),
        OpVs2::Reserved237 => (0, 0),
        OpVs2::Reserved238 => (0, 0),
        OpVs2::Reserved239 => (0, 0),
        OpVs2::Reserved240 => (0, 0),
        OpVs2::Reserved241 => (0, 0),
        OpVs2::Reserved242 => (0, 0),
        OpVs2::Reserved243 => (0, 0),
        OpVs2::Reserved244 => (0, 0),
        OpVs2::Reserved245 => (0, 0),
        OpVs2::Reserved246 => (0, 0),
        OpVs2::Reserved247 => (0, 0),
        OpVs2::Reserved248 => (0, 0),
        OpVs2::Reserved249 => (0, 0),
        OpVs2::Reserved250 => (0, 0),
        OpVs2::Reserved251 => (0, 0),
        OpVs2::Reserved252 => (0, 0),
        OpVs2::Reserved253 => (0, 0),
        OpVs2::Reserved254 => (0, 0),
        OpVs2::Reserved255 => (0, 0),
        OpVs2::Reserved256 => (0, 0),
        OpVs2::Reserved257 => (0, 0),
        OpVs2::Reserved258 => (0, 0),
        OpVs2::Reserved259 => (0, 0),
        OpVs2::Reserved260 => (0, 0),
        OpVs2::Reserved261 => (0, 0),
        OpVs2::Reserved262 => (0, 0),
        OpVs2::Reserved263 => (0, 0),
        OpVs2::Reserved264 => (0, 0),
        OpVs2::Reserved265 => (0, 0),
        OpVs2::Reserved266 => (0, 0),
        OpVs2::Reserved267 => (0, 0),
        OpVs2::Reserved268 => (0, 0),
        OpVs2::Reserved269 => (0, 0),
        OpVs2::Reserved270 => (0, 0),
        OpVs2::Reserved271 => (0, 0),
        OpVs2::Reserved272 => (0, 0),
        OpVs2::Reserved273 => (0, 0),
        OpVs2::Reserved274 => (0, 0),
        OpVs2::Reserved275 => (0, 0),
        OpVs2::Reserved276 => (0, 0),
        OpVs2::Reserved277 => (0, 0),
        OpVs2::Reserved278 => (0, 0),
        OpVs2::Reserved279 => (0, 0),
        OpVs2::Reserved280 => (0, 0),
        OpVs2::Reserved281 => (0, 0),
        OpVs2::Reserved282 => (0, 0),
        OpVs2::Reserved283 => (0, 0),
        OpVs2::Reserved284 => (0, 0),
        OpVs2::Reserved285 => (0, 0),
        OpVs2::Reserved286 => (0, 0),
        OpVs2::Reserved287 => (0, 0),
        OpVs2::Reserved288 => (0, 0),
        OpVs2::Reserved289 => (0, 0),
        OpVs2::Reserved290 => (0, 0),
        OpVs2::Reserved291 => (0, 0),
        OpVs2::Reserved292 => (0, 0),
        OpVs2::Reserved293 => (0, 0),
        OpVs2::Reserved294 => (0, 0),
        OpVs2::Reserved295 => (0, 0),
        OpVs2::Reserved296 => (0, 0),
        OpVs2::Reserved297 => (0, 0),
        OpVs2::Reserved298 => (0, 0),
        OpVs2::Reserved299 => (0, 0),
        OpVs2::Reserved300 => (0, 0),
        OpVs2::Reserved301 => (0, 0),
        OpVs2::Reserved302 => (0, 0),
        OpVs2::Reserved303 => (0, 0),
        OpVs2::Reserved304 => (0, 0),
        OpVs2::Reserved305 => (0, 0),
        OpVs2::Reserved306 => (0, 0),
        OpVs2::Reserved307 => (0, 0),
        OpVs2::Reserved308 => (0, 0),
        OpVs2::Reserved309 => (0, 0),
        OpVs2::Reserved310 => (0, 0),
        OpVs2::Reserved311 => (0, 0),
        OpVs2::Reserved312 => (0, 0),
        OpVs2::Reserved313 => (0, 0),
        OpVs2::Reserved314 => (0, 0),
        OpVs2::Reserved315 => (0, 0),
        OpVs2::Reserved316 => (0, 0),
        OpVs2::Reserved317 => (0, 0),
        OpVs2::Reserved318 => (0, 0),
        OpVs2::Reserved319 => (0, 0),
        OpVs2::Reserved320 => (0, 0),
        OpVs2::Reserved321 => (0, 0),
        OpVs2::Reserved322 => (0, 0),
        OpVs2::Reserved323 => (0, 0),
        OpVs2::Reserved324 => (0, 0),
        OpVs2::Reserved325 => (0, 0),
        OpVs2::Reserved326 => (0, 0),
        OpVs2::Reserved327 => (0, 0),
        OpVs2::Reserved328 => (0, 0),
        OpVs2::Reserved329 => (0, 0),
        OpVs2::Reserved330 => (0, 0),
        OpVs2::Reserved331 => (0, 0),
        OpVs2::Reserved332 => (0, 0),
        OpVs2::Reserved333 => (0, 0),
        OpVs2::Reserved334 => (0, 0),
        OpVs2::Reserved335 => (0, 0),
        OpVs2::Reserved336 => (0, 0),
        OpVs2::Reserved337 => (0, 0),
        OpVs2::Reserved338 => (0, 0),
        OpVs2::Reserved339 => (0, 0),
        OpVs2::Reserved340 => (0, 0),
        OpVs2::Reserved341 => (0, 0),
        OpVs2::Reserved342 => (0, 0),
        OpVs2::Reserved343 => (0, 0),
        OpVs2::Reserved344 => (0, 0),
        OpVs2::Reserved345 => (0, 0),
        OpVs2::Reserved346 => (0, 0),
        OpVs2::Reserved347 => (0, 0),
        OpVs2::Reserved348 => (0, 0),
        OpVs2::Reserved349 => (0, 0),
        OpVs2::Reserved350 => (0, 0),
        OpVs2::Reserved351 => (0, 0),
        OpVs2::Reserved352 => (0, 0),
        OpVs2::Reserved353 => (0, 0),
        OpVs2::Reserved354 => (0, 0),
        OpVs2::Reserved355 => (0, 0),
        OpVs2::Reserved356 => (0, 0),
        OpVs2::Reserved357 => (0, 0),
        OpVs2::Reserved358 => (0, 0),
        OpVs2::Reserved359 => (0, 0),
        OpVs2::Reserved360 => (0, 0),
        OpVs2::Reserved361 => (0, 0),
        OpVs2::Reserved362 => (0, 0),
        OpVs2::Reserved363 => (0, 0),
        OpVs2::Reserved364 => (0, 0),
        OpVs2::Reserved365 => (0, 0),
        OpVs2::Reserved366 => (0, 0),
        OpVs2::Reserved367 => (0, 0),
        OpVs2::Reserved368 => (0, 0),
        OpVs2::Reserved369 => (0, 0),
        OpVs2::Reserved370 => (0, 0),
        OpVs2::Reserved371 => (0, 0),
        OpVs2::Reserved372 => (0, 0),
        OpVs2::Reserved373 => (0, 0),
        OpVs2::Reserved374 => (0, 0),
        OpVs2::Reserved375 => (0, 0),
        OpVs2::Reserved376 => (0, 0),
        OpVs2::Reserved377 => (0, 0),
        OpVs2::Reserved378 => (0, 0),
        OpVs2::Reserved379 => (0, 0),
        OpVs2::Reserved380 => (0, 0),
        OpVs2::Reserved381 => (0, 0),
        OpVs2::Reserved382 => (0, 0),
        OpVs2::Reserved383 => (0, 0),
        OpVs2::Reserved384 => (0, 0),
        OpVs2::Reserved385 => (0, 0),
        OpVs2::Reserved386 => (0, 0),
        OpVs2::Reserved387 => (0, 0),
        OpVs2::Reserved388 => (0, 0),
        OpVs2::Reserved389 => (0, 0),
        OpVs2::Reserved390 => (0, 0),
        OpVs2::Reserved391 => (0, 0),
        OpVs2::Reserved392 => (0, 0),
        OpVs2::Reserved393 => (0, 0),
        OpVs2::Reserved394 => (0, 0),
        OpVs2::Reserved395 => (0, 0),
        OpVs2::Reserved396 => (0, 0),
        OpVs2::Reserved397 => (0, 0),
        OpVs2::Reserved398 => (0, 0),
        OpVs2::Reserved399 => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opcodes_have_names() {
        assert_eq!(OpVs2::Nop.name(), "Nop");
        assert_eq!(OpVs2::Say.name(), "Say");
        assert_eq!(OpVs2::PushLayer.name(), "PushLayer");
        assert!(OpVs2::from_u16(0).is_some());
    }

    use super::*;
    #[test]
    fn all_400() {
        for i in 0..400u16 {
            assert!(OpVs2::from_u16(i).is_some());
        }
    }
}
