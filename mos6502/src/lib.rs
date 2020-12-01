#![no_std]

// based on https://github.com/fogleman/nes/blob/master/nes/cpu.go

const INTERRUPT_NONE: u8 = 1;
const INTERRUPT_NMI: u8 = 2;
const INTERRUPT_IRQ: u8 = 3;

// instructionModes indicates the addressing mode for each instruction
const INSTRUCTION_MODES: [u8; 256] = [
    6, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3, 6, 3, 2,
    2, 2, 2, 1, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3,
    6, 3, 2, 2, 2, 2, 6, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12,
    12, 6, 3, 6, 3, 2, 2, 2, 2, 6, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 8, 1, 1, 1, 10, 9, 6, 9,
    12, 12, 12, 12, 6, 3, 6, 3, 2, 2, 2, 2, 5, 7, 5, 7, 11, 11, 11, 11, 6, 5, 6, 5, 1, 1, 1, 1, 10,
    9, 6, 9, 12, 12, 13, 13, 6, 3, 6, 3, 2, 2, 3, 3, 5, 7, 5, 7, 11, 11, 11, 11, 6, 5, 6, 5, 1, 1,
    1, 1, 10, 9, 6, 9, 12, 12, 13, 13, 6, 3, 6, 3, 2, 2, 3, 3, 5, 7, 5, 7, 11, 11, 11, 11, 6, 5, 6,
    5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3, 6, 3, 2, 2, 2, 2, 5, 7, 5, 7, 11, 11, 11, 11,
    6, 5, 6, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3, 6, 3, 2, 2, 2, 2,
];

// instructionSizes indicates the size of each instruction in bytes
const INSTRUCTION_SIZES: [u8; 256] = [
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    3, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 0, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 0, 3, 0, 0,
    2, 2, 2, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
];

// instructionCycles indicates the number of cycles used by each instruction,
// not including conditional cycles
const INSTRUCTION_CYCLES: [u8; 256] = [
    7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, 2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

pub struct Cpu<'a> {
    memory: &'a dyn Memory, // memory interface
    cycles: u64,            // number of cycles
    pub pc: u16,            // program counter
    sp: u8,                 // stack pointer
    a: u8,                  // accumulator
    x: u8,                  // x register
    y: u8,                  // y register
    c: bool,                // carry flag
    z: bool,                // zero flag
    i: bool,                // interrupt disable flag
    d: bool,                // decimal mode flag
    b: bool,                // break command flag
    u: bool,                // unused flag
    v: bool,                // overflow flag
    n: bool,                // negative flag
    interrupt: u8,          // interrupt type to perform
    stall: u16,             // number of cycles to stall
}

#[derive(Debug)]
enum Mode {
    ModeAbsolute,
    ModeAbsoluteX,
    ModeAbsoluteY,
    ModeAccumulator,
    ModeImmediate,
    ModeImplied,
    ModeIndexedIndirect,
    ModeIndirect,
    ModeIndirectIndexed,
    ModeRelative,
    ModeZeroPage,
    ModeZeroPageX,
    ModeZeroPageY,
}

fn to_mode(value: u8) -> Mode {
    match value - 1 {
        0 => Mode::ModeAbsolute,
        1 => Mode::ModeAbsoluteX,
        2 => Mode::ModeAbsoluteY,
        3 => Mode::ModeAccumulator,
        4 => Mode::ModeImmediate,
        5 => Mode::ModeImplied,
        6 => Mode::ModeIndexedIndirect,
        7 => Mode::ModeIndirect,
        8 => Mode::ModeIndirectIndexed,
        9 => Mode::ModeRelative,
        10 => Mode::ModeZeroPage,
        11 => Mode::ModeZeroPageX,
        12 => Mode::ModeZeroPageY,
        _ => panic!("Unable to map mode {}", value),
    }
}

struct StepInfo {
    address: u16,
    pc: u16,
    mode: Mode,
}

pub trait Memory {
    fn get(&self, addr: u16) -> u8;

    fn set(&self, addr: u16, v: u8);
}

impl<'a> Cpu<'a> {
    pub fn new(memory: &'a dyn Memory) -> Self {
        Cpu {
            memory: memory,
            cycles: 0,
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            c: false,
            z: false,
            i: false,
            d: false,
            b: false,
            u: false,
            v: false,
            n: false,
            interrupt: 0,
            stall: 0,
        }
    }

    pub fn reset(&mut self) {
        self.pc = self.read16(0xfffc);
        self.sp = 0xfd;

        self.set_flags(0x24);
    }

    pub fn start_at(&mut self, addr: u16) {
        self.pc = addr;
        self.sp = 0xfd;

        self.set_flags(0x24);
    }

    fn read16(&self, addr: u16) -> u16 {
        let lo = self.read(addr);
        let hi = self.read(addr + 1);
        (hi as u16) << 8 as u16 | lo as u16
    }

    // read16bug emulates a 6502 bug that caused the low byte to wrap without
    // incrementing the high byte
    fn read16bug(&self, addr: u16) -> u16 {
        let a = addr;
        let b = (a & 0xff00) | ((((a & 0xff) as u8).overflowing_add(1)).0) as u16;
        let lo = self.read(a);
        let hi = self.read(b);
        (hi as u16) << 8 as u16 | lo as u16
    }

    fn read(&self, addr: u16) -> u8 {
        self.memory.get(addr)
    }

    // SetFlags sets the processor status flags
    fn set_flags(&mut self, flags: u8) {
        self.c = (flags >> 0) & 1 == 1;
        self.z = (flags >> 1) & 1 == 1;
        self.i = (flags >> 2) & 1 == 1;
        self.d = (flags >> 3) & 1 == 1;
        self.b = (flags >> 4) & 1 == 1;
        self.u = (flags >> 5) & 1 == 1;
        self.v = (flags >> 6) & 1 == 1;
        self.n = (flags >> 7) & 1 == 1;
    }

    // pagesDiffer returns true if the two addresses reference different pages
    fn pages_differ(a: u16, b: u16) -> bool {
        a & 0xff00 != b & 0xff00
    }

    // addBranchCycles adds a cycle for taking a branch and adds another cycle
    // if the branch jumps to a new page
    fn add_branch_cycles(&mut self, info: &StepInfo) {
        self.cycles += 1;
        if Self::pages_differ(info.pc, info.address) {
            self.cycles += 1;
        }
    }

    fn compare(&mut self, a: u8, b: u8) {
        let r = a.overflowing_sub(b).0;
        self.set_zn(r);
        if a >= b {
            self.c = true;
        } else {
            self.c = false;
        }
    }

    // setZ sets the zero flag if the argument is zero
    fn set_z(&mut self, value: u8) {
        if value == 0 {
            self.z = true;
        } else {
            self.z = false;
        }
    }

    // setN sets the negative flag if the argument is negative (high bit is set)
    fn set_n(&mut self, value: u8) {
        if value & 0x80 != 0 {
            self.n = true;
        } else {
            self.n = false;
        }
    }

    // setZN sets the zero flag and the negative flag
    fn set_zn(&mut self, value: u8) {
        self.set_z(value);
        self.set_n(value);
    }

    // push pushes a byte onto the stack
    fn push(&mut self, value: u8) {
        self.write(0x100 as u16 | self.sp as u16, value);
        let r = self.sp.overflowing_sub(1).0;
        self.sp = r;
    }

    // pull pops a byte from the stack
    fn pull(&mut self) -> u8 {
        let r = self.sp.overflowing_add(1).0;
        self.sp = r;
        self.read(0x100 as u16 | self.sp as u16)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.memory.set(addr, value);
    }

    // push16 pushes two bytes onto the stack
    fn push16(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xff) as u8;
        self.push(hi);
        self.push(lo);
    }

    // pull16 pops two bytes from the stack
    fn pull16(&mut self) -> u16 {
        let lo = self.pull();
        let hi = self.pull();
        (hi as u16) << 8 as u16 | lo as u16
    }

    // Flags returns the processor status flags
    fn flags(&self) -> u8 {
        let mut flags = 0u8;
        flags |= if self.c { 1 } else { 0 } << 0;
        flags |= if self.z { 1 } else { 0 } << 1;
        flags |= if self.i { 1 } else { 0 } << 2;
        flags |= if self.d { 1 } else { 0 } << 3;
        flags |= if self.b { 1 } else { 0 } << 4;
        flags |= if self.u { 1 } else { 0 } << 5;
        flags |= if self.v { 1 } else { 0 } << 6;
        flags |= if self.n { 1 } else { 0 } << 7;
        flags
    }

    // triggerNMI causes a non-maskable interrupt to occur on the next cycle
    pub fn trigger_nmi(&mut self) {
        self.interrupt = INTERRUPT_NMI;
    }

    // triggerIRQ causes an IRQ interrupt to occur on the next cycle
    pub fn trigger_irq(&mut self) {
        if !self.i {
            self.interrupt = INTERRUPT_IRQ;
        }
    }

    // Step executes a single CPU instruction
    pub fn step(&mut self) -> u64 {
        if self.stall > 0 {
            self.stall -= 1;
            return 1;
        }

        let cycles = self.cycles;

        match self.interrupt {
            INTERRUPT_NMI => self.cpu_nmi(),
            INTERRUPT_IRQ => self.cpu_irq(),
            _ => {}
        }
        self.interrupt = INTERRUPT_NONE;

        let opcode = self.read(self.pc);
        let mode = to_mode(INSTRUCTION_MODES[opcode as usize]);

        let address: u16;
        let mut page_crossed = false;

        match mode {
            Mode::ModeAbsolute => address = self.read16(self.pc + 1),
            Mode::ModeAbsoluteX => {
                address = self.read16(self.pc + 1) + self.x as u16;
                page_crossed = Cpu::pages_differ(address - self.x as u16, address)
            }
            Mode::ModeAbsoluteY => {
                address = self.read16(self.pc + 1) + self.y as u16;
                page_crossed = Cpu::pages_differ(address - self.y as u16, address)
            }
            Mode::ModeAccumulator => {
                address = 0;
            }
            Mode::ModeImmediate => {
                address = self.pc + 1;
            }
            Mode::ModeImplied => {
                address = 0;
            }
            Mode::ModeIndexedIndirect => {
                address = self.read16bug(self.read(self.pc + 1).overflowing_add(self.x).0 as u16);
            }
            Mode::ModeIndirect => {
                address = self.read16bug(self.read16(self.pc + 1));
            }
            Mode::ModeIndirectIndexed => {
                address = self.read16bug(self.read(self.pc + 1) as u16) + self.y as u16;
                page_crossed = Cpu::pages_differ(address - self.y as u16, address);
            }
            Mode::ModeRelative => {
                let offset = self.read(self.pc + 1) as u16;
                if offset < 0x80 {
                    address = self.pc.overflowing_add(2).0.overflowing_add(offset).0
                } else {
                    address = self
                        .pc
                        .overflowing_add(2)
                        .0
                        .overflowing_add(offset)
                        .0
                        .overflowing_sub(0x100)
                        .0
                }
            }
            Mode::ModeZeroPage => {
                address = self.read(self.pc + 1) as u16;
            }
            Mode::ModeZeroPageX => {
                address = (self.read(self.pc + 1) as u16 + self.x as u16) as u16 & 0xff
            }
            Mode::ModeZeroPageY => {
                address = (self.read(self.pc + 1) as u16 + self.y as u16) as u16 & 0xff
            }
        }

        self.pc += INSTRUCTION_SIZES[opcode as usize] as u16;
        self.cycles += INSTRUCTION_CYCLES[opcode as usize] as u64;
        if page_crossed {
            self.cycles += 1;
        }

        let info = StepInfo {
            address: address,
            pc: self.pc,
            mode: mode,
        };

        self.execute_opcode(opcode, &info);

        self.cycles - cycles
    }

    fn execute_opcode(&mut self, opcode: u8, info: &StepInfo) {
        match opcode {
            0 => self.brk(info),
            1 => self.ora(info),
            2 => self.kil(info),
            3 => self.slo(info),
            4 => self.nop(info),
            5 => self.ora(info),
            6 => self.asl(info),
            7 => self.slo(info),
            8 => self.php(info),
            9 => self.ora(info),
            10 => self.asl(info),
            11 => self.anc(info),
            12 => self.nop(info),
            13 => self.ora(info),
            14 => self.asl(info),
            15 => self.slo(info),
            16 => self.bpl(info),
            17 => self.ora(info),
            18 => self.kil(info),
            19 => self.slo(info),
            20 => self.nop(info),
            21 => self.ora(info),
            22 => self.asl(info),
            23 => self.slo(info),
            24 => self.clc(info),
            25 => self.ora(info),
            26 => self.nop(info),
            27 => self.slo(info),
            28 => self.nop(info),
            29 => self.ora(info),
            30 => self.asl(info),
            31 => self.slo(info),
            32 => self.jsr(info),
            33 => self.and(info),
            34 => self.kil(info),
            35 => self.rla(info),
            36 => self.bit(info),
            37 => self.and(info),
            38 => self.rol(info),
            39 => self.rla(info),
            40 => self.plp(info),
            41 => self.and(info),
            42 => self.rol(info),
            43 => self.anc(info),
            44 => self.bit(info),
            45 => self.and(info),
            46 => self.rol(info),
            47 => self.rla(info),
            48 => self.bmi(info),
            49 => self.and(info),
            50 => self.kil(info),
            51 => self.rla(info),
            52 => self.nop(info),
            53 => self.and(info),
            54 => self.rol(info),
            55 => self.rla(info),
            56 => self.sec(info),
            57 => self.and(info),
            58 => self.nop(info),
            59 => self.rla(info),
            60 => self.nop(info),
            61 => self.and(info),
            62 => self.rol(info),
            63 => self.rla(info),
            64 => self.rti(info),
            65 => self.eor(info),
            66 => self.kil(info),
            67 => self.sre(info),
            68 => self.nop(info),
            69 => self.eor(info),
            70 => self.lsr(info),
            71 => self.sre(info),
            72 => self.pha(info),
            73 => self.eor(info),
            74 => self.lsr(info),
            75 => self.alr(info),
            76 => self.jmp(info),
            77 => self.eor(info),
            78 => self.lsr(info),
            79 => self.sre(info),
            80 => self.bvc(info),
            81 => self.eor(info),
            82 => self.kil(info),
            83 => self.sre(info),
            84 => self.nop(info),
            85 => self.eor(info),
            86 => self.lsr(info),
            87 => self.sre(info),
            88 => self.cli(info),
            89 => self.eor(info),
            90 => self.nop(info),
            91 => self.sre(info),
            92 => self.nop(info),
            93 => self.eor(info),
            94 => self.lsr(info),
            95 => self.sre(info),
            96 => self.rts(info),
            97 => self.adc(info),
            98 => self.kil(info),
            99 => self.rra(info),
            100 => self.nop(info),
            101 => self.adc(info),
            102 => self.ror(info),
            103 => self.rra(info),
            104 => self.pla(info),
            105 => self.adc(info),
            106 => self.ror(info),
            107 => self.arr(info),
            108 => self.jmp(info),
            109 => self.adc(info),
            110 => self.ror(info),
            111 => self.rra(info),
            112 => self.bvs(info),
            113 => self.adc(info),
            114 => self.kil(info),
            115 => self.rra(info),
            116 => self.nop(info),
            117 => self.adc(info),
            118 => self.ror(info),
            119 => self.rra(info),
            120 => self.sei(info),
            121 => self.adc(info),
            122 => self.nop(info),
            123 => self.rra(info),
            124 => self.nop(info),
            125 => self.adc(info),
            126 => self.ror(info),
            127 => self.rra(info),
            128 => self.nop(info),
            129 => self.sta(info),
            130 => self.nop(info),
            131 => self.sax(info),
            132 => self.sty(info),
            133 => self.sta(info),
            134 => self.stx(info),
            135 => self.sax(info),
            136 => self.dey(info),
            137 => self.nop(info),
            138 => self.txa(info),
            139 => self.xaa(info),
            140 => self.sty(info),
            141 => self.sta(info),
            142 => self.stx(info),
            143 => self.sax(info),
            144 => self.bcc(info),
            145 => self.sta(info),
            146 => self.kil(info),
            147 => self.ahx(info),
            148 => self.sty(info),
            149 => self.sta(info),
            150 => self.stx(info),
            151 => self.sax(info),
            152 => self.tya(info),
            153 => self.sta(info),
            154 => self.txs(info),
            155 => self.tas(info),
            156 => self.shy(info),
            157 => self.sta(info),
            158 => self.shx(info),
            159 => self.ahx(info),
            160 => self.ldy(info),
            161 => self.lda(info),
            162 => self.ldx(info),
            163 => self.lax(info),
            164 => self.ldy(info),
            165 => self.lda(info),
            166 => self.ldx(info),
            167 => self.lax(info),
            168 => self.tay(info),
            169 => self.lda(info),
            170 => self.tax(info),
            171 => self.lax(info),
            172 => self.ldy(info),
            173 => self.lda(info),
            174 => self.ldx(info),
            175 => self.lax(info),
            176 => self.bcs(info),
            177 => self.lda(info),
            178 => self.kil(info),
            179 => self.lax(info),
            180 => self.ldy(info),
            181 => self.lda(info),
            182 => self.ldx(info),
            183 => self.lax(info),
            184 => self.clv(info),
            185 => self.lda(info),
            186 => self.tsx(info),
            187 => self.las(info),
            188 => self.ldy(info),
            189 => self.lda(info),
            190 => self.ldx(info),
            191 => self.lax(info),
            192 => self.cpy(info),
            193 => self.cmp(info),
            194 => self.nop(info),
            195 => self.dcp(info),
            196 => self.cpy(info),
            197 => self.cmp(info),
            198 => self.dec(info),
            199 => self.dcp(info),
            200 => self.iny(info),
            201 => self.cmp(info),
            202 => self.dex(info),
            203 => self.axs(info),
            204 => self.cpy(info),
            205 => self.cmp(info),
            206 => self.dec(info),
            207 => self.dcp(info),
            208 => self.bne(info),
            209 => self.cmp(info),
            210 => self.kil(info),
            211 => self.dcp(info),
            212 => self.nop(info),
            213 => self.cmp(info),
            214 => self.dec(info),
            215 => self.dcp(info),
            216 => self.cld(info),
            217 => self.cmp(info),
            218 => self.nop(info),
            219 => self.dcp(info),
            220 => self.nop(info),
            221 => self.cmp(info),
            222 => self.dec(info),
            223 => self.dcp(info),
            224 => self.cpx(info),
            225 => self.sbc(info),
            226 => self.nop(info),
            227 => self.isc(info),
            228 => self.cpx(info),
            229 => self.sbc(info),
            230 => self.inc(info),
            231 => self.isc(info),
            232 => self.inx(info),
            233 => self.sbc(info),
            234 => self.nop(info),
            235 => self.sbc(info),
            236 => self.cpx(info),
            237 => self.sbc(info),
            238 => self.inc(info),
            239 => self.isc(info),
            240 => self.beq(info),
            241 => self.sbc(info),
            242 => self.kil(info),
            243 => self.isc(info),
            244 => self.nop(info),
            245 => self.sbc(info),
            246 => self.inc(info),
            247 => self.isc(info),
            248 => self.sed(info),
            249 => self.sbc(info),
            250 => self.nop(info),
            251 => self.isc(info),
            252 => self.nop(info),
            253 => self.sbc(info),
            254 => self.inc(info),
            255 => self.isc(info),
        }
    }

    fn cpu_nmi(&mut self) {
        self.push16(self.pc);
        self.push(self.flags());
        self.pc = self.read16(0xfffa);
        self.i = true;
        self.cycles += 7;
    }

    fn cpu_irq(&mut self) {
        self.push16(self.pc);
        self.push(self.flags());
        self.pc = self.read16(0xfffe);
        self.i = true;
        self.cycles += 7;
    }

    // ADC - Add with Carry
    fn adc(&mut self, info: &StepInfo) {
        let a = self.a;
        let b = self.read(info.address);
        let c = if self.c { 1 } else { 0 };
        self.a = a.overflowing_add(b).0.overflowing_add(c).0;
        self.set_zn(self.a);
        if a as u32 + b as u32 + c as u32 > 0xff {
            self.c = true;
        } else {
            self.c = false;
        }
        if (a ^ b) & 0x80 == 0 && (a ^ self.a) & 0x80 != 0 {
            self.v = true;
        } else {
            self.v = false;
        }
    }

    // AND - Logical AND
    fn and(&mut self, info: &StepInfo) {
        self.a = self.a & self.read(info.address);
        self.set_zn(self.a);
    }

    // ASL - Arithmetic Shift Left
    fn asl(&mut self, info: &StepInfo) {
        if let Mode::ModeAccumulator = info.mode {
            self.c = (self.a >> 7) & 1 == 1;
            self.a <<= 1;
            self.set_zn(self.a);
        } else {
            let mut value = self.read(info.address);
            self.c = (value >> 7) & 1 == 1;
            value <<= 1;
            self.write(info.address, value);
            self.set_zn(value);
        }
    }

    // BCC - Branch if Carry Clear
    fn bcc(&mut self, info: &StepInfo) {
        if !self.c {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BCS - Branch if Carry Set
    fn bcs(&mut self, info: &StepInfo) {
        if self.c {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BEQ - Branch if Equal
    fn beq(&mut self, info: &StepInfo) {
        if self.z {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BIT - Bit Test
    fn bit(&mut self, info: &StepInfo) {
        let value = self.read(info.address);
        self.v = (value >> 6) & 1 == 1;
        self.set_z(value & self.a);
        self.set_n(value);
    }

    // BMI - Branch if Minus
    fn bmi(&mut self, info: &StepInfo) {
        if self.n {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BNE - Branch if Not Equal
    fn bne(&mut self, info: &StepInfo) {
        if !self.z {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BPL - Branch if Positive
    fn bpl(&mut self, info: &StepInfo) {
        if !self.n {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BRK - Force Interrupt
    fn brk(&mut self, info: &StepInfo) {
        self.push16(self.pc);
        self.php(info);
        self.sei(info);
        self.pc = self.read16(0xfffe);
    }

    // BVC - Branch if Overflow Clear
    fn bvc(&mut self, info: &StepInfo) {
        if !self.v {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // BVS - Branch if Overflow Set
    fn bvs(&mut self, info: &StepInfo) {
        if self.v {
            self.pc = info.address;
            self.add_branch_cycles(info);
        }
    }

    // CLC - Clear Carry Flag
    fn clc(&mut self, _info: &StepInfo) {
        self.c = false;
    }

    // CLD - Clear Decimal Mode
    fn cld(&mut self, _info: &StepInfo) {
        self.d = false;
    }

    // CLI - Clear Interrupt Disable
    fn cli(&mut self, _info: &StepInfo) {
        self.i = false;
    }

    // CLV - Clear Overflow Flag
    fn clv(&mut self, _info: &StepInfo) {
        self.v = false;
    }

    // CMP - Compare
    fn cmp(&mut self, info: &StepInfo) {
        let value = self.read(info.address);
        self.compare(self.a, value);
    }

    // CPX - Compare X Register
    fn cpx(&mut self, info: &StepInfo) {
        let value = self.read(info.address);
        self.compare(self.x, value);
    }

    // CPY - Compare Y Register
    fn cpy(&mut self, info: &StepInfo) {
        let value = self.read(info.address);
        self.compare(self.y, value);
    }

    // DEC - Decrement Memory
    fn dec(&mut self, info: &StepInfo) {
        let value = self.read(info.address).overflowing_sub(1).0;
        self.write(info.address, value);
        self.set_zn(value);
    }

    // DEX - Decrement X Register
    fn dex(&mut self, _info: &StepInfo) {
        self.x = self.x.overflowing_sub(1).0;
        self.set_zn(self.x);
    }

    // DEY - Decrement Y Register
    fn dey(&mut self, _info: &StepInfo) {
        let r = self.y.overflowing_sub(1).0;
        self.y = r;
        self.set_zn(self.y);
    }

    // EOR - Exclusive OR
    fn eor(&mut self, info: &StepInfo) {
        self.a = self.a ^ self.read(info.address);
        self.set_zn(self.a);
    }

    // INC - Increment Memory
    fn inc(&mut self, info: &StepInfo) {
        let value = self.read(info.address).overflowing_add(1).0;
        self.write(info.address, value);
        self.set_zn(value);
    }

    // INX - Increment X Register
    fn inx(&mut self, _info: &StepInfo) {
        self.x = self.x.overflowing_add(1).0;
        self.set_zn(self.x);
    }

    // INY - Increment Y Register
    fn iny(&mut self, _info: &StepInfo) {
        self.y = self.y.overflowing_add(1).0;
        self.set_zn(self.y);
    }

    // JMP - Jump
    fn jmp(&mut self, info: &StepInfo) {
        self.pc = info.address;
    }

    // JSR - Jump to Subroutine
    fn jsr(&mut self, info: &StepInfo) {
        self.push16(self.pc - 1);
        self.pc = info.address;
    }

    // LDA - Load Accumulator
    fn lda(&mut self, info: &StepInfo) {
        self.a = self.read(info.address);
        self.set_zn(self.a);
    }

    // LDX - Load X Register
    fn ldx(&mut self, info: &StepInfo) {
        self.x = self.read(info.address);
        self.set_zn(self.x);
    }

    // LDY - Load Y Register
    fn ldy(&mut self, info: &StepInfo) {
        self.y = self.read(info.address);
        self.set_zn(self.y);
    }

    // LSR - Logical Shift Right
    fn lsr(&mut self, info: &StepInfo) {
        if let Mode::ModeAccumulator = info.mode {
            self.c = self.a & 1 == 1;
            self.a >>= 1;
            self.set_zn(self.a);
        } else {
            let mut value = self.read(info.address);
            self.c = value & 1 == 1;
            value >>= 1;
            self.write(info.address, value);
            self.set_zn(value);
        }
    }

    // NOP - No Operation
    fn nop(&mut self, _info: &StepInfo) {}

    // ORA - Logical Inclusive OR
    fn ora(&mut self, info: &StepInfo) {
        self.a = self.a | self.read(info.address);
        self.set_zn(self.a);
    }

    // PHA - Push Accumulator
    fn pha(&mut self, _info: &StepInfo) {
        self.push(self.a);
    }

    // PHP - Push Processor Status
    fn php(&mut self, _info: &StepInfo) {
        self.push(self.flags() | 0x10);
    }

    // PLA - Pull Accumulator
    fn pla(&mut self, _info: &StepInfo) {
        self.a = self.pull();
        self.set_zn(self.a);
    }

    // PLP - Pull Processor Status
    fn plp(&mut self, _info: &StepInfo) {
        let pulled = self.pull();
        self.set_flags(pulled & 0xEF | 0x20);
    }

    // ROL - Rotate Left
    fn rol(&mut self, info: &StepInfo) {
        if let Mode::ModeAccumulator = info.mode {
            let c = self.c;
            self.c = (self.a >> 7) & 1 == 1;
            self.a = (self.a << 1) | if c { 1 } else { 0 };
            self.set_zn(self.a);
        } else {
            let c = self.c;
            let mut value = self.read(info.address);
            self.c = (value >> 7) & 1 == 1;
            value = (value << 1) | if c { 1 } else { 0 };
            self.write(info.address, value);
            self.set_zn(value);
        }
    }

    // ROR - Rotate Right
    fn ror(&mut self, info: &StepInfo) {
        if let Mode::ModeAccumulator = info.mode {
            let c = self.c;
            self.c = self.a & 1 == 1;
            self.a = (self.a >> 1) | (if c { 1 } else { 0 } << 7);
            self.set_zn(self.a);
        } else {
            let c = self.c;
            let mut value = self.read(info.address);
            self.c = value & 1 == 1;
            value = (value >> 1) | (if c { 1 } else { 0 } << 7);
            self.write(info.address, value);
            self.set_zn(value);
        }
    }

    // RTI - Return from Interrupt
    fn rti(&mut self, _info: &StepInfo) {
        let pulled = self.pull();
        self.set_flags(pulled & 0xef | 0x20);
        self.pc = self.pull16();
    }

    // RTS - Return from Subroutine
    fn rts(&mut self, _info: &StepInfo) {
        self.pc = self.pull16() + 1;
    }

    // SBC - Subtract with Carry
    fn sbc(&mut self, info: &StepInfo) {
        let a = self.a;
        let b = self.read(info.address);
        let c = self.c;
        self.a = a
            .overflowing_sub(b)
            .0
            .overflowing_sub(1 - if c { 1 } else { 0 })
            .0;
        self.set_zn(self.a);
        if a as i32 - b as i32 - (1 - if c { 1 } else { 0 }) as i32 >= 0 {
            self.c = true;
        } else {
            self.c = false;
        }
        if (a ^ b) & 0x80 != 0 && (a ^ self.a) & 0x80 != 0 {
            self.v = true;
        } else {
            self.v = false;
        }
    }

    // SEC - Set Carry Flag
    fn sec(&mut self, _info: &StepInfo) {
        self.c = true;
    }

    // SED - Set Decimal Flag
    fn sed(&mut self, _info: &StepInfo) {
        self.d = true;
    }

    // SEI - Set Interrupt Disable
    fn sei(&mut self, _info: &StepInfo) {
        self.i = true;
    }

    // STA - Store Accumulator
    fn sta(&mut self, info: &StepInfo) {
        self.write(info.address, self.a);
    }

    // STX - Store X Register
    fn stx(&mut self, info: &StepInfo) {
        self.write(info.address, self.x);
    }

    // STY - Store Y Register
    fn sty(&mut self, info: &StepInfo) {
        self.write(info.address, self.y);
    }

    // TAX - Transfer Accumulator to X
    fn tax(&mut self, _info: &StepInfo) {
        self.x = self.a;
        self.set_zn(self.x);
    }

    // TAY - Transfer Accumulator to Y
    fn tay(&mut self, _info: &StepInfo) {
        self.y = self.a;
        self.set_zn(self.y);
    }

    // TSX - Transfer Stack Pointer to X
    fn tsx(&mut self, _info: &StepInfo) {
        self.x = self.sp;
        self.set_zn(self.x);
    }

    // TXA - Transfer X to Accumulator
    fn txa(&mut self, _info: &StepInfo) {
        self.a = self.x;
        self.set_zn(self.a);
    }

    // TXS - Transfer X to Stack Pointer
    fn txs(&mut self, _info: &StepInfo) {
        self.sp = self.x;
    }

    // TYA - Transfer Y to Accumulator
    fn tya(&mut self, _info: &StepInfo) {
        self.a = self.y;
        self.set_zn(self.a);
    }

    // illegal opcodes below

    fn ahx(&mut self, _info: &StepInfo) {}

    fn alr(&mut self, _info: &StepInfo) {}

    fn anc(&mut self, _info: &StepInfo) {}

    fn arr(&mut self, _info: &StepInfo) {}

    fn axs(&mut self, _info: &StepInfo) {}

    fn dcp(&mut self, _info: &StepInfo) {}

    fn isc(&mut self, _info: &StepInfo) {}

    fn kil(&mut self, _info: &StepInfo) {}

    fn las(&mut self, _info: &StepInfo) {}

    fn lax(&mut self, _info: &StepInfo) {}

    fn rla(&mut self, _info: &StepInfo) {}

    fn rra(&mut self, _info: &StepInfo) {}

    fn sax(&mut self, _info: &StepInfo) {}

    fn shx(&mut self, _info: &StepInfo) {}

    fn shy(&mut self, _info: &StepInfo) {}

    fn slo(&mut self, _info: &StepInfo) {}

    fn sre(&mut self, _info: &StepInfo) {}

    fn tas(&mut self, _info: &StepInfo) {}

    fn xaa(&mut self, _info: &StepInfo) {}
}

#[cfg(test)]
mod tests {
    extern crate std;
    use core::cell::RefCell;
    use std::fs::File;
    use std::io::prelude::*;
    use std::string::String;

    use super::*;

    #[test]
    fn it_works() {
        struct Mem {
            ram: RefCell<[u8; 65536]>,
        }

        impl Memory for Mem {
            fn get(&self, addr: u16) -> u8 {
                self.ram.borrow()[addr as usize]
            }

            fn set(&self, addr: u16, v: u8) {
                self.ram.borrow_mut()[addr as usize] = v
            }
        }

        // see https://github.com/Klaus2m5/6502_65C02_functional_tests/blob/master/6502_functional_test.a65
        let mut f = File::open("data/6502_functional_test.hex").unwrap();
        let mut buffer = [0; 65536];
        f.read(&mut buffer).unwrap();

        let mut ram = [0u8; 65536];
        let mut index = 0usize;

        loop {
            if buffer[index] != ':' as u8 {
                panic!("Expected ':' but was {}", buffer[index] as char);
            }
            index += 1;

            let mut len_str = String::new();
            len_str.push(buffer[index] as char);
            len_str.push(buffer[index + 1] as char);
            index += 2;
            let len = u32::from_str_radix(&len_str, 16).unwrap();
            if len == 0 {
                break;
            }

            let mut addr_str = String::new();
            addr_str.push(buffer[index] as char);
            addr_str.push(buffer[index + 1] as char);
            addr_str.push(buffer[index + 2] as char);
            addr_str.push(buffer[index + 3] as char);
            index += 4;
            let mut addr = u32::from_str_radix(&addr_str, 16).unwrap() as usize;

            index += 2; // skip type

            for _ in 0..len {
                let mut byte_str = String::new();
                byte_str.push(buffer[index] as char);
                byte_str.push(buffer[index + 1] as char);
                index += 2;
                let byte = u8::from_str_radix(&byte_str, 16).unwrap();
                ram[addr] = byte;
                addr += 1usize;
            }

            index += 2; // skip checksum

            while buffer[index] == 10 || buffer[index] == 13 {
                index += 1;
            }
        }

        let mut mem = Mem {
            ram: RefCell::new(ram),
        };

        let mut cpu = Cpu::new(&mut mem);
        cpu.start_at(0x400);

        let mut max = 0u16;
        let mut last_good_pc = 0u16;
        let mut last_pc = 0u16;
        for _ in 0..82500 {
            cpu.step();

            if last_pc == cpu.pc {
                panic!(
                    "wrong? pc = {:x}  x = {:x}, y = {:x} Z={} C={} probably last good pc = {:x}",
                    cpu.pc, cpu.x, cpu.y, cpu.z, cpu.c, last_good_pc
                );
            }

            last_good_pc = last_pc;
            last_pc = cpu.pc;

            if cpu.pc > max {
                max = cpu.pc;
            }
        }

        assert!(max == 0x382e, "max pc should be reached");
    }
}
