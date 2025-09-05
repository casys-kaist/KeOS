.section .text
.macro mk_intr_with_ec, name, target
    .globl \name
        .type   \name, @function
    \name:
        .cfi_startproc
        .cfi_def_cfa rsp, 48
        .cfi_offset rsp, -16
        .cfi_offset rip, -40
        // ss -8
        // rsp -16
        // rflags -24
        // cs -32
        // rip -40
        // error code -48
        cld
        // \name\target: jmp \name\target
        sub rsp, 120
        .cfi_def_cfa_offset 168
        mov [rsp + 0x70], rax
        .cfi_offset rax, -0x48
        mov [rsp + 0x68], rbx
        .cfi_offset rbx, -0x50
        mov [rsp + 0x60], rcx
        .cfi_offset rcx, -0x58
        mov [rsp + 0x58], rdx
        .cfi_offset rdx, -0x60
        mov [rsp + 0x50], rbp
        .cfi_offset rbp, -0x68
        mov [rsp + 0x48], rdi
        .cfi_offset rdi, -0x70
        mov [rsp + 0x40], rsi
        .cfi_offset rsi, -0x78
        mov [rsp + 0x38], r8
        .cfi_offset r8, -0x80
        mov [rsp + 0x30], r9
        .cfi_offset r9, -0x88
        mov [rsp + 0x28], r10
        .cfi_offset r10, -0x90
        mov [rsp + 0x20], r11
        .cfi_offset r11, -0x98
        mov [rsp + 0x18], r12
        .cfi_offset r12, -0xa0
        mov [rsp + 0x10], r13
        .cfi_offset r13, -0xa8
        mov [rsp + 0x8], r14
        .cfi_offset r14, -0xb0
        mov [rsp], r15
        .cfi_offset r15, -0xb8
        mov rsi, [rsp + 0x78]
        mov gs:[32], rsp
        mov rdi, rsp
        call \target
        mov DWORD PTR gs:[32], 0
        mov DWORD PTR gs:[36], 0
        mov rax, [rsp + 0x70]
        .cfi_same_value rax
        mov rbx, [rsp + 0x68]
        .cfi_same_value rbx
        mov rcx, [rsp + 0x60]
        .cfi_same_value rcx
        mov rdx, [rsp + 0x58]
        .cfi_same_value rdx
        mov rbp, [rsp + 0x50]
        .cfi_same_value rbp
        mov rdi, [rsp + 0x48]
        .cfi_same_value rdi
        mov rsi, [rsp + 0x40]
        .cfi_same_value rsi
        mov r8, [rsp + 0x38]
        .cfi_same_value r8
        mov r9, [rsp + 0x30]
        .cfi_same_value r9
        mov r10, [rsp + 0x28]
        .cfi_same_value r12
        mov r11, [rsp + 0x20]
        .cfi_same_value r11
        mov r12, [rsp + 0x18]
        .cfi_same_value r12
        mov r13, [rsp + 0x10]
        .cfi_same_value r13
        mov r14, [rsp + 0x8]
        .cfi_same_value r14
        mov r15, [rsp]
        .cfi_same_value r15
        add rsp, 128
        .cfi_def_cfa_offset 48
        iretq
        .cfi_endproc
.endm

.macro mk_intr_no_ec, name, target
    .globl \name
        .type   \name, @function
    \name:
        .cfi_startproc
        .cfi_def_cfa rsp, 48
        .cfi_offset rsp, -16
        .cfi_offset rip, -40
        // ss -8
        // rsp -16
        // rflags -24
        // cs -32
        // rip -40
        // error code -48
        cld
        // \name\target: jmp \name\target
        sub rsp, 128
        .cfi_def_cfa_offset 168
        mov [rsp + 0x70], rax
        .cfi_offset rax, -0x48
        mov [rsp + 0x68], rbx
        .cfi_offset rbx, -0x50
        mov [rsp + 0x60], rcx
        .cfi_offset rcx, -0x58
        mov [rsp + 0x58], rdx
        .cfi_offset rdx, -0x60
        mov [rsp + 0x50], rbp
        .cfi_offset rbp, -0x68
        mov [rsp + 0x48], rdi
        .cfi_offset rdi, -0x70
        mov [rsp + 0x40], rsi
        .cfi_offset rsi, -0x78
        mov [rsp + 0x38], r8
        .cfi_offset r8, -0x80
        mov [rsp + 0x30], r9
        .cfi_offset r9, -0x88
        mov [rsp + 0x28], r10
        .cfi_offset r10, -0x90
        mov [rsp + 0x20], r11
        .cfi_offset r11, -0x98
        mov [rsp + 0x18], r12
        .cfi_offset r12, -0xa0
        mov [rsp + 0x10], r13
        .cfi_offset r13, -0xa8
        mov [rsp + 0x8], r14
        .cfi_offset r14, -0xb0
        mov [rsp], r15
        .cfi_offset r15, -0xb8
        mov rsi, [rsp + 0x88]
        mov gs:[32], rsp
        mov rdi, rsp
        call \target
        mov DWORD PTR gs:[32], 0
        mov DWORD PTR gs:[36], 0
        mov rax, [rsp + 0x70]
        .cfi_same_value rax
        mov rbx, [rsp + 0x68]
        .cfi_same_value rbx
        mov rcx, [rsp + 0x60]
        .cfi_same_value rcx
        mov rdx, [rsp + 0x58]
        .cfi_same_value rdx
        mov rbp, [rsp + 0x50]
        .cfi_same_value rbp
        mov rdi, [rsp + 0x48]
        .cfi_same_value rdi
        mov rsi, [rsp + 0x40]
        .cfi_same_value rsi
        mov r8, [rsp + 0x38]
        .cfi_same_value r8
        mov r9, [rsp + 0x30]
        .cfi_same_value r9
        mov r10, [rsp + 0x28]
        .cfi_same_value r12
        mov r11, [rsp + 0x20]
        .cfi_same_value r11
        mov r12, [rsp + 0x18]
        .cfi_same_value r12
        mov r13, [rsp + 0x10]
        .cfi_same_value r13
        mov r14, [rsp + 0x8]
        .cfi_same_value r14
        mov r15, [rsp]
        .cfi_same_value r15
        add rsp, 128
        .cfi_def_cfa_offset 48
        iretq
        .cfi_endproc
.endm

.macro mk_isr, name, which
    .globl \name
        .type   \name, @function
    \name:
        .cfi_startproc
        .cfi_def_cfa rsp, 48
        .cfi_offset rsp, -16
        .cfi_offset rip, -40
        cld
        sub rsp, 128
        .cfi_def_cfa_offset 168
        mov [rsp + 0x70], rax
        .cfi_offset rax, -0x48
        mov [rsp + 0x68], rbx
        .cfi_offset rbx, -0x50
        mov [rsp + 0x60], rcx
        .cfi_offset rcx, -0x58
        mov [rsp + 0x58], rdx
        .cfi_offset rdx, -0x60
        mov [rsp + 0x50], rbp
        .cfi_offset rbp, -0x68
        mov [rsp + 0x48], rdi
        .cfi_offset rdi, -0x70
        mov [rsp + 0x40], rsi
        .cfi_offset rsi, -0x78
        mov [rsp + 0x38], r8
        .cfi_offset r8, -0x80
        mov [rsp + 0x30], r9
        .cfi_offset r9, -0x88
        mov [rsp + 0x28], r10
        .cfi_offset r10, -0x90
        mov [rsp + 0x20], r11
        .cfi_offset r11, -0x98
        mov [rsp + 0x18], r12
        .cfi_offset r12, -0xa0
        mov [rsp + 0x10], r13
        .cfi_offset r13, -0xa8
        mov [rsp + 0x8], r14
        .cfi_offset r14, -0xb0
        mov [rsp], r15
        .cfi_offset r15, -0xb8
        mov rsi, \which
        mov rdi, rsp
        call do_handle_irq
        mov rax, [rsp + 0x70]
        .cfi_same_value rax
        mov rbx, [rsp + 0x68]
        .cfi_same_value rbx
        mov rcx, [rsp + 0x60]
        .cfi_same_value rcx
        mov rdx, [rsp + 0x58]
        .cfi_same_value rdx
        mov rbp, [rsp + 0x50]
        .cfi_same_value rbp
        mov rdi, [rsp + 0x48]
        .cfi_same_value rdi
        mov rsi, [rsp + 0x40]
        .cfi_same_value rsi
        mov r8, [rsp + 0x38]
        .cfi_same_value r8
        mov r9, [rsp + 0x30]
        .cfi_same_value r9
        mov r10, [rsp + 0x28]
        .cfi_same_value r10
        mov r11, [rsp + 0x20]
        .cfi_same_value r11
        mov r12, [rsp + 0x18]
        .cfi_same_value r12
        mov r13, [rsp + 0x10]
        .cfi_same_value r13
        mov r14, [rsp + 0x8]
        .cfi_same_value r14
        mov r15, [rsp]
        .cfi_same_value r15
        add rsp, 128
        .cfi_def_cfa_offset 56
        iretq
        .cfi_endproc
.endm

mk_intr_with_ec page_fault handle_page_fault
mk_intr_no_ec nmi handle_nmi
mk_intr_no_ec double_fault handle_double_fault
mk_intr_with_ec general_protection_fault handle_general_protection_fault
mk_intr_no_ec device_not_available handle_device_not_available
mk_intr_no_ec invalid_opcode handle_invalid_opcode
mk_intr_no_ec simd_floating_point_exception handle_simd_floating_point_exception

mk_isr do_isr_32 32
mk_isr do_isr_33 33
mk_isr do_isr_34 34
mk_isr do_isr_35 35
mk_isr do_isr_36 36
mk_isr do_isr_37 37
mk_isr do_isr_38 38
mk_isr do_isr_39 39
mk_isr do_isr_40 40
mk_isr do_isr_41 41
mk_isr do_isr_42 42
mk_isr do_isr_43 43
mk_isr do_isr_44 44
mk_isr do_isr_45 45
mk_isr do_isr_46 46
mk_isr do_isr_47 47
mk_isr do_isr_48 48
mk_isr do_isr_49 49
mk_isr do_isr_50 50
mk_isr do_isr_51 51
mk_isr do_isr_52 52
mk_isr do_isr_53 53
mk_isr do_isr_54 54
mk_isr do_isr_55 55
mk_isr do_isr_56 56
mk_isr do_isr_57 57
mk_isr do_isr_58 58
mk_isr do_isr_59 59
mk_isr do_isr_60 60
mk_isr do_isr_61 61
mk_isr do_isr_62 62
mk_isr do_isr_63 63
mk_isr do_isr_64 64
mk_isr do_isr_65 65
mk_isr do_isr_66 66
mk_isr do_isr_67 67
mk_isr do_isr_68 68
mk_isr do_isr_69 69
mk_isr do_isr_70 70
mk_isr do_isr_71 71
mk_isr do_isr_72 72
mk_isr do_isr_73 73
mk_isr do_isr_74 74
mk_isr do_isr_75 75
mk_isr do_isr_76 76
mk_isr do_isr_77 77
mk_isr do_isr_78 78
mk_isr do_isr_79 79
mk_isr do_isr_80 80
mk_isr do_isr_81 81
mk_isr do_isr_82 82
mk_isr do_isr_83 83
mk_isr do_isr_84 84
mk_isr do_isr_85 85
mk_isr do_isr_86 86
mk_isr do_isr_87 87
mk_isr do_isr_88 88
mk_isr do_isr_89 89
mk_isr do_isr_90 90
mk_isr do_isr_91 91
mk_isr do_isr_92 92
mk_isr do_isr_93 93
mk_isr do_isr_94 94
mk_isr do_isr_95 95
mk_isr do_isr_96 96
mk_isr do_isr_97 97
mk_isr do_isr_98 98
mk_isr do_isr_99 99
mk_isr do_isr_100 100
mk_isr do_isr_101 101
mk_isr do_isr_102 102
mk_isr do_isr_103 103
mk_isr do_isr_104 104
mk_isr do_isr_105 105
mk_isr do_isr_106 106
mk_isr do_isr_107 107
mk_isr do_isr_108 108
mk_isr do_isr_109 109
mk_isr do_isr_110 110
mk_isr do_isr_111 111
mk_isr do_isr_112 112
mk_isr do_isr_113 113
mk_isr do_isr_114 114
mk_isr do_isr_115 115
mk_isr do_isr_116 116
mk_isr do_isr_117 117
mk_isr do_isr_118 118
mk_isr do_isr_119 119
mk_isr do_isr_120 120
mk_isr do_isr_121 121
mk_isr do_isr_122 122
mk_isr do_isr_123 123
mk_isr do_isr_124 124
mk_isr do_isr_125 125
mk_isr do_isr_126 126
mk_isr do_isr_127 127
mk_isr do_isr_128 128
mk_isr do_isr_129 129
mk_isr do_isr_130 130
mk_isr do_isr_131 131
mk_isr do_isr_132 132
mk_isr do_isr_133 133
mk_isr do_isr_134 134
mk_isr do_isr_135 135
mk_isr do_isr_136 136
mk_isr do_isr_137 137
mk_isr do_isr_138 138
mk_isr do_isr_139 139
mk_isr do_isr_140 140
mk_isr do_isr_141 141
mk_isr do_isr_142 142
mk_isr do_isr_143 143
mk_isr do_isr_144 144
mk_isr do_isr_145 145
mk_isr do_isr_146 146
mk_isr do_isr_147 147
mk_isr do_isr_148 148
mk_isr do_isr_149 149
mk_isr do_isr_150 150
mk_isr do_isr_151 151
mk_isr do_isr_152 152
mk_isr do_isr_153 153
mk_isr do_isr_154 154
mk_isr do_isr_155 155
mk_isr do_isr_156 156
mk_isr do_isr_157 157
mk_isr do_isr_158 158
mk_isr do_isr_159 159
mk_isr do_isr_160 160
mk_isr do_isr_161 161
mk_isr do_isr_162 162
mk_isr do_isr_163 163
mk_isr do_isr_164 164
mk_isr do_isr_165 165
mk_isr do_isr_166 166
mk_isr do_isr_167 167
mk_isr do_isr_168 168
mk_isr do_isr_169 169
mk_isr do_isr_170 170
mk_isr do_isr_171 171
mk_isr do_isr_172 172
mk_isr do_isr_173 173
mk_isr do_isr_174 174
mk_isr do_isr_175 175
mk_isr do_isr_176 176
mk_isr do_isr_177 177
mk_isr do_isr_178 178
mk_isr do_isr_179 179
mk_isr do_isr_180 180
mk_isr do_isr_181 181
mk_isr do_isr_182 182
mk_isr do_isr_183 183
mk_isr do_isr_184 184
mk_isr do_isr_185 185
mk_isr do_isr_186 186
mk_isr do_isr_187 187
mk_isr do_isr_188 188
mk_isr do_isr_189 189
mk_isr do_isr_190 190
mk_isr do_isr_191 191
mk_isr do_isr_192 192
mk_isr do_isr_193 193
mk_isr do_isr_194 194
mk_isr do_isr_195 195
mk_isr do_isr_196 196
mk_isr do_isr_197 197
mk_isr do_isr_198 198
mk_isr do_isr_199 199
mk_isr do_isr_200 200
mk_isr do_isr_201 201
mk_isr do_isr_202 202
mk_isr do_isr_203 203
mk_isr do_isr_204 204
mk_isr do_isr_205 205
mk_isr do_isr_206 206
mk_isr do_isr_207 207
mk_isr do_isr_208 208
mk_isr do_isr_209 209
mk_isr do_isr_210 210
mk_isr do_isr_211 211
mk_isr do_isr_212 212
mk_isr do_isr_213 213
mk_isr do_isr_214 214
mk_isr do_isr_215 215
mk_isr do_isr_216 216
mk_isr do_isr_217 217
mk_isr do_isr_218 218
mk_isr do_isr_219 219
mk_isr do_isr_220 220
mk_isr do_isr_221 221
mk_isr do_isr_222 222
mk_isr do_isr_223 223
mk_isr do_isr_224 224
mk_isr do_isr_225 225
mk_isr do_isr_226 226
mk_isr do_isr_227 227
mk_isr do_isr_228 228
mk_isr do_isr_229 229
mk_isr do_isr_230 230
mk_isr do_isr_231 231
mk_isr do_isr_232 232
mk_isr do_isr_233 233
mk_isr do_isr_234 234
mk_isr do_isr_235 235
mk_isr do_isr_236 236
mk_isr do_isr_237 237
mk_isr do_isr_238 238
mk_isr do_isr_239 239
mk_isr do_isr_240 240
mk_isr do_isr_241 241
mk_isr do_isr_242 242
mk_isr do_isr_243 243
mk_isr do_isr_244 244
mk_isr do_isr_245 245
mk_isr do_isr_246 246
mk_isr do_isr_247 247
mk_isr do_isr_248 248
mk_isr do_isr_249 249
mk_isr do_isr_250 250
mk_isr do_isr_251 251
mk_isr do_isr_252 252
mk_isr do_isr_253 253
mk_isr do_isr_254 254
mk_isr do_isr_255 255