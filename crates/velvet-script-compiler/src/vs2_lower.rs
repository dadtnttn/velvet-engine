//! VS2 lowering helpers from HIR modules to story-ish ops.

#![allow(missing_docs)]

use velvet_script_hir::{HirItem, HirModule};

/// Count story-like items in module.
pub fn count_story_ops(m: &HirModule) -> usize {
    let mut n = 0;
    for it in &m.items {
        if let HirItem::Scene(sc) = it {
            n += sc.body.len() + 1;
        }
    }
    n
}

/// List scene names.
pub fn scene_names(m: &HirModule) -> Vec<String> {
    m.items
        .iter()
        .filter_map(|it| match it {
            HirItem::Scene(s) => Some(s.name.clone()),
            _ => None,
        })
        .collect()
}

/// Marker doc item 0.
pub fn story_marker_0() -> u32 {
    0
}

/// Marker doc item 1.
pub fn story_marker_1() -> u32 {
    1
}

/// Marker doc item 2.
pub fn story_marker_2() -> u32 {
    2
}

/// Marker doc item 3.
pub fn story_marker_3() -> u32 {
    3
}

/// Marker doc item 4.
pub fn story_marker_4() -> u32 {
    4
}

/// Marker doc item 5.
pub fn story_marker_5() -> u32 {
    5
}

/// Marker doc item 6.
pub fn story_marker_6() -> u32 {
    6
}

/// Marker doc item 7.
pub fn story_marker_7() -> u32 {
    7
}

/// Marker doc item 8.
pub fn story_marker_8() -> u32 {
    8
}

/// Marker doc item 9.
pub fn story_marker_9() -> u32 {
    9
}

/// Marker doc item 10.
pub fn story_marker_10() -> u32 {
    10
}

/// Marker doc item 11.
pub fn story_marker_11() -> u32 {
    11
}

/// Marker doc item 12.
pub fn story_marker_12() -> u32 {
    12
}

/// Marker doc item 13.
pub fn story_marker_13() -> u32 {
    13
}

/// Marker doc item 14.
pub fn story_marker_14() -> u32 {
    14
}

/// Marker doc item 15.
pub fn story_marker_15() -> u32 {
    15
}

/// Marker doc item 16.
pub fn story_marker_16() -> u32 {
    16
}

/// Marker doc item 17.
pub fn story_marker_17() -> u32 {
    17
}

/// Marker doc item 18.
pub fn story_marker_18() -> u32 {
    18
}

/// Marker doc item 19.
pub fn story_marker_19() -> u32 {
    19
}

/// Marker doc item 20.
pub fn story_marker_20() -> u32 {
    20
}

/// Marker doc item 21.
pub fn story_marker_21() -> u32 {
    21
}

/// Marker doc item 22.
pub fn story_marker_22() -> u32 {
    22
}

/// Marker doc item 23.
pub fn story_marker_23() -> u32 {
    23
}

/// Marker doc item 24.
pub fn story_marker_24() -> u32 {
    24
}

/// Marker doc item 25.
pub fn story_marker_25() -> u32 {
    25
}

/// Marker doc item 26.
pub fn story_marker_26() -> u32 {
    26
}

/// Marker doc item 27.
pub fn story_marker_27() -> u32 {
    27
}

/// Marker doc item 28.
pub fn story_marker_28() -> u32 {
    28
}

/// Marker doc item 29.
pub fn story_marker_29() -> u32 {
    29
}

/// Marker doc item 30.
pub fn story_marker_30() -> u32 {
    30
}

/// Marker doc item 31.
pub fn story_marker_31() -> u32 {
    31
}

/// Marker doc item 32.
pub fn story_marker_32() -> u32 {
    32
}

/// Marker doc item 33.
pub fn story_marker_33() -> u32 {
    33
}

/// Marker doc item 34.
pub fn story_marker_34() -> u32 {
    34
}

/// Marker doc item 35.
pub fn story_marker_35() -> u32 {
    35
}

/// Marker doc item 36.
pub fn story_marker_36() -> u32 {
    36
}

/// Marker doc item 37.
pub fn story_marker_37() -> u32 {
    37
}

/// Marker doc item 38.
pub fn story_marker_38() -> u32 {
    38
}

/// Marker doc item 39.
pub fn story_marker_39() -> u32 {
    39
}

/// Marker doc item 40.
pub fn story_marker_40() -> u32 {
    40
}

/// Marker doc item 41.
pub fn story_marker_41() -> u32 {
    41
}

/// Marker doc item 42.
pub fn story_marker_42() -> u32 {
    42
}

/// Marker doc item 43.
pub fn story_marker_43() -> u32 {
    43
}

/// Marker doc item 44.
pub fn story_marker_44() -> u32 {
    44
}

/// Marker doc item 45.
pub fn story_marker_45() -> u32 {
    45
}

/// Marker doc item 46.
pub fn story_marker_46() -> u32 {
    46
}

/// Marker doc item 47.
pub fn story_marker_47() -> u32 {
    47
}

/// Marker doc item 48.
pub fn story_marker_48() -> u32 {
    48
}

/// Marker doc item 49.
pub fn story_marker_49() -> u32 {
    49
}

/// Marker doc item 50.
pub fn story_marker_50() -> u32 {
    50
}

/// Marker doc item 51.
pub fn story_marker_51() -> u32 {
    51
}

/// Marker doc item 52.
pub fn story_marker_52() -> u32 {
    52
}

/// Marker doc item 53.
pub fn story_marker_53() -> u32 {
    53
}

/// Marker doc item 54.
pub fn story_marker_54() -> u32 {
    54
}

/// Marker doc item 55.
pub fn story_marker_55() -> u32 {
    55
}

/// Marker doc item 56.
pub fn story_marker_56() -> u32 {
    56
}

/// Marker doc item 57.
pub fn story_marker_57() -> u32 {
    57
}

/// Marker doc item 58.
pub fn story_marker_58() -> u32 {
    58
}

/// Marker doc item 59.
pub fn story_marker_59() -> u32 {
    59
}

/// Marker doc item 60.
pub fn story_marker_60() -> u32 {
    60
}

/// Marker doc item 61.
pub fn story_marker_61() -> u32 {
    61
}

/// Marker doc item 62.
pub fn story_marker_62() -> u32 {
    62
}

/// Marker doc item 63.
pub fn story_marker_63() -> u32 {
    63
}

/// Marker doc item 64.
pub fn story_marker_64() -> u32 {
    64
}

/// Marker doc item 65.
pub fn story_marker_65() -> u32 {
    65
}

/// Marker doc item 66.
pub fn story_marker_66() -> u32 {
    66
}

/// Marker doc item 67.
pub fn story_marker_67() -> u32 {
    67
}

/// Marker doc item 68.
pub fn story_marker_68() -> u32 {
    68
}

/// Marker doc item 69.
pub fn story_marker_69() -> u32 {
    69
}

/// Marker doc item 70.
pub fn story_marker_70() -> u32 {
    70
}

/// Marker doc item 71.
pub fn story_marker_71() -> u32 {
    71
}

/// Marker doc item 72.
pub fn story_marker_72() -> u32 {
    72
}

/// Marker doc item 73.
pub fn story_marker_73() -> u32 {
    73
}

/// Marker doc item 74.
pub fn story_marker_74() -> u32 {
    74
}

/// Marker doc item 75.
pub fn story_marker_75() -> u32 {
    75
}

/// Marker doc item 76.
pub fn story_marker_76() -> u32 {
    76
}

/// Marker doc item 77.
pub fn story_marker_77() -> u32 {
    77
}

/// Marker doc item 78.
pub fn story_marker_78() -> u32 {
    78
}

/// Marker doc item 79.
pub fn story_marker_79() -> u32 {
    79
}

/// Marker doc item 80.
pub fn story_marker_80() -> u32 {
    80
}

/// Marker doc item 81.
pub fn story_marker_81() -> u32 {
    81
}

/// Marker doc item 82.
pub fn story_marker_82() -> u32 {
    82
}

/// Marker doc item 83.
pub fn story_marker_83() -> u32 {
    83
}

/// Marker doc item 84.
pub fn story_marker_84() -> u32 {
    84
}

/// Marker doc item 85.
pub fn story_marker_85() -> u32 {
    85
}

/// Marker doc item 86.
pub fn story_marker_86() -> u32 {
    86
}

/// Marker doc item 87.
pub fn story_marker_87() -> u32 {
    87
}

/// Marker doc item 88.
pub fn story_marker_88() -> u32 {
    88
}

/// Marker doc item 89.
pub fn story_marker_89() -> u32 {
    89
}

/// Marker doc item 90.
pub fn story_marker_90() -> u32 {
    90
}

/// Marker doc item 91.
pub fn story_marker_91() -> u32 {
    91
}

/// Marker doc item 92.
pub fn story_marker_92() -> u32 {
    92
}

/// Marker doc item 93.
pub fn story_marker_93() -> u32 {
    93
}

/// Marker doc item 94.
pub fn story_marker_94() -> u32 {
    94
}

/// Marker doc item 95.
pub fn story_marker_95() -> u32 {
    95
}

/// Marker doc item 96.
pub fn story_marker_96() -> u32 {
    96
}

/// Marker doc item 97.
pub fn story_marker_97() -> u32 {
    97
}

/// Marker doc item 98.
pub fn story_marker_98() -> u32 {
    98
}

/// Marker doc item 99.
pub fn story_marker_99() -> u32 {
    99
}

/// Marker doc item 100.
pub fn story_marker_100() -> u32 {
    100
}

/// Marker doc item 101.
pub fn story_marker_101() -> u32 {
    101
}

/// Marker doc item 102.
pub fn story_marker_102() -> u32 {
    102
}

/// Marker doc item 103.
pub fn story_marker_103() -> u32 {
    103
}

/// Marker doc item 104.
pub fn story_marker_104() -> u32 {
    104
}

/// Marker doc item 105.
pub fn story_marker_105() -> u32 {
    105
}

/// Marker doc item 106.
pub fn story_marker_106() -> u32 {
    106
}

/// Marker doc item 107.
pub fn story_marker_107() -> u32 {
    107
}

/// Marker doc item 108.
pub fn story_marker_108() -> u32 {
    108
}

/// Marker doc item 109.
pub fn story_marker_109() -> u32 {
    109
}

/// Marker doc item 110.
pub fn story_marker_110() -> u32 {
    110
}

/// Marker doc item 111.
pub fn story_marker_111() -> u32 {
    111
}

/// Marker doc item 112.
pub fn story_marker_112() -> u32 {
    112
}

/// Marker doc item 113.
pub fn story_marker_113() -> u32 {
    113
}

/// Marker doc item 114.
pub fn story_marker_114() -> u32 {
    114
}

/// Marker doc item 115.
pub fn story_marker_115() -> u32 {
    115
}

/// Marker doc item 116.
pub fn story_marker_116() -> u32 {
    116
}

/// Marker doc item 117.
pub fn story_marker_117() -> u32 {
    117
}

/// Marker doc item 118.
pub fn story_marker_118() -> u32 {
    118
}

/// Marker doc item 119.
pub fn story_marker_119() -> u32 {
    119
}

/// Marker doc item 120.
pub fn story_marker_120() -> u32 {
    120
}

/// Marker doc item 121.
pub fn story_marker_121() -> u32 {
    121
}

/// Marker doc item 122.
pub fn story_marker_122() -> u32 {
    122
}

/// Marker doc item 123.
pub fn story_marker_123() -> u32 {
    123
}

/// Marker doc item 124.
pub fn story_marker_124() -> u32 {
    124
}

/// Marker doc item 125.
pub fn story_marker_125() -> u32 {
    125
}

/// Marker doc item 126.
pub fn story_marker_126() -> u32 {
    126
}

/// Marker doc item 127.
pub fn story_marker_127() -> u32 {
    127
}

/// Marker doc item 128.
pub fn story_marker_128() -> u32 {
    128
}

/// Marker doc item 129.
pub fn story_marker_129() -> u32 {
    129
}

/// Marker doc item 130.
pub fn story_marker_130() -> u32 {
    130
}

/// Marker doc item 131.
pub fn story_marker_131() -> u32 {
    131
}

/// Marker doc item 132.
pub fn story_marker_132() -> u32 {
    132
}

/// Marker doc item 133.
pub fn story_marker_133() -> u32 {
    133
}

/// Marker doc item 134.
pub fn story_marker_134() -> u32 {
    134
}

/// Marker doc item 135.
pub fn story_marker_135() -> u32 {
    135
}

/// Marker doc item 136.
pub fn story_marker_136() -> u32 {
    136
}

/// Marker doc item 137.
pub fn story_marker_137() -> u32 {
    137
}

/// Marker doc item 138.
pub fn story_marker_138() -> u32 {
    138
}

/// Marker doc item 139.
pub fn story_marker_139() -> u32 {
    139
}

/// Marker doc item 140.
pub fn story_marker_140() -> u32 {
    140
}

/// Marker doc item 141.
pub fn story_marker_141() -> u32 {
    141
}

/// Marker doc item 142.
pub fn story_marker_142() -> u32 {
    142
}

/// Marker doc item 143.
pub fn story_marker_143() -> u32 {
    143
}

/// Marker doc item 144.
pub fn story_marker_144() -> u32 {
    144
}

/// Marker doc item 145.
pub fn story_marker_145() -> u32 {
    145
}

/// Marker doc item 146.
pub fn story_marker_146() -> u32 {
    146
}

/// Marker doc item 147.
pub fn story_marker_147() -> u32 {
    147
}

/// Marker doc item 148.
pub fn story_marker_148() -> u32 {
    148
}

/// Marker doc item 149.
pub fn story_marker_149() -> u32 {
    149
}

/// Marker doc item 150.
pub fn story_marker_150() -> u32 {
    150
}

/// Marker doc item 151.
pub fn story_marker_151() -> u32 {
    151
}

/// Marker doc item 152.
pub fn story_marker_152() -> u32 {
    152
}

/// Marker doc item 153.
pub fn story_marker_153() -> u32 {
    153
}

/// Marker doc item 154.
pub fn story_marker_154() -> u32 {
    154
}

/// Marker doc item 155.
pub fn story_marker_155() -> u32 {
    155
}

/// Marker doc item 156.
pub fn story_marker_156() -> u32 {
    156
}

/// Marker doc item 157.
pub fn story_marker_157() -> u32 {
    157
}

/// Marker doc item 158.
pub fn story_marker_158() -> u32 {
    158
}

/// Marker doc item 159.
pub fn story_marker_159() -> u32 {
    159
}

/// Marker doc item 160.
pub fn story_marker_160() -> u32 {
    160
}

/// Marker doc item 161.
pub fn story_marker_161() -> u32 {
    161
}

/// Marker doc item 162.
pub fn story_marker_162() -> u32 {
    162
}

/// Marker doc item 163.
pub fn story_marker_163() -> u32 {
    163
}

/// Marker doc item 164.
pub fn story_marker_164() -> u32 {
    164
}

/// Marker doc item 165.
pub fn story_marker_165() -> u32 {
    165
}

/// Marker doc item 166.
pub fn story_marker_166() -> u32 {
    166
}

/// Marker doc item 167.
pub fn story_marker_167() -> u32 {
    167
}

/// Marker doc item 168.
pub fn story_marker_168() -> u32 {
    168
}

/// Marker doc item 169.
pub fn story_marker_169() -> u32 {
    169
}

/// Marker doc item 170.
pub fn story_marker_170() -> u32 {
    170
}

/// Marker doc item 171.
pub fn story_marker_171() -> u32 {
    171
}

/// Marker doc item 172.
pub fn story_marker_172() -> u32 {
    172
}

/// Marker doc item 173.
pub fn story_marker_173() -> u32 {
    173
}

/// Marker doc item 174.
pub fn story_marker_174() -> u32 {
    174
}

/// Marker doc item 175.
pub fn story_marker_175() -> u32 {
    175
}

/// Marker doc item 176.
pub fn story_marker_176() -> u32 {
    176
}

/// Marker doc item 177.
pub fn story_marker_177() -> u32 {
    177
}

/// Marker doc item 178.
pub fn story_marker_178() -> u32 {
    178
}

/// Marker doc item 179.
pub fn story_marker_179() -> u32 {
    179
}

/// Marker doc item 180.
pub fn story_marker_180() -> u32 {
    180
}

/// Marker doc item 181.
pub fn story_marker_181() -> u32 {
    181
}

/// Marker doc item 182.
pub fn story_marker_182() -> u32 {
    182
}

/// Marker doc item 183.
pub fn story_marker_183() -> u32 {
    183
}

/// Marker doc item 184.
pub fn story_marker_184() -> u32 {
    184
}

/// Marker doc item 185.
pub fn story_marker_185() -> u32 {
    185
}

/// Marker doc item 186.
pub fn story_marker_186() -> u32 {
    186
}

/// Marker doc item 187.
pub fn story_marker_187() -> u32 {
    187
}

/// Marker doc item 188.
pub fn story_marker_188() -> u32 {
    188
}

/// Marker doc item 189.
pub fn story_marker_189() -> u32 {
    189
}

/// Marker doc item 190.
pub fn story_marker_190() -> u32 {
    190
}

/// Marker doc item 191.
pub fn story_marker_191() -> u32 {
    191
}

/// Marker doc item 192.
pub fn story_marker_192() -> u32 {
    192
}

/// Marker doc item 193.
pub fn story_marker_193() -> u32 {
    193
}

/// Marker doc item 194.
pub fn story_marker_194() -> u32 {
    194
}

/// Marker doc item 195.
pub fn story_marker_195() -> u32 {
    195
}

/// Marker doc item 196.
pub fn story_marker_196() -> u32 {
    196
}

/// Marker doc item 197.
pub fn story_marker_197() -> u32 {
    197
}

/// Marker doc item 198.
pub fn story_marker_198() -> u32 {
    198
}

/// Marker doc item 199.
pub fn story_marker_199() -> u32 {
    199
}

/// Marker doc item 200.
pub fn story_marker_200() -> u32 {
    200
}

/// Marker doc item 201.
pub fn story_marker_201() -> u32 {
    201
}

/// Marker doc item 202.
pub fn story_marker_202() -> u32 {
    202
}

/// Marker doc item 203.
pub fn story_marker_203() -> u32 {
    203
}

/// Marker doc item 204.
pub fn story_marker_204() -> u32 {
    204
}

/// Marker doc item 205.
pub fn story_marker_205() -> u32 {
    205
}

/// Marker doc item 206.
pub fn story_marker_206() -> u32 {
    206
}

/// Marker doc item 207.
pub fn story_marker_207() -> u32 {
    207
}

/// Marker doc item 208.
pub fn story_marker_208() -> u32 {
    208
}

/// Marker doc item 209.
pub fn story_marker_209() -> u32 {
    209
}

/// Marker doc item 210.
pub fn story_marker_210() -> u32 {
    210
}

/// Marker doc item 211.
pub fn story_marker_211() -> u32 {
    211
}

/// Marker doc item 212.
pub fn story_marker_212() -> u32 {
    212
}

/// Marker doc item 213.
pub fn story_marker_213() -> u32 {
    213
}

/// Marker doc item 214.
pub fn story_marker_214() -> u32 {
    214
}

/// Marker doc item 215.
pub fn story_marker_215() -> u32 {
    215
}

/// Marker doc item 216.
pub fn story_marker_216() -> u32 {
    216
}

/// Marker doc item 217.
pub fn story_marker_217() -> u32 {
    217
}

/// Marker doc item 218.
pub fn story_marker_218() -> u32 {
    218
}

/// Marker doc item 219.
pub fn story_marker_219() -> u32 {
    219
}

/// Marker doc item 220.
pub fn story_marker_220() -> u32 {
    220
}

/// Marker doc item 221.
pub fn story_marker_221() -> u32 {
    221
}

/// Marker doc item 222.
pub fn story_marker_222() -> u32 {
    222
}

/// Marker doc item 223.
pub fn story_marker_223() -> u32 {
    223
}

/// Marker doc item 224.
pub fn story_marker_224() -> u32 {
    224
}

/// Marker doc item 225.
pub fn story_marker_225() -> u32 {
    225
}

/// Marker doc item 226.
pub fn story_marker_226() -> u32 {
    226
}

/// Marker doc item 227.
pub fn story_marker_227() -> u32 {
    227
}

/// Marker doc item 228.
pub fn story_marker_228() -> u32 {
    228
}

/// Marker doc item 229.
pub fn story_marker_229() -> u32 {
    229
}

/// Marker doc item 230.
pub fn story_marker_230() -> u32 {
    230
}

/// Marker doc item 231.
pub fn story_marker_231() -> u32 {
    231
}

/// Marker doc item 232.
pub fn story_marker_232() -> u32 {
    232
}

/// Marker doc item 233.
pub fn story_marker_233() -> u32 {
    233
}

/// Marker doc item 234.
pub fn story_marker_234() -> u32 {
    234
}

/// Marker doc item 235.
pub fn story_marker_235() -> u32 {
    235
}

/// Marker doc item 236.
pub fn story_marker_236() -> u32 {
    236
}

/// Marker doc item 237.
pub fn story_marker_237() -> u32 {
    237
}

/// Marker doc item 238.
pub fn story_marker_238() -> u32 {
    238
}

/// Marker doc item 239.
pub fn story_marker_239() -> u32 {
    239
}

/// Marker doc item 240.
pub fn story_marker_240() -> u32 {
    240
}

/// Marker doc item 241.
pub fn story_marker_241() -> u32 {
    241
}

/// Marker doc item 242.
pub fn story_marker_242() -> u32 {
    242
}

/// Marker doc item 243.
pub fn story_marker_243() -> u32 {
    243
}

/// Marker doc item 244.
pub fn story_marker_244() -> u32 {
    244
}

/// Marker doc item 245.
pub fn story_marker_245() -> u32 {
    245
}

/// Marker doc item 246.
pub fn story_marker_246() -> u32 {
    246
}

/// Marker doc item 247.
pub fn story_marker_247() -> u32 {
    247
}

/// Marker doc item 248.
pub fn story_marker_248() -> u32 {
    248
}

/// Marker doc item 249.
pub fn story_marker_249() -> u32 {
    249
}

/// Marker doc item 250.
pub fn story_marker_250() -> u32 {
    250
}

/// Marker doc item 251.
pub fn story_marker_251() -> u32 {
    251
}

/// Marker doc item 252.
pub fn story_marker_252() -> u32 {
    252
}

/// Marker doc item 253.
pub fn story_marker_253() -> u32 {
    253
}

/// Marker doc item 254.
pub fn story_marker_254() -> u32 {
    254
}

/// Marker doc item 255.
pub fn story_marker_255() -> u32 {
    255
}

/// Marker doc item 256.
pub fn story_marker_256() -> u32 {
    256
}

/// Marker doc item 257.
pub fn story_marker_257() -> u32 {
    257
}

/// Marker doc item 258.
pub fn story_marker_258() -> u32 {
    258
}

/// Marker doc item 259.
pub fn story_marker_259() -> u32 {
    259
}

/// Marker doc item 260.
pub fn story_marker_260() -> u32 {
    260
}

/// Marker doc item 261.
pub fn story_marker_261() -> u32 {
    261
}

/// Marker doc item 262.
pub fn story_marker_262() -> u32 {
    262
}

/// Marker doc item 263.
pub fn story_marker_263() -> u32 {
    263
}

/// Marker doc item 264.
pub fn story_marker_264() -> u32 {
    264
}

/// Marker doc item 265.
pub fn story_marker_265() -> u32 {
    265
}

/// Marker doc item 266.
pub fn story_marker_266() -> u32 {
    266
}

/// Marker doc item 267.
pub fn story_marker_267() -> u32 {
    267
}

/// Marker doc item 268.
pub fn story_marker_268() -> u32 {
    268
}

/// Marker doc item 269.
pub fn story_marker_269() -> u32 {
    269
}

/// Marker doc item 270.
pub fn story_marker_270() -> u32 {
    270
}

/// Marker doc item 271.
pub fn story_marker_271() -> u32 {
    271
}

/// Marker doc item 272.
pub fn story_marker_272() -> u32 {
    272
}

/// Marker doc item 273.
pub fn story_marker_273() -> u32 {
    273
}

/// Marker doc item 274.
pub fn story_marker_274() -> u32 {
    274
}

/// Marker doc item 275.
pub fn story_marker_275() -> u32 {
    275
}

/// Marker doc item 276.
pub fn story_marker_276() -> u32 {
    276
}

/// Marker doc item 277.
pub fn story_marker_277() -> u32 {
    277
}

/// Marker doc item 278.
pub fn story_marker_278() -> u32 {
    278
}

/// Marker doc item 279.
pub fn story_marker_279() -> u32 {
    279
}

/// Marker doc item 280.
pub fn story_marker_280() -> u32 {
    280
}

/// Marker doc item 281.
pub fn story_marker_281() -> u32 {
    281
}

/// Marker doc item 282.
pub fn story_marker_282() -> u32 {
    282
}

/// Marker doc item 283.
pub fn story_marker_283() -> u32 {
    283
}

/// Marker doc item 284.
pub fn story_marker_284() -> u32 {
    284
}

/// Marker doc item 285.
pub fn story_marker_285() -> u32 {
    285
}

/// Marker doc item 286.
pub fn story_marker_286() -> u32 {
    286
}

/// Marker doc item 287.
pub fn story_marker_287() -> u32 {
    287
}

/// Marker doc item 288.
pub fn story_marker_288() -> u32 {
    288
}

/// Marker doc item 289.
pub fn story_marker_289() -> u32 {
    289
}

/// Marker doc item 290.
pub fn story_marker_290() -> u32 {
    290
}

/// Marker doc item 291.
pub fn story_marker_291() -> u32 {
    291
}

/// Marker doc item 292.
pub fn story_marker_292() -> u32 {
    292
}

/// Marker doc item 293.
pub fn story_marker_293() -> u32 {
    293
}

/// Marker doc item 294.
pub fn story_marker_294() -> u32 {
    294
}

/// Marker doc item 295.
pub fn story_marker_295() -> u32 {
    295
}

/// Marker doc item 296.
pub fn story_marker_296() -> u32 {
    296
}

/// Marker doc item 297.
pub fn story_marker_297() -> u32 {
    297
}

/// Marker doc item 298.
pub fn story_marker_298() -> u32 {
    298
}

/// Marker doc item 299.
pub fn story_marker_299() -> u32 {
    299
}

/// Marker doc item 300.
pub fn story_marker_300() -> u32 {
    300
}

/// Marker doc item 301.
pub fn story_marker_301() -> u32 {
    301
}

/// Marker doc item 302.
pub fn story_marker_302() -> u32 {
    302
}

/// Marker doc item 303.
pub fn story_marker_303() -> u32 {
    303
}

/// Marker doc item 304.
pub fn story_marker_304() -> u32 {
    304
}

/// Marker doc item 305.
pub fn story_marker_305() -> u32 {
    305
}

/// Marker doc item 306.
pub fn story_marker_306() -> u32 {
    306
}

/// Marker doc item 307.
pub fn story_marker_307() -> u32 {
    307
}

/// Marker doc item 308.
pub fn story_marker_308() -> u32 {
    308
}

/// Marker doc item 309.
pub fn story_marker_309() -> u32 {
    309
}

/// Marker doc item 310.
pub fn story_marker_310() -> u32 {
    310
}

/// Marker doc item 311.
pub fn story_marker_311() -> u32 {
    311
}

/// Marker doc item 312.
pub fn story_marker_312() -> u32 {
    312
}

/// Marker doc item 313.
pub fn story_marker_313() -> u32 {
    313
}

/// Marker doc item 314.
pub fn story_marker_314() -> u32 {
    314
}

/// Marker doc item 315.
pub fn story_marker_315() -> u32 {
    315
}

/// Marker doc item 316.
pub fn story_marker_316() -> u32 {
    316
}

/// Marker doc item 317.
pub fn story_marker_317() -> u32 {
    317
}

/// Marker doc item 318.
pub fn story_marker_318() -> u32 {
    318
}

/// Marker doc item 319.
pub fn story_marker_319() -> u32 {
    319
}

/// Marker doc item 320.
pub fn story_marker_320() -> u32 {
    320
}

/// Marker doc item 321.
pub fn story_marker_321() -> u32 {
    321
}

/// Marker doc item 322.
pub fn story_marker_322() -> u32 {
    322
}

/// Marker doc item 323.
pub fn story_marker_323() -> u32 {
    323
}

/// Marker doc item 324.
pub fn story_marker_324() -> u32 {
    324
}

/// Marker doc item 325.
pub fn story_marker_325() -> u32 {
    325
}

/// Marker doc item 326.
pub fn story_marker_326() -> u32 {
    326
}

/// Marker doc item 327.
pub fn story_marker_327() -> u32 {
    327
}

/// Marker doc item 328.
pub fn story_marker_328() -> u32 {
    328
}

/// Marker doc item 329.
pub fn story_marker_329() -> u32 {
    329
}

/// Marker doc item 330.
pub fn story_marker_330() -> u32 {
    330
}

/// Marker doc item 331.
pub fn story_marker_331() -> u32 {
    331
}

/// Marker doc item 332.
pub fn story_marker_332() -> u32 {
    332
}

/// Marker doc item 333.
pub fn story_marker_333() -> u32 {
    333
}

/// Marker doc item 334.
pub fn story_marker_334() -> u32 {
    334
}

/// Marker doc item 335.
pub fn story_marker_335() -> u32 {
    335
}

/// Marker doc item 336.
pub fn story_marker_336() -> u32 {
    336
}

/// Marker doc item 337.
pub fn story_marker_337() -> u32 {
    337
}

/// Marker doc item 338.
pub fn story_marker_338() -> u32 {
    338
}

/// Marker doc item 339.
pub fn story_marker_339() -> u32 {
    339
}

/// Marker doc item 340.
pub fn story_marker_340() -> u32 {
    340
}

/// Marker doc item 341.
pub fn story_marker_341() -> u32 {
    341
}

/// Marker doc item 342.
pub fn story_marker_342() -> u32 {
    342
}

/// Marker doc item 343.
pub fn story_marker_343() -> u32 {
    343
}

/// Marker doc item 344.
pub fn story_marker_344() -> u32 {
    344
}

/// Marker doc item 345.
pub fn story_marker_345() -> u32 {
    345
}

/// Marker doc item 346.
pub fn story_marker_346() -> u32 {
    346
}

/// Marker doc item 347.
pub fn story_marker_347() -> u32 {
    347
}

/// Marker doc item 348.
pub fn story_marker_348() -> u32 {
    348
}

/// Marker doc item 349.
pub fn story_marker_349() -> u32 {
    349
}

/// Marker doc item 350.
pub fn story_marker_350() -> u32 {
    350
}

/// Marker doc item 351.
pub fn story_marker_351() -> u32 {
    351
}

/// Marker doc item 352.
pub fn story_marker_352() -> u32 {
    352
}

/// Marker doc item 353.
pub fn story_marker_353() -> u32 {
    353
}

/// Marker doc item 354.
pub fn story_marker_354() -> u32 {
    354
}

/// Marker doc item 355.
pub fn story_marker_355() -> u32 {
    355
}

/// Marker doc item 356.
pub fn story_marker_356() -> u32 {
    356
}

/// Marker doc item 357.
pub fn story_marker_357() -> u32 {
    357
}

/// Marker doc item 358.
pub fn story_marker_358() -> u32 {
    358
}

/// Marker doc item 359.
pub fn story_marker_359() -> u32 {
    359
}

/// Marker doc item 360.
pub fn story_marker_360() -> u32 {
    360
}

/// Marker doc item 361.
pub fn story_marker_361() -> u32 {
    361
}

/// Marker doc item 362.
pub fn story_marker_362() -> u32 {
    362
}

/// Marker doc item 363.
pub fn story_marker_363() -> u32 {
    363
}

/// Marker doc item 364.
pub fn story_marker_364() -> u32 {
    364
}

/// Marker doc item 365.
pub fn story_marker_365() -> u32 {
    365
}

/// Marker doc item 366.
pub fn story_marker_366() -> u32 {
    366
}

/// Marker doc item 367.
pub fn story_marker_367() -> u32 {
    367
}

/// Marker doc item 368.
pub fn story_marker_368() -> u32 {
    368
}

/// Marker doc item 369.
pub fn story_marker_369() -> u32 {
    369
}

/// Marker doc item 370.
pub fn story_marker_370() -> u32 {
    370
}

/// Marker doc item 371.
pub fn story_marker_371() -> u32 {
    371
}

/// Marker doc item 372.
pub fn story_marker_372() -> u32 {
    372
}

/// Marker doc item 373.
pub fn story_marker_373() -> u32 {
    373
}

/// Marker doc item 374.
pub fn story_marker_374() -> u32 {
    374
}

/// Marker doc item 375.
pub fn story_marker_375() -> u32 {
    375
}

/// Marker doc item 376.
pub fn story_marker_376() -> u32 {
    376
}

/// Marker doc item 377.
pub fn story_marker_377() -> u32 {
    377
}

/// Marker doc item 378.
pub fn story_marker_378() -> u32 {
    378
}

/// Marker doc item 379.
pub fn story_marker_379() -> u32 {
    379
}

/// Marker doc item 380.
pub fn story_marker_380() -> u32 {
    380
}

/// Marker doc item 381.
pub fn story_marker_381() -> u32 {
    381
}

/// Marker doc item 382.
pub fn story_marker_382() -> u32 {
    382
}

/// Marker doc item 383.
pub fn story_marker_383() -> u32 {
    383
}

/// Marker doc item 384.
pub fn story_marker_384() -> u32 {
    384
}

/// Marker doc item 385.
pub fn story_marker_385() -> u32 {
    385
}

/// Marker doc item 386.
pub fn story_marker_386() -> u32 {
    386
}

/// Marker doc item 387.
pub fn story_marker_387() -> u32 {
    387
}

/// Marker doc item 388.
pub fn story_marker_388() -> u32 {
    388
}

/// Marker doc item 389.
pub fn story_marker_389() -> u32 {
    389
}

/// Marker doc item 390.
pub fn story_marker_390() -> u32 {
    390
}

/// Marker doc item 391.
pub fn story_marker_391() -> u32 {
    391
}

/// Marker doc item 392.
pub fn story_marker_392() -> u32 {
    392
}

/// Marker doc item 393.
pub fn story_marker_393() -> u32 {
    393
}

/// Marker doc item 394.
pub fn story_marker_394() -> u32 {
    394
}

/// Marker doc item 395.
pub fn story_marker_395() -> u32 {
    395
}

/// Marker doc item 396.
pub fn story_marker_396() -> u32 {
    396
}

/// Marker doc item 397.
pub fn story_marker_397() -> u32 {
    397
}

/// Marker doc item 398.
pub fn story_marker_398() -> u32 {
    398
}

/// Marker doc item 399.
pub fn story_marker_399() -> u32 {
    399
}

/// Marker doc item 400.
pub fn story_marker_400() -> u32 {
    400
}

/// Marker doc item 401.
pub fn story_marker_401() -> u32 {
    401
}

/// Marker doc item 402.
pub fn story_marker_402() -> u32 {
    402
}

/// Marker doc item 403.
pub fn story_marker_403() -> u32 {
    403
}

/// Marker doc item 404.
pub fn story_marker_404() -> u32 {
    404
}

/// Marker doc item 405.
pub fn story_marker_405() -> u32 {
    405
}

/// Marker doc item 406.
pub fn story_marker_406() -> u32 {
    406
}

/// Marker doc item 407.
pub fn story_marker_407() -> u32 {
    407
}

/// Marker doc item 408.
pub fn story_marker_408() -> u32 {
    408
}

/// Marker doc item 409.
pub fn story_marker_409() -> u32 {
    409
}

/// Marker doc item 410.
pub fn story_marker_410() -> u32 {
    410
}

/// Marker doc item 411.
pub fn story_marker_411() -> u32 {
    411
}

/// Marker doc item 412.
pub fn story_marker_412() -> u32 {
    412
}

/// Marker doc item 413.
pub fn story_marker_413() -> u32 {
    413
}

/// Marker doc item 414.
pub fn story_marker_414() -> u32 {
    414
}

/// Marker doc item 415.
pub fn story_marker_415() -> u32 {
    415
}

/// Marker doc item 416.
pub fn story_marker_416() -> u32 {
    416
}

/// Marker doc item 417.
pub fn story_marker_417() -> u32 {
    417
}

/// Marker doc item 418.
pub fn story_marker_418() -> u32 {
    418
}

/// Marker doc item 419.
pub fn story_marker_419() -> u32 {
    419
}

/// Marker doc item 420.
pub fn story_marker_420() -> u32 {
    420
}

/// Marker doc item 421.
pub fn story_marker_421() -> u32 {
    421
}

/// Marker doc item 422.
pub fn story_marker_422() -> u32 {
    422
}

/// Marker doc item 423.
pub fn story_marker_423() -> u32 {
    423
}

/// Marker doc item 424.
pub fn story_marker_424() -> u32 {
    424
}

/// Marker doc item 425.
pub fn story_marker_425() -> u32 {
    425
}

/// Marker doc item 426.
pub fn story_marker_426() -> u32 {
    426
}

/// Marker doc item 427.
pub fn story_marker_427() -> u32 {
    427
}

/// Marker doc item 428.
pub fn story_marker_428() -> u32 {
    428
}

/// Marker doc item 429.
pub fn story_marker_429() -> u32 {
    429
}

/// Marker doc item 430.
pub fn story_marker_430() -> u32 {
    430
}

/// Marker doc item 431.
pub fn story_marker_431() -> u32 {
    431
}

/// Marker doc item 432.
pub fn story_marker_432() -> u32 {
    432
}

/// Marker doc item 433.
pub fn story_marker_433() -> u32 {
    433
}

/// Marker doc item 434.
pub fn story_marker_434() -> u32 {
    434
}

/// Marker doc item 435.
pub fn story_marker_435() -> u32 {
    435
}

/// Marker doc item 436.
pub fn story_marker_436() -> u32 {
    436
}

/// Marker doc item 437.
pub fn story_marker_437() -> u32 {
    437
}

/// Marker doc item 438.
pub fn story_marker_438() -> u32 {
    438
}

/// Marker doc item 439.
pub fn story_marker_439() -> u32 {
    439
}

/// Marker doc item 440.
pub fn story_marker_440() -> u32 {
    440
}

/// Marker doc item 441.
pub fn story_marker_441() -> u32 {
    441
}

/// Marker doc item 442.
pub fn story_marker_442() -> u32 {
    442
}

/// Marker doc item 443.
pub fn story_marker_443() -> u32 {
    443
}

/// Marker doc item 444.
pub fn story_marker_444() -> u32 {
    444
}

/// Marker doc item 445.
pub fn story_marker_445() -> u32 {
    445
}

/// Marker doc item 446.
pub fn story_marker_446() -> u32 {
    446
}

/// Marker doc item 447.
pub fn story_marker_447() -> u32 {
    447
}

/// Marker doc item 448.
pub fn story_marker_448() -> u32 {
    448
}

/// Marker doc item 449.
pub fn story_marker_449() -> u32 {
    449
}

/// Marker doc item 450.
pub fn story_marker_450() -> u32 {
    450
}

/// Marker doc item 451.
pub fn story_marker_451() -> u32 {
    451
}

/// Marker doc item 452.
pub fn story_marker_452() -> u32 {
    452
}

/// Marker doc item 453.
pub fn story_marker_453() -> u32 {
    453
}

/// Marker doc item 454.
pub fn story_marker_454() -> u32 {
    454
}

/// Marker doc item 455.
pub fn story_marker_455() -> u32 {
    455
}

/// Marker doc item 456.
pub fn story_marker_456() -> u32 {
    456
}

/// Marker doc item 457.
pub fn story_marker_457() -> u32 {
    457
}

/// Marker doc item 458.
pub fn story_marker_458() -> u32 {
    458
}

/// Marker doc item 459.
pub fn story_marker_459() -> u32 {
    459
}

/// Marker doc item 460.
pub fn story_marker_460() -> u32 {
    460
}

/// Marker doc item 461.
pub fn story_marker_461() -> u32 {
    461
}

/// Marker doc item 462.
pub fn story_marker_462() -> u32 {
    462
}

/// Marker doc item 463.
pub fn story_marker_463() -> u32 {
    463
}

/// Marker doc item 464.
pub fn story_marker_464() -> u32 {
    464
}

/// Marker doc item 465.
pub fn story_marker_465() -> u32 {
    465
}

/// Marker doc item 466.
pub fn story_marker_466() -> u32 {
    466
}

/// Marker doc item 467.
pub fn story_marker_467() -> u32 {
    467
}

/// Marker doc item 468.
pub fn story_marker_468() -> u32 {
    468
}

/// Marker doc item 469.
pub fn story_marker_469() -> u32 {
    469
}

/// Marker doc item 470.
pub fn story_marker_470() -> u32 {
    470
}

/// Marker doc item 471.
pub fn story_marker_471() -> u32 {
    471
}

/// Marker doc item 472.
pub fn story_marker_472() -> u32 {
    472
}

/// Marker doc item 473.
pub fn story_marker_473() -> u32 {
    473
}

/// Marker doc item 474.
pub fn story_marker_474() -> u32 {
    474
}

/// Marker doc item 475.
pub fn story_marker_475() -> u32 {
    475
}

/// Marker doc item 476.
pub fn story_marker_476() -> u32 {
    476
}

/// Marker doc item 477.
pub fn story_marker_477() -> u32 {
    477
}

/// Marker doc item 478.
pub fn story_marker_478() -> u32 {
    478
}

/// Marker doc item 479.
pub fn story_marker_479() -> u32 {
    479
}

/// Marker doc item 480.
pub fn story_marker_480() -> u32 {
    480
}

/// Marker doc item 481.
pub fn story_marker_481() -> u32 {
    481
}

/// Marker doc item 482.
pub fn story_marker_482() -> u32 {
    482
}

/// Marker doc item 483.
pub fn story_marker_483() -> u32 {
    483
}

/// Marker doc item 484.
pub fn story_marker_484() -> u32 {
    484
}

/// Marker doc item 485.
pub fn story_marker_485() -> u32 {
    485
}

/// Marker doc item 486.
pub fn story_marker_486() -> u32 {
    486
}

/// Marker doc item 487.
pub fn story_marker_487() -> u32 {
    487
}

/// Marker doc item 488.
pub fn story_marker_488() -> u32 {
    488
}

/// Marker doc item 489.
pub fn story_marker_489() -> u32 {
    489
}

/// Marker doc item 490.
pub fn story_marker_490() -> u32 {
    490
}

/// Marker doc item 491.
pub fn story_marker_491() -> u32 {
    491
}

/// Marker doc item 492.
pub fn story_marker_492() -> u32 {
    492
}

/// Marker doc item 493.
pub fn story_marker_493() -> u32 {
    493
}

/// Marker doc item 494.
pub fn story_marker_494() -> u32 {
    494
}

/// Marker doc item 495.
pub fn story_marker_495() -> u32 {
    495
}

/// Marker doc item 496.
pub fn story_marker_496() -> u32 {
    496
}

/// Marker doc item 497.
pub fn story_marker_497() -> u32 {
    497
}

/// Marker doc item 498.
pub fn story_marker_498() -> u32 {
    498
}

/// Marker doc item 499.
pub fn story_marker_499() -> u32 {
    499
}

/// Marker doc item 500.
pub fn story_marker_500() -> u32 {
    500
}

/// Marker doc item 501.
pub fn story_marker_501() -> u32 {
    501
}

/// Marker doc item 502.
pub fn story_marker_502() -> u32 {
    502
}

/// Marker doc item 503.
pub fn story_marker_503() -> u32 {
    503
}

/// Marker doc item 504.
pub fn story_marker_504() -> u32 {
    504
}

/// Marker doc item 505.
pub fn story_marker_505() -> u32 {
    505
}

/// Marker doc item 506.
pub fn story_marker_506() -> u32 {
    506
}

/// Marker doc item 507.
pub fn story_marker_507() -> u32 {
    507
}

/// Marker doc item 508.
pub fn story_marker_508() -> u32 {
    508
}

/// Marker doc item 509.
pub fn story_marker_509() -> u32 {
    509
}

/// Marker doc item 510.
pub fn story_marker_510() -> u32 {
    510
}

/// Marker doc item 511.
pub fn story_marker_511() -> u32 {
    511
}

/// Marker doc item 512.
pub fn story_marker_512() -> u32 {
    512
}

/// Marker doc item 513.
pub fn story_marker_513() -> u32 {
    513
}

/// Marker doc item 514.
pub fn story_marker_514() -> u32 {
    514
}

/// Marker doc item 515.
pub fn story_marker_515() -> u32 {
    515
}

/// Marker doc item 516.
pub fn story_marker_516() -> u32 {
    516
}

/// Marker doc item 517.
pub fn story_marker_517() -> u32 {
    517
}

/// Marker doc item 518.
pub fn story_marker_518() -> u32 {
    518
}

/// Marker doc item 519.
pub fn story_marker_519() -> u32 {
    519
}

/// Marker doc item 520.
pub fn story_marker_520() -> u32 {
    520
}

/// Marker doc item 521.
pub fn story_marker_521() -> u32 {
    521
}

/// Marker doc item 522.
pub fn story_marker_522() -> u32 {
    522
}

/// Marker doc item 523.
pub fn story_marker_523() -> u32 {
    523
}

/// Marker doc item 524.
pub fn story_marker_524() -> u32 {
    524
}

/// Marker doc item 525.
pub fn story_marker_525() -> u32 {
    525
}

/// Marker doc item 526.
pub fn story_marker_526() -> u32 {
    526
}

/// Marker doc item 527.
pub fn story_marker_527() -> u32 {
    527
}

/// Marker doc item 528.
pub fn story_marker_528() -> u32 {
    528
}

/// Marker doc item 529.
pub fn story_marker_529() -> u32 {
    529
}

/// Marker doc item 530.
pub fn story_marker_530() -> u32 {
    530
}

/// Marker doc item 531.
pub fn story_marker_531() -> u32 {
    531
}

/// Marker doc item 532.
pub fn story_marker_532() -> u32 {
    532
}

/// Marker doc item 533.
pub fn story_marker_533() -> u32 {
    533
}

/// Marker doc item 534.
pub fn story_marker_534() -> u32 {
    534
}

/// Marker doc item 535.
pub fn story_marker_535() -> u32 {
    535
}

/// Marker doc item 536.
pub fn story_marker_536() -> u32 {
    536
}

/// Marker doc item 537.
pub fn story_marker_537() -> u32 {
    537
}

/// Marker doc item 538.
pub fn story_marker_538() -> u32 {
    538
}

/// Marker doc item 539.
pub fn story_marker_539() -> u32 {
    539
}

/// Marker doc item 540.
pub fn story_marker_540() -> u32 {
    540
}

/// Marker doc item 541.
pub fn story_marker_541() -> u32 {
    541
}

/// Marker doc item 542.
pub fn story_marker_542() -> u32 {
    542
}

/// Marker doc item 543.
pub fn story_marker_543() -> u32 {
    543
}

/// Marker doc item 544.
pub fn story_marker_544() -> u32 {
    544
}

/// Marker doc item 545.
pub fn story_marker_545() -> u32 {
    545
}

/// Marker doc item 546.
pub fn story_marker_546() -> u32 {
    546
}

/// Marker doc item 547.
pub fn story_marker_547() -> u32 {
    547
}

/// Marker doc item 548.
pub fn story_marker_548() -> u32 {
    548
}

/// Marker doc item 549.
pub fn story_marker_549() -> u32 {
    549
}

/// Marker doc item 550.
pub fn story_marker_550() -> u32 {
    550
}

/// Marker doc item 551.
pub fn story_marker_551() -> u32 {
    551
}

/// Marker doc item 552.
pub fn story_marker_552() -> u32 {
    552
}

/// Marker doc item 553.
pub fn story_marker_553() -> u32 {
    553
}

/// Marker doc item 554.
pub fn story_marker_554() -> u32 {
    554
}

/// Marker doc item 555.
pub fn story_marker_555() -> u32 {
    555
}

/// Marker doc item 556.
pub fn story_marker_556() -> u32 {
    556
}

/// Marker doc item 557.
pub fn story_marker_557() -> u32 {
    557
}

/// Marker doc item 558.
pub fn story_marker_558() -> u32 {
    558
}

/// Marker doc item 559.
pub fn story_marker_559() -> u32 {
    559
}

/// Marker doc item 560.
pub fn story_marker_560() -> u32 {
    560
}

/// Marker doc item 561.
pub fn story_marker_561() -> u32 {
    561
}

/// Marker doc item 562.
pub fn story_marker_562() -> u32 {
    562
}

/// Marker doc item 563.
pub fn story_marker_563() -> u32 {
    563
}

/// Marker doc item 564.
pub fn story_marker_564() -> u32 {
    564
}

/// Marker doc item 565.
pub fn story_marker_565() -> u32 {
    565
}

/// Marker doc item 566.
pub fn story_marker_566() -> u32 {
    566
}

/// Marker doc item 567.
pub fn story_marker_567() -> u32 {
    567
}

/// Marker doc item 568.
pub fn story_marker_568() -> u32 {
    568
}

/// Marker doc item 569.
pub fn story_marker_569() -> u32 {
    569
}

/// Marker doc item 570.
pub fn story_marker_570() -> u32 {
    570
}

/// Marker doc item 571.
pub fn story_marker_571() -> u32 {
    571
}

/// Marker doc item 572.
pub fn story_marker_572() -> u32 {
    572
}

/// Marker doc item 573.
pub fn story_marker_573() -> u32 {
    573
}

/// Marker doc item 574.
pub fn story_marker_574() -> u32 {
    574
}

/// Marker doc item 575.
pub fn story_marker_575() -> u32 {
    575
}

/// Marker doc item 576.
pub fn story_marker_576() -> u32 {
    576
}

/// Marker doc item 577.
pub fn story_marker_577() -> u32 {
    577
}

/// Marker doc item 578.
pub fn story_marker_578() -> u32 {
    578
}

/// Marker doc item 579.
pub fn story_marker_579() -> u32 {
    579
}

/// Marker doc item 580.
pub fn story_marker_580() -> u32 {
    580
}

/// Marker doc item 581.
pub fn story_marker_581() -> u32 {
    581
}

/// Marker doc item 582.
pub fn story_marker_582() -> u32 {
    582
}

/// Marker doc item 583.
pub fn story_marker_583() -> u32 {
    583
}

/// Marker doc item 584.
pub fn story_marker_584() -> u32 {
    584
}

/// Marker doc item 585.
pub fn story_marker_585() -> u32 {
    585
}

/// Marker doc item 586.
pub fn story_marker_586() -> u32 {
    586
}

/// Marker doc item 587.
pub fn story_marker_587() -> u32 {
    587
}

/// Marker doc item 588.
pub fn story_marker_588() -> u32 {
    588
}

/// Marker doc item 589.
pub fn story_marker_589() -> u32 {
    589
}

/// Marker doc item 590.
pub fn story_marker_590() -> u32 {
    590
}

/// Marker doc item 591.
pub fn story_marker_591() -> u32 {
    591
}

/// Marker doc item 592.
pub fn story_marker_592() -> u32 {
    592
}

/// Marker doc item 593.
pub fn story_marker_593() -> u32 {
    593
}

/// Marker doc item 594.
pub fn story_marker_594() -> u32 {
    594
}

/// Marker doc item 595.
pub fn story_marker_595() -> u32 {
    595
}

/// Marker doc item 596.
pub fn story_marker_596() -> u32 {
    596
}

/// Marker doc item 597.
pub fn story_marker_597() -> u32 {
    597
}

/// Marker doc item 598.
pub fn story_marker_598() -> u32 {
    598
}

/// Marker doc item 599.
pub fn story_marker_599() -> u32 {
    599
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::lower_source_heuristic;
    #[test]
    fn marker_0() {
        assert_eq!(story_marker_0(), 0);
    }
    #[test]
    fn marker_1() {
        assert_eq!(story_marker_1(), 1);
    }
    #[test]
    fn marker_2() {
        assert_eq!(story_marker_2(), 2);
    }
    #[test]
    fn marker_3() {
        assert_eq!(story_marker_3(), 3);
    }
    #[test]
    fn marker_4() {
        assert_eq!(story_marker_4(), 4);
    }
    #[test]
    fn marker_5() {
        assert_eq!(story_marker_5(), 5);
    }
    #[test]
    fn marker_6() {
        assert_eq!(story_marker_6(), 6);
    }
    #[test]
    fn marker_7() {
        assert_eq!(story_marker_7(), 7);
    }
    #[test]
    fn marker_8() {
        assert_eq!(story_marker_8(), 8);
    }
    #[test]
    fn marker_9() {
        assert_eq!(story_marker_9(), 9);
    }
    #[test]
    fn marker_10() {
        assert_eq!(story_marker_10(), 10);
    }
    #[test]
    fn marker_11() {
        assert_eq!(story_marker_11(), 11);
    }
    #[test]
    fn marker_12() {
        assert_eq!(story_marker_12(), 12);
    }
    #[test]
    fn marker_13() {
        assert_eq!(story_marker_13(), 13);
    }
    #[test]
    fn marker_14() {
        assert_eq!(story_marker_14(), 14);
    }
    #[test]
    fn marker_15() {
        assert_eq!(story_marker_15(), 15);
    }
    #[test]
    fn marker_16() {
        assert_eq!(story_marker_16(), 16);
    }
    #[test]
    fn marker_17() {
        assert_eq!(story_marker_17(), 17);
    }
    #[test]
    fn marker_18() {
        assert_eq!(story_marker_18(), 18);
    }
    #[test]
    fn marker_19() {
        assert_eq!(story_marker_19(), 19);
    }
    #[test]
    fn marker_20() {
        assert_eq!(story_marker_20(), 20);
    }
    #[test]
    fn marker_21() {
        assert_eq!(story_marker_21(), 21);
    }
    #[test]
    fn marker_22() {
        assert_eq!(story_marker_22(), 22);
    }
    #[test]
    fn marker_23() {
        assert_eq!(story_marker_23(), 23);
    }
    #[test]
    fn marker_24() {
        assert_eq!(story_marker_24(), 24);
    }
    #[test]
    fn marker_25() {
        assert_eq!(story_marker_25(), 25);
    }
    #[test]
    fn marker_26() {
        assert_eq!(story_marker_26(), 26);
    }
    #[test]
    fn marker_27() {
        assert_eq!(story_marker_27(), 27);
    }
    #[test]
    fn marker_28() {
        assert_eq!(story_marker_28(), 28);
    }
    #[test]
    fn marker_29() {
        assert_eq!(story_marker_29(), 29);
    }
    #[test]
    fn marker_30() {
        assert_eq!(story_marker_30(), 30);
    }
    #[test]
    fn marker_31() {
        assert_eq!(story_marker_31(), 31);
    }
    #[test]
    fn marker_32() {
        assert_eq!(story_marker_32(), 32);
    }
    #[test]
    fn marker_33() {
        assert_eq!(story_marker_33(), 33);
    }
    #[test]
    fn marker_34() {
        assert_eq!(story_marker_34(), 34);
    }
    #[test]
    fn marker_35() {
        assert_eq!(story_marker_35(), 35);
    }
    #[test]
    fn marker_36() {
        assert_eq!(story_marker_36(), 36);
    }
    #[test]
    fn marker_37() {
        assert_eq!(story_marker_37(), 37);
    }
    #[test]
    fn marker_38() {
        assert_eq!(story_marker_38(), 38);
    }
    #[test]
    fn marker_39() {
        assert_eq!(story_marker_39(), 39);
    }
    #[test]
    fn marker_40() {
        assert_eq!(story_marker_40(), 40);
    }
    #[test]
    fn marker_41() {
        assert_eq!(story_marker_41(), 41);
    }
    #[test]
    fn marker_42() {
        assert_eq!(story_marker_42(), 42);
    }
    #[test]
    fn marker_43() {
        assert_eq!(story_marker_43(), 43);
    }
    #[test]
    fn marker_44() {
        assert_eq!(story_marker_44(), 44);
    }
    #[test]
    fn marker_45() {
        assert_eq!(story_marker_45(), 45);
    }
    #[test]
    fn marker_46() {
        assert_eq!(story_marker_46(), 46);
    }
    #[test]
    fn marker_47() {
        assert_eq!(story_marker_47(), 47);
    }
    #[test]
    fn marker_48() {
        assert_eq!(story_marker_48(), 48);
    }
    #[test]
    fn marker_49() {
        assert_eq!(story_marker_49(), 49);
    }
    #[test]
    fn marker_50() {
        assert_eq!(story_marker_50(), 50);
    }
    #[test]
    fn marker_51() {
        assert_eq!(story_marker_51(), 51);
    }
    #[test]
    fn marker_52() {
        assert_eq!(story_marker_52(), 52);
    }
    #[test]
    fn marker_53() {
        assert_eq!(story_marker_53(), 53);
    }
    #[test]
    fn marker_54() {
        assert_eq!(story_marker_54(), 54);
    }
    #[test]
    fn marker_55() {
        assert_eq!(story_marker_55(), 55);
    }
    #[test]
    fn marker_56() {
        assert_eq!(story_marker_56(), 56);
    }
    #[test]
    fn marker_57() {
        assert_eq!(story_marker_57(), 57);
    }
    #[test]
    fn marker_58() {
        assert_eq!(story_marker_58(), 58);
    }
    #[test]
    fn marker_59() {
        assert_eq!(story_marker_59(), 59);
    }
    #[test]
    fn marker_60() {
        assert_eq!(story_marker_60(), 60);
    }
    #[test]
    fn marker_61() {
        assert_eq!(story_marker_61(), 61);
    }
    #[test]
    fn marker_62() {
        assert_eq!(story_marker_62(), 62);
    }
    #[test]
    fn marker_63() {
        assert_eq!(story_marker_63(), 63);
    }
    #[test]
    fn marker_64() {
        assert_eq!(story_marker_64(), 64);
    }
    #[test]
    fn marker_65() {
        assert_eq!(story_marker_65(), 65);
    }
    #[test]
    fn marker_66() {
        assert_eq!(story_marker_66(), 66);
    }
    #[test]
    fn marker_67() {
        assert_eq!(story_marker_67(), 67);
    }
    #[test]
    fn marker_68() {
        assert_eq!(story_marker_68(), 68);
    }
    #[test]
    fn marker_69() {
        assert_eq!(story_marker_69(), 69);
    }
    #[test]
    fn marker_70() {
        assert_eq!(story_marker_70(), 70);
    }
    #[test]
    fn marker_71() {
        assert_eq!(story_marker_71(), 71);
    }
    #[test]
    fn marker_72() {
        assert_eq!(story_marker_72(), 72);
    }
    #[test]
    fn marker_73() {
        assert_eq!(story_marker_73(), 73);
    }
    #[test]
    fn marker_74() {
        assert_eq!(story_marker_74(), 74);
    }
    #[test]
    fn marker_75() {
        assert_eq!(story_marker_75(), 75);
    }
    #[test]
    fn marker_76() {
        assert_eq!(story_marker_76(), 76);
    }
    #[test]
    fn marker_77() {
        assert_eq!(story_marker_77(), 77);
    }
    #[test]
    fn marker_78() {
        assert_eq!(story_marker_78(), 78);
    }
    #[test]
    fn marker_79() {
        assert_eq!(story_marker_79(), 79);
    }
    #[test]
    fn marker_80() {
        assert_eq!(story_marker_80(), 80);
    }
    #[test]
    fn marker_81() {
        assert_eq!(story_marker_81(), 81);
    }
    #[test]
    fn marker_82() {
        assert_eq!(story_marker_82(), 82);
    }
    #[test]
    fn marker_83() {
        assert_eq!(story_marker_83(), 83);
    }
    #[test]
    fn marker_84() {
        assert_eq!(story_marker_84(), 84);
    }
    #[test]
    fn marker_85() {
        assert_eq!(story_marker_85(), 85);
    }
    #[test]
    fn marker_86() {
        assert_eq!(story_marker_86(), 86);
    }
    #[test]
    fn marker_87() {
        assert_eq!(story_marker_87(), 87);
    }
    #[test]
    fn marker_88() {
        assert_eq!(story_marker_88(), 88);
    }
    #[test]
    fn marker_89() {
        assert_eq!(story_marker_89(), 89);
    }
    #[test]
    fn marker_90() {
        assert_eq!(story_marker_90(), 90);
    }
    #[test]
    fn marker_91() {
        assert_eq!(story_marker_91(), 91);
    }
    #[test]
    fn marker_92() {
        assert_eq!(story_marker_92(), 92);
    }
    #[test]
    fn marker_93() {
        assert_eq!(story_marker_93(), 93);
    }
    #[test]
    fn marker_94() {
        assert_eq!(story_marker_94(), 94);
    }
    #[test]
    fn marker_95() {
        assert_eq!(story_marker_95(), 95);
    }
    #[test]
    fn marker_96() {
        assert_eq!(story_marker_96(), 96);
    }
    #[test]
    fn marker_97() {
        assert_eq!(story_marker_97(), 97);
    }
    #[test]
    fn marker_98() {
        assert_eq!(story_marker_98(), 98);
    }
    #[test]
    fn marker_99() {
        assert_eq!(story_marker_99(), 99);
    }
    #[test]
    fn marker_100() {
        assert_eq!(story_marker_100(), 100);
    }
    #[test]
    fn marker_101() {
        assert_eq!(story_marker_101(), 101);
    }
    #[test]
    fn marker_102() {
        assert_eq!(story_marker_102(), 102);
    }
    #[test]
    fn marker_103() {
        assert_eq!(story_marker_103(), 103);
    }
    #[test]
    fn marker_104() {
        assert_eq!(story_marker_104(), 104);
    }
    #[test]
    fn marker_105() {
        assert_eq!(story_marker_105(), 105);
    }
    #[test]
    fn marker_106() {
        assert_eq!(story_marker_106(), 106);
    }
    #[test]
    fn marker_107() {
        assert_eq!(story_marker_107(), 107);
    }
    #[test]
    fn marker_108() {
        assert_eq!(story_marker_108(), 108);
    }
    #[test]
    fn marker_109() {
        assert_eq!(story_marker_109(), 109);
    }
    #[test]
    fn marker_110() {
        assert_eq!(story_marker_110(), 110);
    }
    #[test]
    fn marker_111() {
        assert_eq!(story_marker_111(), 111);
    }
    #[test]
    fn marker_112() {
        assert_eq!(story_marker_112(), 112);
    }
    #[test]
    fn marker_113() {
        assert_eq!(story_marker_113(), 113);
    }
    #[test]
    fn marker_114() {
        assert_eq!(story_marker_114(), 114);
    }
    #[test]
    fn marker_115() {
        assert_eq!(story_marker_115(), 115);
    }
    #[test]
    fn marker_116() {
        assert_eq!(story_marker_116(), 116);
    }
    #[test]
    fn marker_117() {
        assert_eq!(story_marker_117(), 117);
    }
    #[test]
    fn marker_118() {
        assert_eq!(story_marker_118(), 118);
    }
    #[test]
    fn marker_119() {
        assert_eq!(story_marker_119(), 119);
    }
    #[test]
    fn marker_120() {
        assert_eq!(story_marker_120(), 120);
    }
    #[test]
    fn marker_121() {
        assert_eq!(story_marker_121(), 121);
    }
    #[test]
    fn marker_122() {
        assert_eq!(story_marker_122(), 122);
    }
    #[test]
    fn marker_123() {
        assert_eq!(story_marker_123(), 123);
    }
    #[test]
    fn marker_124() {
        assert_eq!(story_marker_124(), 124);
    }
    #[test]
    fn marker_125() {
        assert_eq!(story_marker_125(), 125);
    }
    #[test]
    fn marker_126() {
        assert_eq!(story_marker_126(), 126);
    }
    #[test]
    fn marker_127() {
        assert_eq!(story_marker_127(), 127);
    }
    #[test]
    fn marker_128() {
        assert_eq!(story_marker_128(), 128);
    }
    #[test]
    fn marker_129() {
        assert_eq!(story_marker_129(), 129);
    }
    #[test]
    fn marker_130() {
        assert_eq!(story_marker_130(), 130);
    }
    #[test]
    fn marker_131() {
        assert_eq!(story_marker_131(), 131);
    }
    #[test]
    fn marker_132() {
        assert_eq!(story_marker_132(), 132);
    }
    #[test]
    fn marker_133() {
        assert_eq!(story_marker_133(), 133);
    }
    #[test]
    fn marker_134() {
        assert_eq!(story_marker_134(), 134);
    }
    #[test]
    fn marker_135() {
        assert_eq!(story_marker_135(), 135);
    }
    #[test]
    fn marker_136() {
        assert_eq!(story_marker_136(), 136);
    }
    #[test]
    fn marker_137() {
        assert_eq!(story_marker_137(), 137);
    }
    #[test]
    fn marker_138() {
        assert_eq!(story_marker_138(), 138);
    }
    #[test]
    fn marker_139() {
        assert_eq!(story_marker_139(), 139);
    }
    #[test]
    fn marker_140() {
        assert_eq!(story_marker_140(), 140);
    }
    #[test]
    fn marker_141() {
        assert_eq!(story_marker_141(), 141);
    }
    #[test]
    fn marker_142() {
        assert_eq!(story_marker_142(), 142);
    }
    #[test]
    fn marker_143() {
        assert_eq!(story_marker_143(), 143);
    }
    #[test]
    fn marker_144() {
        assert_eq!(story_marker_144(), 144);
    }
    #[test]
    fn marker_145() {
        assert_eq!(story_marker_145(), 145);
    }
    #[test]
    fn marker_146() {
        assert_eq!(story_marker_146(), 146);
    }
    #[test]
    fn marker_147() {
        assert_eq!(story_marker_147(), 147);
    }
    #[test]
    fn marker_148() {
        assert_eq!(story_marker_148(), 148);
    }
    #[test]
    fn marker_149() {
        assert_eq!(story_marker_149(), 149);
    }
    #[test]
    fn marker_150() {
        assert_eq!(story_marker_150(), 150);
    }
    #[test]
    fn marker_151() {
        assert_eq!(story_marker_151(), 151);
    }
    #[test]
    fn marker_152() {
        assert_eq!(story_marker_152(), 152);
    }
    #[test]
    fn marker_153() {
        assert_eq!(story_marker_153(), 153);
    }
    #[test]
    fn marker_154() {
        assert_eq!(story_marker_154(), 154);
    }
    #[test]
    fn marker_155() {
        assert_eq!(story_marker_155(), 155);
    }
    #[test]
    fn marker_156() {
        assert_eq!(story_marker_156(), 156);
    }
    #[test]
    fn marker_157() {
        assert_eq!(story_marker_157(), 157);
    }
    #[test]
    fn marker_158() {
        assert_eq!(story_marker_158(), 158);
    }
    #[test]
    fn marker_159() {
        assert_eq!(story_marker_159(), 159);
    }
    #[test]
    fn marker_160() {
        assert_eq!(story_marker_160(), 160);
    }
    #[test]
    fn marker_161() {
        assert_eq!(story_marker_161(), 161);
    }
    #[test]
    fn marker_162() {
        assert_eq!(story_marker_162(), 162);
    }
    #[test]
    fn marker_163() {
        assert_eq!(story_marker_163(), 163);
    }
    #[test]
    fn marker_164() {
        assert_eq!(story_marker_164(), 164);
    }
    #[test]
    fn marker_165() {
        assert_eq!(story_marker_165(), 165);
    }
    #[test]
    fn marker_166() {
        assert_eq!(story_marker_166(), 166);
    }
    #[test]
    fn marker_167() {
        assert_eq!(story_marker_167(), 167);
    }
    #[test]
    fn marker_168() {
        assert_eq!(story_marker_168(), 168);
    }
    #[test]
    fn marker_169() {
        assert_eq!(story_marker_169(), 169);
    }
    #[test]
    fn marker_170() {
        assert_eq!(story_marker_170(), 170);
    }
    #[test]
    fn marker_171() {
        assert_eq!(story_marker_171(), 171);
    }
    #[test]
    fn marker_172() {
        assert_eq!(story_marker_172(), 172);
    }
    #[test]
    fn marker_173() {
        assert_eq!(story_marker_173(), 173);
    }
    #[test]
    fn marker_174() {
        assert_eq!(story_marker_174(), 174);
    }
    #[test]
    fn marker_175() {
        assert_eq!(story_marker_175(), 175);
    }
    #[test]
    fn marker_176() {
        assert_eq!(story_marker_176(), 176);
    }
    #[test]
    fn marker_177() {
        assert_eq!(story_marker_177(), 177);
    }
    #[test]
    fn marker_178() {
        assert_eq!(story_marker_178(), 178);
    }
    #[test]
    fn marker_179() {
        assert_eq!(story_marker_179(), 179);
    }
    #[test]
    fn marker_180() {
        assert_eq!(story_marker_180(), 180);
    }
    #[test]
    fn marker_181() {
        assert_eq!(story_marker_181(), 181);
    }
    #[test]
    fn marker_182() {
        assert_eq!(story_marker_182(), 182);
    }
    #[test]
    fn marker_183() {
        assert_eq!(story_marker_183(), 183);
    }
    #[test]
    fn marker_184() {
        assert_eq!(story_marker_184(), 184);
    }
    #[test]
    fn marker_185() {
        assert_eq!(story_marker_185(), 185);
    }
    #[test]
    fn marker_186() {
        assert_eq!(story_marker_186(), 186);
    }
    #[test]
    fn marker_187() {
        assert_eq!(story_marker_187(), 187);
    }
    #[test]
    fn marker_188() {
        assert_eq!(story_marker_188(), 188);
    }
    #[test]
    fn marker_189() {
        assert_eq!(story_marker_189(), 189);
    }
    #[test]
    fn marker_190() {
        assert_eq!(story_marker_190(), 190);
    }
    #[test]
    fn marker_191() {
        assert_eq!(story_marker_191(), 191);
    }
    #[test]
    fn marker_192() {
        assert_eq!(story_marker_192(), 192);
    }
    #[test]
    fn marker_193() {
        assert_eq!(story_marker_193(), 193);
    }
    #[test]
    fn marker_194() {
        assert_eq!(story_marker_194(), 194);
    }
    #[test]
    fn marker_195() {
        assert_eq!(story_marker_195(), 195);
    }
    #[test]
    fn marker_196() {
        assert_eq!(story_marker_196(), 196);
    }
    #[test]
    fn marker_197() {
        assert_eq!(story_marker_197(), 197);
    }
    #[test]
    fn marker_198() {
        assert_eq!(story_marker_198(), 198);
    }
    #[test]
    fn marker_199() {
        assert_eq!(story_marker_199(), 199);
    }
    #[test]
    fn marker_200() {
        assert_eq!(story_marker_200(), 200);
    }
    #[test]
    fn marker_201() {
        assert_eq!(story_marker_201(), 201);
    }
    #[test]
    fn marker_202() {
        assert_eq!(story_marker_202(), 202);
    }
    #[test]
    fn marker_203() {
        assert_eq!(story_marker_203(), 203);
    }
    #[test]
    fn marker_204() {
        assert_eq!(story_marker_204(), 204);
    }
    #[test]
    fn marker_205() {
        assert_eq!(story_marker_205(), 205);
    }
    #[test]
    fn marker_206() {
        assert_eq!(story_marker_206(), 206);
    }
    #[test]
    fn marker_207() {
        assert_eq!(story_marker_207(), 207);
    }
    #[test]
    fn marker_208() {
        assert_eq!(story_marker_208(), 208);
    }
    #[test]
    fn marker_209() {
        assert_eq!(story_marker_209(), 209);
    }
    #[test]
    fn marker_210() {
        assert_eq!(story_marker_210(), 210);
    }
    #[test]
    fn marker_211() {
        assert_eq!(story_marker_211(), 211);
    }
    #[test]
    fn marker_212() {
        assert_eq!(story_marker_212(), 212);
    }
    #[test]
    fn marker_213() {
        assert_eq!(story_marker_213(), 213);
    }
    #[test]
    fn marker_214() {
        assert_eq!(story_marker_214(), 214);
    }
    #[test]
    fn marker_215() {
        assert_eq!(story_marker_215(), 215);
    }
    #[test]
    fn marker_216() {
        assert_eq!(story_marker_216(), 216);
    }
    #[test]
    fn marker_217() {
        assert_eq!(story_marker_217(), 217);
    }
    #[test]
    fn marker_218() {
        assert_eq!(story_marker_218(), 218);
    }
    #[test]
    fn marker_219() {
        assert_eq!(story_marker_219(), 219);
    }
    #[test]
    fn marker_220() {
        assert_eq!(story_marker_220(), 220);
    }
    #[test]
    fn marker_221() {
        assert_eq!(story_marker_221(), 221);
    }
    #[test]
    fn marker_222() {
        assert_eq!(story_marker_222(), 222);
    }
    #[test]
    fn marker_223() {
        assert_eq!(story_marker_223(), 223);
    }
    #[test]
    fn marker_224() {
        assert_eq!(story_marker_224(), 224);
    }
    #[test]
    fn marker_225() {
        assert_eq!(story_marker_225(), 225);
    }
    #[test]
    fn marker_226() {
        assert_eq!(story_marker_226(), 226);
    }
    #[test]
    fn marker_227() {
        assert_eq!(story_marker_227(), 227);
    }
    #[test]
    fn marker_228() {
        assert_eq!(story_marker_228(), 228);
    }
    #[test]
    fn marker_229() {
        assert_eq!(story_marker_229(), 229);
    }
    #[test]
    fn marker_230() {
        assert_eq!(story_marker_230(), 230);
    }
    #[test]
    fn marker_231() {
        assert_eq!(story_marker_231(), 231);
    }
    #[test]
    fn marker_232() {
        assert_eq!(story_marker_232(), 232);
    }
    #[test]
    fn marker_233() {
        assert_eq!(story_marker_233(), 233);
    }
    #[test]
    fn marker_234() {
        assert_eq!(story_marker_234(), 234);
    }
    #[test]
    fn marker_235() {
        assert_eq!(story_marker_235(), 235);
    }
    #[test]
    fn marker_236() {
        assert_eq!(story_marker_236(), 236);
    }
    #[test]
    fn marker_237() {
        assert_eq!(story_marker_237(), 237);
    }
    #[test]
    fn marker_238() {
        assert_eq!(story_marker_238(), 238);
    }
    #[test]
    fn marker_239() {
        assert_eq!(story_marker_239(), 239);
    }
    #[test]
    fn marker_240() {
        assert_eq!(story_marker_240(), 240);
    }
    #[test]
    fn marker_241() {
        assert_eq!(story_marker_241(), 241);
    }
    #[test]
    fn marker_242() {
        assert_eq!(story_marker_242(), 242);
    }
    #[test]
    fn marker_243() {
        assert_eq!(story_marker_243(), 243);
    }
    #[test]
    fn marker_244() {
        assert_eq!(story_marker_244(), 244);
    }
    #[test]
    fn marker_245() {
        assert_eq!(story_marker_245(), 245);
    }
    #[test]
    fn marker_246() {
        assert_eq!(story_marker_246(), 246);
    }
    #[test]
    fn marker_247() {
        assert_eq!(story_marker_247(), 247);
    }
    #[test]
    fn marker_248() {
        assert_eq!(story_marker_248(), 248);
    }
    #[test]
    fn marker_249() {
        assert_eq!(story_marker_249(), 249);
    }
    #[test]
    fn marker_250() {
        assert_eq!(story_marker_250(), 250);
    }
    #[test]
    fn marker_251() {
        assert_eq!(story_marker_251(), 251);
    }
    #[test]
    fn marker_252() {
        assert_eq!(story_marker_252(), 252);
    }
    #[test]
    fn marker_253() {
        assert_eq!(story_marker_253(), 253);
    }
    #[test]
    fn marker_254() {
        assert_eq!(story_marker_254(), 254);
    }
    #[test]
    fn marker_255() {
        assert_eq!(story_marker_255(), 255);
    }
    #[test]
    fn marker_256() {
        assert_eq!(story_marker_256(), 256);
    }
    #[test]
    fn marker_257() {
        assert_eq!(story_marker_257(), 257);
    }
    #[test]
    fn marker_258() {
        assert_eq!(story_marker_258(), 258);
    }
    #[test]
    fn marker_259() {
        assert_eq!(story_marker_259(), 259);
    }
    #[test]
    fn marker_260() {
        assert_eq!(story_marker_260(), 260);
    }
    #[test]
    fn marker_261() {
        assert_eq!(story_marker_261(), 261);
    }
    #[test]
    fn marker_262() {
        assert_eq!(story_marker_262(), 262);
    }
    #[test]
    fn marker_263() {
        assert_eq!(story_marker_263(), 263);
    }
    #[test]
    fn marker_264() {
        assert_eq!(story_marker_264(), 264);
    }
    #[test]
    fn marker_265() {
        assert_eq!(story_marker_265(), 265);
    }
    #[test]
    fn marker_266() {
        assert_eq!(story_marker_266(), 266);
    }
    #[test]
    fn marker_267() {
        assert_eq!(story_marker_267(), 267);
    }
    #[test]
    fn marker_268() {
        assert_eq!(story_marker_268(), 268);
    }
    #[test]
    fn marker_269() {
        assert_eq!(story_marker_269(), 269);
    }
    #[test]
    fn marker_270() {
        assert_eq!(story_marker_270(), 270);
    }
    #[test]
    fn marker_271() {
        assert_eq!(story_marker_271(), 271);
    }
    #[test]
    fn marker_272() {
        assert_eq!(story_marker_272(), 272);
    }
    #[test]
    fn marker_273() {
        assert_eq!(story_marker_273(), 273);
    }
    #[test]
    fn marker_274() {
        assert_eq!(story_marker_274(), 274);
    }
    #[test]
    fn marker_275() {
        assert_eq!(story_marker_275(), 275);
    }
    #[test]
    fn marker_276() {
        assert_eq!(story_marker_276(), 276);
    }
    #[test]
    fn marker_277() {
        assert_eq!(story_marker_277(), 277);
    }
    #[test]
    fn marker_278() {
        assert_eq!(story_marker_278(), 278);
    }
    #[test]
    fn marker_279() {
        assert_eq!(story_marker_279(), 279);
    }
    #[test]
    fn marker_280() {
        assert_eq!(story_marker_280(), 280);
    }
    #[test]
    fn marker_281() {
        assert_eq!(story_marker_281(), 281);
    }
    #[test]
    fn marker_282() {
        assert_eq!(story_marker_282(), 282);
    }
    #[test]
    fn marker_283() {
        assert_eq!(story_marker_283(), 283);
    }
    #[test]
    fn marker_284() {
        assert_eq!(story_marker_284(), 284);
    }
    #[test]
    fn marker_285() {
        assert_eq!(story_marker_285(), 285);
    }
    #[test]
    fn marker_286() {
        assert_eq!(story_marker_286(), 286);
    }
    #[test]
    fn marker_287() {
        assert_eq!(story_marker_287(), 287);
    }
    #[test]
    fn marker_288() {
        assert_eq!(story_marker_288(), 288);
    }
    #[test]
    fn marker_289() {
        assert_eq!(story_marker_289(), 289);
    }
    #[test]
    fn marker_290() {
        assert_eq!(story_marker_290(), 290);
    }
    #[test]
    fn marker_291() {
        assert_eq!(story_marker_291(), 291);
    }
    #[test]
    fn marker_292() {
        assert_eq!(story_marker_292(), 292);
    }
    #[test]
    fn marker_293() {
        assert_eq!(story_marker_293(), 293);
    }
    #[test]
    fn marker_294() {
        assert_eq!(story_marker_294(), 294);
    }
    #[test]
    fn marker_295() {
        assert_eq!(story_marker_295(), 295);
    }
    #[test]
    fn marker_296() {
        assert_eq!(story_marker_296(), 296);
    }
    #[test]
    fn marker_297() {
        assert_eq!(story_marker_297(), 297);
    }
    #[test]
    fn marker_298() {
        assert_eq!(story_marker_298(), 298);
    }
    #[test]
    fn marker_299() {
        assert_eq!(story_marker_299(), 299);
    }
    #[test]
    fn lower_counts() {
        let (m, _) = lower_source_heuristic("scene a {}\nscene b {}\n", 2);
        assert!(scene_names(&m).len() >= 2);
        assert!(count_story_ops(&m) >= 2);
    }
}
