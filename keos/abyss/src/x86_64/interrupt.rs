//! Interrupt.

use super::segmentation::SegmentSelector;
/// Type of exception
pub enum ExceptionType {
    Trap,
    Interrupt,
}

/// Exception vector enumeration of the x86_64.
///
/// See Intel 64 and IA-32 Architectures Software Developer’s Manual, Volume
/// 3A: System Programming Guide, Part 1, Chapter 6.15.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ExceptionVector {
    /// Interrupt 0 - Divide Error Exception (#DE)
    DivideError = 0,
    /// Interrupt 1 - Debug Exception (#DB)
    Debug = 1,
    /// Interrupt 2 - NonMaskableInterrupt Interrupt
    NonMaskableInterrupt = 2,
    /// Interrupt 3 - Breakpoint Exception (#BP)
    Breakpoint = 3,
    /// Interrupt 4 - Overflow Exception (#OF)
    OverflowException = 4,
    /// Interrupt 5 - Bound Range Exceeded Exception (#BR)
    BoundRangeExceeded = 5,
    /// Interrupt 6 - Invalid Opcode Exception (#UD)
    InvalidOpcode = 6,
    /// Interrupt 7 - Device Not Available Exception (#NM)
    DeviceNotAvailable = 7,
    /// Interrupt 8 - Double Fault Exception (#DF)
    DoubleFault = 8,
    /// Interrupt 9 - Coprocessor Segment Overrun
    CoprocessorSegmentOverrun = 9,
    /// Interrupt 10 - Invalid TSS Exception (#TS)
    InvalidTss = 10,
    /// Interrupt 11 - Segment Not Present (#NP)
    SegmentNotPresent = 11,
    /// Interrupt 12 - Stack Fault Exception (#SS)
    StackFault = 12,
    /// Interrupt 13 - General Protection Exception (#GP)
    GeneralProtection = 13,
    /// Interrupt 14 - Page-Fault Exception (#PF)
    PageFault = 14,
    /// Interrupt 15 - Reserved.
    _Reserved0 = 15,
    /// Interrupt 16 - x87 FPU Floating-Point Error (#MF)
    X87FpuFloatingPointError = 16,
    /// Interrupt 17 - Alignment Check Exception (#AC)
    AlignmentCheckException = 17,
    /// Interrupt 18 - Machine-Check Exception (#MC)
    MachineCheckException = 18,
    /// Interrupt 19 - SIMD Floating-Point Exception (#XM)
    SimdFloatingPointException = 19,
    /// Interrupt 20 - Virtualization Exception (#VE)
    VirtualizationException = 20,
    /// Interrupt 21 - Reserved1.
    _Reserved1 = 21,
    /// Interrupt 22 - Reserved2.
    _Reserved2 = 22,
    /// Interrupt 23 - Reserved3.
    _Reserved3 = 23,
    /// Interrupt 24 - Reserved4.
    _Reserved4 = 24,
    /// Interrupt 25 - Reserved5.
    _Reserved5 = 25,
    /// Interrupt 26 - Reserved6.
    _Reserved6 = 26,
    /// Interrupt 27 - Reserved7.
    _Reserved7 = 27,
    /// Interrupt 28 - Reserved8.
    _Reserved8 = 28,
    /// Interrupt 29 - Reserved9.
    _Reserved9 = 29,
    /// Interrupt 30 - Reserved10.
    _Reserved10 = 30,
    /// Interrupt 31 - Reserved11.
    _Reserved11 = 31,
    /// Interrupt 32 - User Defined Interrupt0
    UserDefined0 = 32,
    /// Interrupt 33 - User Defined Interrupt1
    UserDefined1 = 33,
    /// Interrupt 34 - User Defined Interrupt2
    UserDefined2 = 34,
    /// Interrupt 35 - User Defined Interrupt3
    UserDefined3 = 35,
    /// Interrupt 36 - User Defined Interrupt4
    UserDefined4 = 36,
    /// Interrupt 37 - User Defined Interrupt5
    UserDefined5 = 37,
    /// Interrupt 38 - User Defined Interrupt6
    UserDefined6 = 38,
    /// Interrupt 39 - User Defined Interrupt7
    UserDefined7 = 39,
    /// Interrupt 40 - User Defined Interrupt8
    UserDefined8 = 40,
    /// Interrupt 41 - User Defined Interrupt9
    UserDefined9 = 41,
    /// Interrupt 42 - User Defined Interrupt10
    UserDefined10 = 42,
    /// Interrupt 43 - User Defined Interrupt11
    UserDefined11 = 43,
    /// Interrupt 44 - User Defined Interrupt12
    UserDefined12 = 44,
    /// Interrupt 45 - User Defined Interrupt13
    UserDefined13 = 45,
    /// Interrupt 46 - User Defined Interrupt14
    UserDefined14 = 46,
    /// Interrupt 47 - User Defined Interrupt15
    UserDefined15 = 47,
    /// Interrupt 48 - User Defined Interrupt16
    UserDefined16 = 48,
    /// Interrupt 49 - User Defined Interrupt17
    UserDefined17 = 49,
    /// Interrupt 50 - User Defined Interrupt18
    UserDefined18 = 50,
    /// Interrupt 51 - User Defined Interrupt19
    UserDefined19 = 51,
    /// Interrupt 52 - User Defined Interrupt20
    UserDefined20 = 52,
    /// Interrupt 53 - User Defined Interrupt21
    UserDefined21 = 53,
    /// Interrupt 54 - User Defined Interrupt22
    UserDefined22 = 54,
    /// Interrupt 55 - User Defined Interrupt23
    UserDefined23 = 55,
    /// Interrupt 56 - User Defined Interrupt24
    UserDefined24 = 56,
    /// Interrupt 57 - User Defined Interrupt25
    UserDefined25 = 57,
    /// Interrupt 58 - User Defined Interrupt26
    UserDefined26 = 58,
    /// Interrupt 59 - User Defined Interrupt27
    UserDefined27 = 59,
    /// Interrupt 60 - User Defined Interrupt28
    UserDefined28 = 60,
    /// Interrupt 61 - User Defined Interrupt29
    UserDefined29 = 61,
    /// Interrupt 62 - User Defined Interrupt30
    UserDefined30 = 62,
    /// Interrupt 63 - User Defined Interrupt31
    UserDefined31 = 63,
    /// Interrupt 64 - User Defined Interrupt32
    UserDefined32 = 64,
    /// Interrupt 65 - User Defined Interrupt33
    UserDefined33 = 65,
    /// Interrupt 66 - User Defined Interrupt34
    UserDefined34 = 66,
    /// Interrupt 67 - User Defined Interrupt35
    UserDefined35 = 67,
    /// Interrupt 68 - User Defined Interrupt36
    UserDefined36 = 68,
    /// Interrupt 69 - User Defined Interrupt37
    UserDefined37 = 69,
    /// Interrupt 70 - User Defined Interrupt38
    UserDefined38 = 70,
    /// Interrupt 71 - User Defined Interrupt39
    UserDefined39 = 71,
    /// Interrupt 72 - User Defined Interrupt40
    UserDefined40 = 72,
    /// Interrupt 73 - User Defined Interrupt41
    UserDefined41 = 73,
    /// Interrupt 74 - User Defined Interrupt42
    UserDefined42 = 74,
    /// Interrupt 75 - User Defined Interrupt43
    UserDefined43 = 75,
    /// Interrupt 76 - User Defined Interrupt44
    UserDefined44 = 76,
    /// Interrupt 77 - User Defined Interrupt45
    UserDefined45 = 77,
    /// Interrupt 78 - User Defined Interrupt46
    UserDefined46 = 78,
    /// Interrupt 79 - User Defined Interrupt47
    UserDefined47 = 79,
    /// Interrupt 80 - User Defined Interrupt48
    UserDefined48 = 80,
    /// Interrupt 81 - User Defined Interrupt49
    UserDefined49 = 81,
    /// Interrupt 82 - User Defined Interrupt50
    UserDefined50 = 82,
    /// Interrupt 83 - User Defined Interrupt51
    UserDefined51 = 83,
    /// Interrupt 84 - User Defined Interrupt52
    UserDefined52 = 84,
    /// Interrupt 85 - User Defined Interrupt53
    UserDefined53 = 85,
    /// Interrupt 86 - User Defined Interrupt54
    UserDefined54 = 86,
    /// Interrupt 87 - User Defined Interrupt55
    UserDefined55 = 87,
    /// Interrupt 88 - User Defined Interrupt56
    UserDefined56 = 88,
    /// Interrupt 89 - User Defined Interrupt57
    UserDefined57 = 89,
    /// Interrupt 90 - User Defined Interrupt58
    UserDefined58 = 90,
    /// Interrupt 91 - User Defined Interrupt59
    UserDefined59 = 91,
    /// Interrupt 92 - User Defined Interrupt60
    UserDefined60 = 92,
    /// Interrupt 93 - User Defined Interrupt61
    UserDefined61 = 93,
    /// Interrupt 94 - User Defined Interrupt62
    UserDefined62 = 94,
    /// Interrupt 95 - User Defined Interrupt63
    UserDefined63 = 95,
    /// Interrupt 96 - User Defined Interrupt64
    UserDefined64 = 96,
    /// Interrupt 97 - User Defined Interrupt65
    UserDefined65 = 97,
    /// Interrupt 98 - User Defined Interrupt66
    UserDefined66 = 98,
    /// Interrupt 99 - User Defined Interrupt67
    UserDefined67 = 99,
    /// Interrupt 100 - User Defined Interrupt68
    UserDefined68 = 100,
    /// Interrupt 101 - User Defined Interrupt69
    UserDefined69 = 101,
    /// Interrupt 102 - User Defined Interrupt70
    UserDefined70 = 102,
    /// Interrupt 103 - User Defined Interrupt71
    UserDefined71 = 103,
    /// Interrupt 104 - User Defined Interrupt72
    UserDefined72 = 104,
    /// Interrupt 105 - User Defined Interrupt73
    UserDefined73 = 105,
    /// Interrupt 106 - User Defined Interrupt74
    UserDefined74 = 106,
    /// Interrupt 107 - User Defined Interrupt75
    UserDefined75 = 107,
    /// Interrupt 108 - User Defined Interrupt76
    UserDefined76 = 108,
    /// Interrupt 109 - User Defined Interrupt77
    UserDefined77 = 109,
    /// Interrupt 110 - User Defined Interrupt78
    UserDefined78 = 110,
    /// Interrupt 111 - User Defined Interrupt79
    UserDefined79 = 111,
    /// Interrupt 112 - User Defined Interrupt80
    UserDefined80 = 112,
    /// Interrupt 113 - User Defined Interrupt81
    UserDefined81 = 113,
    /// Interrupt 114 - User Defined Interrupt82
    UserDefined82 = 114,
    /// Interrupt 115 - User Defined Interrupt83
    UserDefined83 = 115,
    /// Interrupt 116 - User Defined Interrupt84
    UserDefined84 = 116,
    /// Interrupt 117 - User Defined Interrupt85
    UserDefined85 = 117,
    /// Interrupt 118 - User Defined Interrupt86
    UserDefined86 = 118,
    /// Interrupt 119 - User Defined Interrupt87
    UserDefined87 = 119,
    /// Interrupt 120 - User Defined Interrupt88
    UserDefined88 = 120,
    /// Interrupt 121 - User Defined Interrupt89
    UserDefined89 = 121,
    /// Interrupt 122 - User Defined Interrupt90
    UserDefined90 = 122,
    /// Interrupt 123 - User Defined Interrupt91
    UserDefined91 = 123,
    /// Interrupt 124 - User Defined Interrupt92
    UserDefined92 = 124,
    /// Interrupt 125 - User Defined Interrupt93
    UserDefined93 = 125,
    /// Interrupt 126 - User Defined Interrupt94
    UserDefined94 = 126,
    /// Interrupt 127 - User Defined Interrupt95
    UserDefined95 = 127,
    /// Interrupt 128 - User Defined Interrupt96
    UserDefined96 = 128,
    /// Interrupt 129 - User Defined Interrupt97
    UserDefined97 = 129,
    /// Interrupt 130 - User Defined Interrupt98
    UserDefined98 = 130,
    /// Interrupt 131 - User Defined Interrupt99
    UserDefined99 = 131,
    /// Interrupt 132 - User Defined Interrupt100
    UserDefined100 = 132,
    /// Interrupt 133 - User Defined Interrupt101
    UserDefined101 = 133,
    /// Interrupt 134 - User Defined Interrupt102
    UserDefined102 = 134,
    /// Interrupt 135 - User Defined Interrupt103
    UserDefined103 = 135,
    /// Interrupt 136 - User Defined Interrupt104
    UserDefined104 = 136,
    /// Interrupt 137 - User Defined Interrupt105
    UserDefined105 = 137,
    /// Interrupt 138 - User Defined Interrupt106
    UserDefined106 = 138,
    /// Interrupt 139 - User Defined Interrupt107
    UserDefined107 = 139,
    /// Interrupt 140 - User Defined Interrupt108
    UserDefined108 = 140,
    /// Interrupt 141 - User Defined Interrupt109
    UserDefined109 = 141,
    /// Interrupt 142 - User Defined Interrupt110
    UserDefined110 = 142,
    /// Interrupt 143 - User Defined Interrupt111
    UserDefined111 = 143,
    /// Interrupt 144 - User Defined Interrupt112
    UserDefined112 = 144,
    /// Interrupt 145 - User Defined Interrupt113
    UserDefined113 = 145,
    /// Interrupt 146 - User Defined Interrupt114
    UserDefined114 = 146,
    /// Interrupt 147 - User Defined Interrupt115
    UserDefined115 = 147,
    /// Interrupt 148 - User Defined Interrupt116
    UserDefined116 = 148,
    /// Interrupt 149 - User Defined Interrupt117
    UserDefined117 = 149,
    /// Interrupt 150 - User Defined Interrupt118
    UserDefined118 = 150,
    /// Interrupt 151 - User Defined Interrupt119
    UserDefined119 = 151,
    /// Interrupt 152 - User Defined Interrupt120
    UserDefined120 = 152,
    /// Interrupt 153 - User Defined Interrupt121
    UserDefined121 = 153,
    /// Interrupt 154 - User Defined Interrupt122
    UserDefined122 = 154,
    /// Interrupt 155 - User Defined Interrupt123
    UserDefined123 = 155,
    /// Interrupt 156 - User Defined Interrupt124
    UserDefined124 = 156,
    /// Interrupt 157 - User Defined Interrupt125
    UserDefined125 = 157,
    /// Interrupt 158 - User Defined Interrupt126
    UserDefined126 = 158,
    /// Interrupt 159 - User Defined Interrupt127
    UserDefined127 = 159,
    /// Interrupt 160 - User Defined Interrupt128
    UserDefined128 = 160,
    /// Interrupt 161 - User Defined Interrupt129
    UserDefined129 = 161,
    /// Interrupt 162 - User Defined Interrupt130
    UserDefined130 = 162,
    /// Interrupt 163 - User Defined Interrupt131
    UserDefined131 = 163,
    /// Interrupt 164 - User Defined Interrupt132
    UserDefined132 = 164,
    /// Interrupt 165 - User Defined Interrupt133
    UserDefined133 = 165,
    /// Interrupt 166 - User Defined Interrupt134
    UserDefined134 = 166,
    /// Interrupt 167 - User Defined Interrupt135
    UserDefined135 = 167,
    /// Interrupt 168 - User Defined Interrupt136
    UserDefined136 = 168,
    /// Interrupt 169 - User Defined Interrupt137
    UserDefined137 = 169,
    /// Interrupt 170 - User Defined Interrupt138
    UserDefined138 = 170,
    /// Interrupt 171 - User Defined Interrupt139
    UserDefined139 = 171,
    /// Interrupt 172 - User Defined Interrupt140
    UserDefined140 = 172,
    /// Interrupt 173 - User Defined Interrupt141
    UserDefined141 = 173,
    /// Interrupt 174 - User Defined Interrupt142
    UserDefined142 = 174,
    /// Interrupt 175 - User Defined Interrupt143
    UserDefined143 = 175,
    /// Interrupt 176 - User Defined Interrupt144
    UserDefined144 = 176,
    /// Interrupt 177 - User Defined Interrupt145
    UserDefined145 = 177,
    /// Interrupt 178 - User Defined Interrupt146
    UserDefined146 = 178,
    /// Interrupt 179 - User Defined Interrupt147
    UserDefined147 = 179,
    /// Interrupt 180 - User Defined Interrupt148
    UserDefined148 = 180,
    /// Interrupt 181 - User Defined Interrupt149
    UserDefined149 = 181,
    /// Interrupt 182 - User Defined Interrupt150
    UserDefined150 = 182,
    /// Interrupt 183 - User Defined Interrupt151
    UserDefined151 = 183,
    /// Interrupt 184 - User Defined Interrupt152
    UserDefined152 = 184,
    /// Interrupt 185 - User Defined Interrupt153
    UserDefined153 = 185,
    /// Interrupt 186 - User Defined Interrupt154
    UserDefined154 = 186,
    /// Interrupt 187 - User Defined Interrupt155
    UserDefined155 = 187,
    /// Interrupt 188 - User Defined Interrupt156
    UserDefined156 = 188,
    /// Interrupt 189 - User Defined Interrupt157
    UserDefined157 = 189,
    /// Interrupt 190 - User Defined Interrupt158
    UserDefined158 = 190,
    /// Interrupt 191 - User Defined Interrupt159
    UserDefined159 = 191,
    /// Interrupt 192 - User Defined Interrupt160
    UserDefined160 = 192,
    /// Interrupt 193 - User Defined Interrupt161
    UserDefined161 = 193,
    /// Interrupt 194 - User Defined Interrupt162
    UserDefined162 = 194,
    /// Interrupt 195 - User Defined Interrupt163
    UserDefined163 = 195,
    /// Interrupt 196 - User Defined Interrupt164
    UserDefined164 = 196,
    /// Interrupt 197 - User Defined Interrupt165
    UserDefined165 = 197,
    /// Interrupt 198 - User Defined Interrupt166
    UserDefined166 = 198,
    /// Interrupt 199 - User Defined Interrupt167
    UserDefined167 = 199,
    /// Interrupt 200 - User Defined Interrupt168
    UserDefined168 = 200,
    /// Interrupt 201 - User Defined Interrupt169
    UserDefined169 = 201,
    /// Interrupt 202 - User Defined Interrupt170
    UserDefined170 = 202,
    /// Interrupt 203 - User Defined Interrupt171
    UserDefined171 = 203,
    /// Interrupt 204 - User Defined Interrupt172
    UserDefined172 = 204,
    /// Interrupt 205 - User Defined Interrupt173
    UserDefined173 = 205,
    /// Interrupt 206 - User Defined Interrupt174
    UserDefined174 = 206,
    /// Interrupt 207 - User Defined Interrupt175
    UserDefined175 = 207,
    /// Interrupt 208 - User Defined Interrupt176
    UserDefined176 = 208,
    /// Interrupt 209 - User Defined Interrupt177
    UserDefined177 = 209,
    /// Interrupt 210 - User Defined Interrupt178
    UserDefined178 = 210,
    /// Interrupt 211 - User Defined Interrupt179
    UserDefined179 = 211,
    /// Interrupt 212 - User Defined Interrupt180
    UserDefined180 = 212,
    /// Interrupt 213 - User Defined Interrupt181
    UserDefined181 = 213,
    /// Interrupt 214 - User Defined Interrupt182
    UserDefined182 = 214,
    /// Interrupt 215 - User Defined Interrupt183
    UserDefined183 = 215,
    /// Interrupt 216 - User Defined Interrupt184
    UserDefined184 = 216,
    /// Interrupt 217 - User Defined Interrupt185
    UserDefined185 = 217,
    /// Interrupt 218 - User Defined Interrupt186
    UserDefined186 = 218,
    /// Interrupt 219 - User Defined Interrupt187
    UserDefined187 = 219,
    /// Interrupt 220 - User Defined Interrupt188
    UserDefined188 = 220,
    /// Interrupt 221 - User Defined Interrupt189
    UserDefined189 = 221,
    /// Interrupt 222 - User Defined Interrupt190
    UserDefined190 = 222,
    /// Interrupt 223 - User Defined Interrupt191
    UserDefined191 = 223,
    /// Interrupt 224 - User Defined Interrupt192
    UserDefined192 = 224,
    /// Interrupt 225 - User Defined Interrupt193
    UserDefined193 = 225,
    /// Interrupt 226 - User Defined Interrupt194
    UserDefined194 = 226,
    /// Interrupt 227 - User Defined Interrupt195
    UserDefined195 = 227,
    /// Interrupt 228 - User Defined Interrupt196
    UserDefined196 = 228,
    /// Interrupt 229 - User Defined Interrupt197
    UserDefined197 = 229,
    /// Interrupt 230 - User Defined Interrupt198
    UserDefined198 = 230,
    /// Interrupt 231 - User Defined Interrupt199
    UserDefined199 = 231,
    /// Interrupt 232 - User Defined Interrupt200
    UserDefined200 = 232,
    /// Interrupt 233 - User Defined Interrupt201
    UserDefined201 = 233,
    /// Interrupt 234 - User Defined Interrupt202
    UserDefined202 = 234,
    /// Interrupt 235 - User Defined Interrupt203
    UserDefined203 = 235,
    /// Interrupt 236 - User Defined Interrupt204
    UserDefined204 = 236,
    /// Interrupt 237 - User Defined Interrupt205
    UserDefined205 = 237,
    /// Interrupt 238 - User Defined Interrupt206
    UserDefined206 = 238,
    /// Interrupt 239 - User Defined Interrupt207
    UserDefined207 = 239,
    /// Interrupt 240 - User Defined Interrupt208
    UserDefined208 = 240,
    /// Interrupt 241 - User Defined Interrupt209
    UserDefined209 = 241,
    /// Interrupt 242 - User Defined Interrupt210
    UserDefined210 = 242,
    /// Interrupt 243 - User Defined Interrupt211
    UserDefined211 = 243,
    /// Interrupt 244 - User Defined Interrupt212
    UserDefined212 = 244,
    /// Interrupt 245 - User Defined Interrupt213
    UserDefined213 = 245,
    /// Interrupt 246 - User Defined Interrupt214
    UserDefined214 = 246,
    /// Interrupt 247 - User Defined Interrupt215
    UserDefined215 = 247,
    /// Interrupt 248 - User Defined Interrupt216
    UserDefined216 = 248,
    /// Interrupt 249 - User Defined Interrupt217
    UserDefined217 = 249,
    /// Interrupt 250 - User Defined Interrupt218
    UserDefined218 = 250,
    /// Interrupt 251 - User Defined Interrupt219
    UserDefined219 = 251,
    /// Interrupt 252 - User Defined Interrupt220
    UserDefined220 = 252,
    /// Interrupt 253 - User Defined Interrupt221
    UserDefined221 = 253,
    /// Interrupt 254 - User Defined Interrupt222
    UserDefined222 = 254,
    /// Interrupt 255 - User Defined Interrupt223
    UserDefined223 = 255,
}

impl ExceptionVector {
    /// Create a new ExceptionVector from usize.
    #[inline(always)]
    pub fn new(x: usize) -> Option<Self> {
        x.try_into()
            .ok()
            .map(|n: u8| unsafe { core::mem::transmute(n) })
    }

    /// Cast the ExceptionVector into usize.
    #[inline(always)]
    pub const fn into_raw(self) -> usize {
        self as usize
    }
}

/// Stack frame on interrupt.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub rip: usize,
    pub cs: SegmentSelector,
    #[doc(hidden)]
    pub __pad0: u16,
    #[doc(hidden)]
    pub __pad1: u32,
    pub rflags: crate::x86_64::Rflags,
    pub rsp: usize,
    pub ss: SegmentSelector,
    #[doc(hidden)]
    pub __pad2: u16,
    #[doc(hidden)]
    pub __pad3: u32,
}

impl core::fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        #[repr(transparent)]
        pub struct HexFormatter(usize);

        impl core::fmt::Debug for HexFormatter {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                write!(f, "{:x}", self.0)
            }
        }

        f.debug_struct("InterruptStackFrame")
            .field("rip", &HexFormatter(self.rip))
            .field("cs", &self.cs)
            .field("rflags", &self.rflags)
            .field("rsp", &HexFormatter(self.rsp))
            .field("ss", &self.ss)
            .finish()
    }
}

/// X86_64's IDT table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InterruptGateDescriptor<F> {
    lo: u16,
    selector: u16,
    options: u16,
    mid: u16,
    hi: u32,
    _rev: u32,
    _ty: core::marker::PhantomData<F>,
}

impl<F> InterruptGateDescriptor<F> {
    /// Empty this gate entry.
    #[inline(always)]
    pub const fn empty() -> Self {
        InterruptGateDescriptor {
            lo: 0,
            selector: 0,
            options: 0,
            mid: 0,
            hi: 0,
            _rev: 0,
            _ty: core::marker::PhantomData,
        }
    }
}

macro_rules! define_interrupt_handler {
    ($(#[$attr:meta])* $handler:ident = $alias:ty; $($t:tt)*) => {
        $(#[$attr])*
        pub type $handler = $alias;
        impl InterruptGateDescriptor<$handler> {
            /// Set this gate entry.
            #[inline(always)]
            pub fn set(&mut self, ss: SegmentSelector, ty_: ExceptionType, f: $handler) {
                *self = InterruptGateDescriptor {
                    lo: f as usize as u16,
                    selector: (ss.index() << 3) as u16,
                    options: ((1 << 15)
                        | (ss.dpl() as u16) << 13
                        | if matches!(ty_, ExceptionType::Trap) {
                            0xf << 8
                        } else {
                            0xe << 8 // Interrupt.
                        }),
                        mid: ((f as usize as u64) >> 16) as u16,
                        hi: ((f as usize as u64) >> 32) as u32,
                        _rev: 0,
                        _ty: core::marker::PhantomData,
                };
            }
        }

        define_interrupt_handler!($($t)*);
    };
    () => {}
}

bitflags::bitflags! {
    /// List of error codes on page fault.
    #[repr(transparent)]
    pub struct PFErrorCode: u64 {
        /// When set, the page fault was caused by a page-protection violation. When not set, it was caused by a non-present page.
        const PRESENT = 1 << 0;
        /// When set, the page fault was caused by a write access. When not set, it was caused by a read access.
        const WRITE_ACCESS = 1 << 1;
        /// When set, the page fault was caused while CPL = 3. This does not necessarily mean that the page fault was a privilege violation.
        const USER = 1 << 2;
        /// When set, one or more page directory entries contain reserved bits which are set to 1. This only applies when the PSE or PAE flags in CR4 are set to 1.
        const RESERVED_WRITE = 1 << 3;
        /// When set, the page fault was caused by an instruction fetch. This only applies when the No-Execute bit is supported and enabled.
        const INSTRCUTION_FETCH = 1 << 4;
    }
}

/// Must be zero.
#[repr(transparent)]
pub struct MustbeZero(u64);

define_interrupt_handler!(
    /// Normal handler without error code.
    Handler = unsafe extern "x86-interrupt" fn(&mut InterruptStackFrame);
    /// Handler that push SegmentSelector as Error code.
    /// General Protection Fault.
    /// Invalid TSS Exception.
    /// Segment Not Present Exception Handler.
    /// Stack Fault Exception Handler.
    HandlerWithSegmentSelectorErrorCode =
        unsafe extern "x86-interrupt" fn(&mut InterruptStackFrame, SegmentSelector);
    /// Page-Fault Handler.
    HandlerPageFault = unsafe extern "x86-interrupt" fn(&mut InterruptStackFrame, PFErrorCode);
    /// Align Check Exception Handler.
    HandlerAlignCheck = unsafe extern "x86-interrupt" fn(&mut InterruptStackFrame, MustbeZero);
    /// Double fault Exception. Abort.
    AbortDoubleFault = unsafe extern "x86-interrupt" fn(&mut InterruptStackFrame, MustbeZero) -> !;
    /// Machine Check Exception. Abort.
    AbortMachineCheck = unsafe extern "x86-interrupt" fn(&mut InterruptStackFrame) -> !;
);

use crate::x86_64::table::InterruptDescriptorTable;
/// The unique kernel IDT.
pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::empty();
