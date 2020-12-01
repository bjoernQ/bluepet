// base is 0xe800
const PIA1: u16 = 0x0010;
const PIA2: u16 = 0x0020;
const PORTA: u16 = 0x00;
const CRA: u16 = 0x01;
const PORTB: u16 = 0x02;
const CRB: u16 = 0x03;

const VIA: u16 = 0x0040;
const VPORTB: u16 = 0x00;
const VPORTA: u16 = 0x01;
const DDRB: u16 = 0x02;
const DDRA: u16 = 0x03;
const T1LO: u16 = 0x04;
const T1HI: u16 = 0x05;
const T1LLO: u16 = 0x06;
const T1LHI: u16 = 0x07;
const T2LO: u16 = 0x08;
const T2HI: u16 = 0x09;
const SHIFT: u16 = 0x0a;
const ACR: u16 = 0x0b;
const PCR: u16 = 0x0c;
const IFR: u16 = 0x0d;
const IER: u16 = 0x0e;

const IER_MASTER: u8 = 0x80;
const IER_TIMER1: u8 = 0x40;
const IER_TIMER2: u8 = 0x20;
const IER_CB1_ACTIVE: u8 = 0x10;
const IER_CB2_ACTIVE: u8 = 0x08;
const IER_SHIFT_REG: u8 = 0x04;
const IER_CA1_ACTIVE: u8 = 0x02;
const IER_CA2_ACTIVE: u8 = 0x01;

const VIA_VIDEO_RETRACE: u8 = 0x20;
const VIA_ACR_SHIFT_MASK: u8 = 0x1c;
const VIA_ACR_T1_CONTINUOUS: u8 = 0x40;

// 1ms internal clock tick
const TICK_FREQ: u16 = 1000;

// 50Hz system interrupt frequency
const SYS_TICKS: u16 = 20;

const PIA1_PORTB: u16 = PIA1 + PORTB;
const PIA1_PORTA: u16 = PIA1 + PORTA;
const PIA1_CRA: u16 = PIA1 + CRA;
const PIA1_CRB: u16 = PIA1 + CRB;

const PIA2_PORTB: u16 = PIA2 + PORTB;
const PIA2_PORTA: u16 = PIA2 + PORTA;
const PIA2_CRA: u16 = PIA2 + CRA;
const PIA2_CRB: u16 = PIA2 + CRB;

const VIA_VPORTA: u16 = VIA + VPORTA;
const VIA_VPORTB: u16 = VIA + VPORTB;
const VIA_SHIFT: u16 = VIA + SHIFT;
const VIA_ACR: u16 = VIA + ACR;
const VIA_IER: u16 = VIA + IER;
const VIA_IFR: u16 = VIA + IFR;
const VIA_T1LO: u16 = VIA + T1LO;
const VIA_T1HI: u16 = VIA + T1HI;
const VIA_T1LLO: u16 = VIA + T1LLO;
const VIA_T1LHI: u16 = VIA + T1LHI;
const VIA_T2LO: u16 = VIA + T2LO;
const VIA_T2HI: u16 = VIA + T2HI;
const VIA_PCR: u16 = VIA + PCR;
const VIA_DDRA: u16 = VIA + DDRA;
const VIA_DDRB: u16 = VIA + DDRB;

const VIA_ANH: u16 = 0x4f;

#[derive(Debug)]
pub struct Io<'a> {
    ticks: u16,
    timer1: bool,
    timer2: bool,
    t1: u16,
    t2: u16,
    t1_latch: u16,
    acr: u8,
    ier: u8,
    ifr: u8,
    ddra: u8,
    ddrb: u8,
    porta: u8,
    portb: u8,

    pia1_pa_in: u8,
    pia1_pa_out: u8,
    pia1_ddra: u8,
    pia1_cra: u8,
    pia1_pb_in: u8,
    pia1_pb_out: u8,
    pia1_ddrb: u8,
    pia1_crb: u8,
    pia1_ca2: u8,

    pia2_pa_in: u8,
    pia2_pa_out: u8,
    pia2_ddra: u8,
    pia2_cra: u8,
    pia2_pb_in: u8,
    pia2_pb_out: u8,
    pia2_ddrb: u8,
    pia2_crb: u8,

    via_drb_in: u8,
    via_drb_out: u8,
    via_dra_in: u8,
    via_dra_out: u8,

    via_t1cl: u8,
    via_t1ch: u8,
    via_t1_1shot: u8,
    via_t1ll: u8,
    via_t1lh: u8,
    via_t2cl: u8,
    via_t2ch: u8,
    via_t2_1shot: u8,
    via_sr: u8,
    via_pcr: u8,

    pub keyboard: Keyboard,
    pub ieee: Ieee<'a>,
}

impl<'a> Io<'a> {
    pub fn new(keyboard: Keyboard, storage: &'a mut dyn Storage) -> Io {
        Io {
            ticks: 0,
            timer1: false,
            timer2: false,
            t1: 0,
            t2: 0,
            t1_latch: 0,
            acr: 0,
            ier: 0x80,
            ifr: 0,
            ddra: 0,
            ddrb: 0,
            porta: 0,
            portb: 0,

            pia1_pa_in: 0xf0,
            pia1_pa_out: 0,
            pia1_ddra: 0,
            pia1_cra: 0,
            pia1_pb_in: 0xff,
            pia1_pb_out: 0,
            pia1_ddrb: 0,
            pia1_crb: 0,
            pia1_ca2: 0,

            pia2_pa_in: 0,
            pia2_pa_out: 0,
            pia2_ddra: 0,
            pia2_cra: 0,
            pia2_pb_in: 0,
            pia2_pb_out: 0,
            pia2_ddrb: 0,
            pia2_crb: 0,

            via_drb_in: 0,
            via_drb_out: 0,
            via_dra_in: 0,
            via_dra_out: 0,
            via_t1cl: 0xff,
            via_t1ch: 0xff,
            via_t1_1shot: 0,
            via_t1ll: 0xff,
            via_t1lh: 0xff,
            via_t2cl: 0xff,
            via_t2ch: 0xff,
            via_t2_1shot: 0,
            via_sr: 0,
            via_pcr: 0,

            keyboard: keyboard,

            ieee: Ieee::new(storage),
        }
    }

    pub fn reset(&mut self) {
        self.acr = 0x00;
        self.ifr = 0x00;
        self.ier = 0x00;

        self.t1 = 0;
        self.t2 = 0;

        self.timer1 = false;
        self.timer2 = false;

        self.keyboard.reset();
    }

    // true -> trigger IRQ, false otherwise
    pub fn tick(&mut self) -> bool {
        let mut raise_irq = false;

        if self.ticks == SYS_TICKS {
            self.ticks = 0;
            self.portb |= VIA_VIDEO_RETRACE;
            raise_irq = true;
        } else {
            self.portb = self.portb & !VIA_VIDEO_RETRACE;
            self.ticks += 1;
        }

        if self.timer1 {
            if self.t1 < TICK_FREQ {
                self.t1 = 0;
                self.timer1 = false;
                self.ifr |= IER_TIMER1;

                if (self.ier & IER_MASTER) != 0 && (self.ier & IER_TIMER1) != 0 {
                    raise_irq = true;
                }
            } else {
                self.t1 -= TICK_FREQ;
            }
        }

        if self.timer2 {
            if self.t2 < TICK_FREQ {
                self.t2 = 0;
                self.timer2 = false;
                self.ifr |= IER_TIMER2;

                if (self.ier & IER_MASTER) != 0 && (self.ier & IER_TIMER2) != 0 {
                    raise_irq = true;
                }
            } else {
                self.t2 -= TICK_FREQ;
            }
        }

        if (self.pia1_cra & 0x81) == 0x81
            || (self.pia1_cra & 0x48) == 0x48
            || (self.pia1_crb & 0x81) == 0x81
            || (self.pia1_crb & 0x48) == 0x48
        {
            raise_irq = true;
        }

        if (self.ifr & self.ier & 0x7f) != 0 {
            self.ifr |= 0x80;
            raise_irq = true;
        } else {
            self.ifr &= !0x80;
        }

        raise_irq
    }

    pub fn read(&mut self, offset: u16) -> u8 {
        let mut r = 0x00u8;
        match offset {
            PIA1_PORTA => {
                if (self.pia1_cra & 0x04) != 0 {
                    /* Clear IRQs in CRA as side-effect of reading PA. */
                    if (self.pia1_cra & 0xC0) != 0 {
                        self.pia1_cra &= 0x3F;
                    }
                    if (self.pia1_ddra & 0x40) == 0 {
                        if self.ieee.eoi_in() {
                            self.pia1_pa_in |= 0x40;
                        } else {
                            self.pia1_pa_in &= 0xbf;
                        }
                    }

                    r = (self.pia1_pa_in & !self.pia1_ddra) | (self.pia1_pa_out & self.pia1_ddra);
                } else {
                    r = 0x80 + self.keyboard.row();
                }
            }
            PIA1_CRA => {
                r = self.pia1_cra;
            }
            PIA1_PORTB => {
                if (self.pia1_crb & 0x04) != 0 {
                    /* Clear IRQs in CRB as side-effect of reading PB. */
                    if (self.pia1_crb & 0xC0) != 0 {
                        self.pia1_crb &= 0x3F;
                    }
                    r = (self.pia1_pb_in & !self.pia1_ddrb) | (self.pia1_pb_out & self.pia1_ddrb);
                } else {
                    r = self.keyboard.read();
                }
            }
            PIA1_CRB => {
                r = self.pia1_crb;
            }

            VIA_T1LO => {
                r = (self.t1 & 0xff) as u8;
                self.ifr &= !IER_TIMER1;
            }
            VIA_T1HI => {
                r = ((self.t1 >> 8) & 0xff) as u8;
            }
            VIA_T1LLO => {
                r = (self.t1_latch & 0xff) as u8;
            }
            VIA_T1LHI => {
                r = ((self.t1_latch >> 8) & 0xff) as u8;
            }
            VIA_T2LO => {
                r = (self.t2 & 0xff) as u8;
                self.ifr &= !IER_TIMER2;
            }
            VIA_T2HI => {
                r = ((self.t2 >> 8) & 0xff) as u8;
            }

            PIA2_PORTA => {
                if (self.pia2_cra & 0x04) != 0 {
                    /* Clear IRQs in CRA as side-effect of reading PA. */
                    if (self.pia2_cra & 0xC0) != 0 {
                        self.pia2_cra &= 0x3F;
                        // this.updateIrq();
                    }
                    if self.pia2_ddra == 0 {
                        self.pia2_pa_in = self.ieee.dio_in();
                    }
                    r = (self.pia2_pa_in & !self.pia2_ddra) | (self.pia2_pa_out & self.pia2_ddra);
                } else {
                    r = self.pia2_ddra;
                }
            }
            PIA2_CRA => {
                r = self.pia2_cra;
            }
            PIA2_PORTB => {
                if (self.pia2_crb & 0x04) != 0 {
                    /* Clear IRQs in CRB as side-effect of reading PB. */
                    if (self.pia2_crb & 0x3F) != 0 {
                        self.pia2_crb &= 0x3F;
                    }
                    r = (self.pia2_pb_in & !self.pia2_ddrb) | (self.pia2_pb_out & self.pia2_ddrb);
                } else {
                    r = self.pia2_ddrb;
                }
            }

            PIA2_CRB => {
                if self.ieee.srq_in() {
                    self.pia2_crb |= 0x80;
                } else {
                    self.pia2_crb &= 0x7f;
                }
                r = self.pia2_crb;
            }

            VIA_VPORTB => {
                /* Clear CB2 interrupt flag IFR3 (if not "independent"
                 * interrupt)
                 */
                if (self.via_pcr & 0xa0) != 0x20 {
                    if (self.ifr & 0x08) != 0 {
                        self.ifr &= !0x08;
                        if (self.ier & 0x08) != 0 {
                            //this.updateIrq();
                        }
                    }
                }
                /* Clear CB1 interrupt flag IFR4 */
                if (self.ifr & 0x10) != 0 {
                    self.ifr &= !0x10;
                    if (self.ier & 0x10) != 0 {
                        // this.updateIrq();
                    }
                }
                if (self.ddrb & 0x80) == 0 {
                    if self.ieee.dav_in() {
                        self.via_drb_in |= 0x80;
                    } else {
                        self.via_drb_in &= 0x7f;
                    }
                }
                if (self.ddrb & 0x40) == 0 {
                    if self.ieee.nrfd_in() {
                        self.via_drb_in |= 0x40;
                    } else {
                        self.via_drb_in &= 0xbf;
                    }
                }
                if (self.ddrb & 0x01) == 0 {
                    if self.ieee.ndac_in() {
                        self.via_drb_in |= 0x01;
                    } else {
                        self.via_drb_in &= 0xfe;
                    }
                }
                r = (self.via_drb_in & !self.ddrb) | (self.via_drb_out & self.ddrb);
            }

            VIA_VPORTA => {
                /* Clear CA2 interrupt flag IFR0 (if not "independent"
                 * interrupt)
                 */
                if (self.via_pcr & 0x0a) != 0x02 {
                    if (self.ifr & 0x01) != 0 {
                        self.ifr &= !0x01;
                        if (self.ier & 0x01) != 0 {
                            // this.updateIrq();
                        }
                    }
                }

                /* Clear CA1 interrupt flag IFR1 */
                if (self.ifr & 0x02) != 0 {
                    self.ifr &= !0x02;
                    if (self.ier & 0x02) != 0 {
                        // this.updateIrq();
                    }
                }
                r = (self.via_dra_in & !self.ddra) | (self.via_dra_out & self.ddra);
            }

            VIA_DDRB => {
                r = self.ddrb;
            }

            VIA_DDRA => {
                r = self.ddra;
            }

            VIA_SHIFT => {
                /* Clear SR int flag IFR2 */
                if (self.ifr & 0x04) != 0 {
                    self.ifr &= !0x04;
                    if (self.ier & 0x04) != 0 {
                        // nothing
                    }
                }
                r = self.via_sr;
            }
            VIA_ACR => {
                r = self.acr;
            }
            VIA_PCR => {
                r = self.via_pcr;
            }
            VIA_IFR => {
                r = self.ifr;
            }
            VIA_IER => {
                r = self.ier;
            }
            VIA_ANH => {
                // VIA_PA with no handshake.
                r = (self.via_dra_in & !self.ddra) | (self.via_dra_out & self.ddra);
            }
            _ => {}
        }

        r
    }

    pub fn write(&mut self, offset: u16, v: u8) {
        match offset {
            PIA1_PORTA => {
                if (self.pia1_cra & 0x04) != 0 {
                    self.pia1_pa_out = v;
                    // Which keyrow are we accessing?
                    if (self.pia1_pa_out & 15) < 10 {
                        self.keyboard.write(v & 0x0f);
                        self.pia1_pb_in = self.keyboard.row(); // ???? keyrow[pia1_pa_out & 15];
                    }
                } else {
                    self.pia1_ddra = v;
                }
            }
            PIA1_CRA => {
                self.pia1_cra = (self.pia1_cra & 0xc0) | (v & 0x3f);
                // Change in CA2? (screen blank)
                if (self.pia1_cra & 0x38) == 0x38 && self.pia1_ca2 == 0 {
                    // CA2 transitioning high. (Screen On)
                    self.pia1_ca2 = 1;
                    self.ieee.eoi_out(true);
                } else if (self.pia1_cra & 0x38) == 0x30 && self.pia1_ca2 != 0 {
                    // CA2 transitioning low. (Screen Blank)
                    self.pia1_ca2 = 0;
                    self.ieee.eoi_out(false);
                }
            }

            PIA1_CRB => {
                // ???
            }

            VIA_T1LLO => {
                self.t1_latch = (self.t1 & 0xff00) | v as u16;
            }
            VIA_T1LO => {
                self.t1_latch = (self.t1 & 0xff00) | v as u16;
            }
            VIA_T1LHI => {
                self.t1_latch = (self.t1_latch & 0xff) | (v as u16) << 8;
                self.ifr &= !IER_TIMER1;
            }
            VIA_T1HI => {
                self.t1 = self.t1_latch;
                self.ifr &= !IER_TIMER1;
                self.timer1 = true;
            }
            VIA_T2LO => {
                self.t2 = v as u16;
                self.timer2 = false;
                self.ifr &= !IER_TIMER2;
            }
            VIA_T2HI => {
                self.t2 += (v as u16) << 8;
                self.ifr &= !IER_TIMER2;
                self.timer2 = true;
            }
            PIA2_PORTA => {
                if self.pia2_cra & 0x04 != 0 {
                    self.pia2_pa_out = v;
                } else {
                    self.pia2_ddra = v;
                }
            }
            PIA2_CRA => {
                self.pia2_cra = (self.pia2_cra & 0xc0) | (v & 0x3f);
                self.ieee.ndac_out((self.pia2_cra & 0x08) != 0x00);
            }
            PIA2_PORTB => {
                if (self.pia2_crb & 0x04) != 0 {
                    self.pia2_pb_out = v;
                    if self.pia2_ddrb == 0xff {
                        self.ieee.dio_out(self.pia2_pb_out);
                    }
                } else {
                    self.pia2_ddrb = v;
                }
            }
            PIA2_CRB => {
                self.pia2_crb = (self.pia2_crb & 0xc0) | (v & 0x3f);
                self.ieee.dav_out((self.pia2_crb & 0x08) != 0x00);
            }

            VIA_VPORTB => {
                // Clear CB2 interrupt flag IFR3 (if not "independent" interrupt)
                if (self.via_pcr & 0xa0) != 0x20 {
                    if (self.ifr & 0x08) != 0 {
                        self.ifr &= !0x08;
                        if (self.ier & 0x08) != 0 {
                            // XXX    this.updateIrq();
                        }
                    }
                }
                // Clear CB1 interrupt flag IFR4
                if (self.ifr & 0x10) != 0 {
                    self.ifr &= !0x10;
                    if (self.ier & 0x10) != 0 {
                        // XXX    this.updateIrq();
                    }
                }
                self.via_drb_out = v;

                // IEEE outputs
                if (self.ddrb & 0x04) != 0 {
                    self.ieee.atn_out((self.via_drb_out & 0x04) != 0x00);
                }
                if (self.ddrb & 0x02) != 0 {
                    self.ieee.nrfd_out((self.via_drb_out & 0x02) != 0x00);
                }
            }
            VIA_VPORTA => {
                // Clear CA2 interrupt flag IFR0 (if not "independent" interrupt)
                if (self.via_pcr & 0x0a) != 0x02 {
                    if (self.ifr & 0x01) != 0 {
                        self.ifr &= !0x01;
                        if (self.ier & 0x01) != 0 {
                            // nothing
                        }
                    }
                }

                // Clear CA1 interrupt flag IFR1
                if (self.ifr & 0x02) != 0 {
                    self.ifr &= !0x02;
                    if (self.ier & 0x02) != 0 {
                        // nothing
                    }
                }
                self.via_dra_out = v;
            }
            VIA_DDRB => {
                self.ddrb = v;
            }
            VIA_DDRA => {
                self.ddra = v;
            }
            VIA_SHIFT => {
                /* Clear SR int flag IFR2 */
                if (self.ifr & 0x04) != 0 {
                    self.ifr &= !0x04;
                    if (self.ier & 0x04) != 0 {
                        // nothing
                    }
                }
                self.via_sr = v;
            }
            VIA_ACR => {
                self.acr = v;
            }
            VIA_PCR => {
                /* Did we change CA2 output? */
                if (self.via_pcr & 0x0c) == 0x0c
                    && (v & 0x0c) == 0x0c
                    && ((self.via_pcr ^ v) & 0x02) != 0
                {
                    // nothing
                }
                self.via_pcr = v;
            }
            VIA_IER => {
                if (v & 0x80) != 0 {
                    self.ier |= v;
                } else {
                    self.ier &= !v;
                }
            }
            VIA_ANH => {
                /* VIA_PA with no handshake. */
                self.via_dra_out = v;
            }

            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct Keyboard {
    rows: [u8; 10],
    row: u8,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            rows: [0u8; 10],
            row: 0,
        }
    }

    pub fn reset(&mut self) {
        for i in 0..10 {
            self.rows[i] = 0;
        }
    }

    pub fn read(&mut self) -> u8 {
        self.rows[self.row as usize] ^ 0xff
    }

    pub fn row(&mut self) -> u8 {
        self.row
    }

    pub fn write(&mut self, v: u8) {
        self.row = v;
    }

    /// Sets a key represented by the parameter to down.
    /// the high nibble is the row, the low nibble is the column
    pub fn key_down(&mut self, k: u8) {
        self.rows[((k & 0xf0) >> 4) as usize] |= 1 << (k & 0x0f);
    }

    /// Sets a key represented by the parameter to up.
    /// the high nibble is the row, the low nibble is the column
    pub fn key_up(&mut self, k: u8) {
        self.rows[((k & 0xf0) >> 4) as usize] &= !(1 << (k & 0x0f));
    }
}

const MY_ADDRESS: u8 = 8;

#[derive(Debug)]
enum IeeeState {
    IDLE,
    LISTEN,
    FNAME,
    LOAD,
    SAVE,
    SAVE1,
}

pub trait Storage {
    fn start_filename(&mut self);

    fn next_filename_byte(&mut self, value: u8);

    fn start_save(&mut self);

    fn end_save(&mut self);

    fn fname_done(&mut self);

    fn has_data_to_load(&mut self) -> bool;

    fn load_data_byte(&mut self, index: usize) -> u8;

    fn save_data_byte(&mut self, index: usize, value: u8);

    fn load_data_len(&mut self) -> usize;
}

impl core::fmt::Debug for dyn Storage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Storage")
    }
}

pub struct Ieee<'a> {
    state: IeeeState,
    dio: u8,
    nrfd_i: bool,
    ndac_i: bool,
    ndac_o: bool,
    nrfd_o: bool,
    atn: bool,
    dav_i: bool,
    dav_o: bool,
    srq: bool,
    eoi_i: bool,
    eoi_o: bool,

    old_rom: bool,
    data_index: usize,

    storage: &'a mut dyn Storage,
}

impl<'a> core::fmt::Debug for Ieee<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Ieee")
    }
}

impl<'a> Ieee<'a> {
    fn new(storage: &'a mut dyn Storage) -> Ieee<'a> {
        Ieee {
            state: IeeeState::IDLE,
            dio: 0,
            nrfd_i: true,
            ndac_i: true,
            ndac_o: true,
            nrfd_o: true,
            atn: true,
            dav_i: true,
            dav_o: true,
            srq: true,
            eoi_i: true,
            eoi_o: true,
            old_rom: false,
            data_index: 0,
            storage: storage,
        }
    }

    fn data_in(&mut self, d8: u8) {
        if !self.atn {
            match self.state {
                IeeeState::IDLE => {
                    if d8 == 0x20 + MY_ADDRESS {
                        self.state = IeeeState::LISTEN;
                    } else if d8 == 0x40 + MY_ADDRESS {
                        self.old_rom = false;
                        self.state = IeeeState::LOAD;
                    } else if d8 == 0x7f && self.storage.has_data_to_load() {
                        // Old PET ROMs LOAD.
                        // Assume program starts at either 0x0400 or 0x0401.
                        self.data_index = if self.storage.load_data_byte(0) == 0 {
                            2
                        } else {
                            1
                        };

                        // Put first data on bus.
                        self.dio = self.storage.load_data_byte(self.data_index) ^ 0xff;
                        self.dav_i = false;

                        self.old_rom = true;
                        self.state = IeeeState::LOAD;
                    } else if d8 == 0x3f {
                        // Old PET ROMs save
                        self.old_rom = true;
                        self.storage.start_save();
                        self.state = IeeeState::SAVE1;
                    }
                }
                IeeeState::LISTEN => {
                    // unlisten
                    if d8 == 0x3f {
                        self.state = IeeeState::IDLE;
                    } else if d8 == 0xf0 || d8 == 0xf1 {
                        // load or save
                        self.storage.start_filename();
                        self.data_index = 0;
                        self.state = IeeeState::FNAME;
                    } else if d8 == 0x61 {
                        self.storage.start_save();
                        self.state = IeeeState::SAVE;
                    }
                }
                IeeeState::FNAME => {
                    // unlisten
                    if d8 == 0x3f {
                        self.storage.fname_done();
                        self.state = IeeeState::IDLE;
                    }
                }
                IeeeState::LOAD => {
                    // untalk
                    if d8 == 0x5f {
                        self.state = IeeeState::IDLE;
                    }
                }
                IeeeState::SAVE => {
                    // unlisten
                    if d8 == 0x3f {
                        self.storage.end_save();
                        self.state = IeeeState::IDLE;
                    }
                }
                IeeeState::SAVE1 => {
                    if self.eoi_o {
                        // Data comes with ATN low in old ROMs.
                        self.storage.save_data_byte(self.data_index, d8);
                        self.data_index += 1;
                    } else {
                        // Ignore last byte.
                        self.storage.end_save();
                        self.old_rom = false;
                        self.state = IeeeState::IDLE;
                    }
                }
            }
        } else {
            match self.state {
                IeeeState::FNAME => {
                    self.storage.next_filename_byte(d8);
                }
                IeeeState::SAVE => {
                    self.storage.save_data_byte(self.data_index, d8);
                    self.data_index += 1;
                }
                _ => {}
            }
        }
    }

    fn dio_out(&mut self, d8: u8) {
        self.dio = d8;
    }

    fn dio_in(&self) -> u8 {
        self.dio
    }

    fn ndac_in(&self) -> bool {
        self.ndac_i && self.ndac_o
    }

    fn ndac_out(&mut self, flag: bool) {
        if !self.ndac_o && flag {
            // Positive transition of NDAC.  Data acknowledged.
            if let IeeeState::LOAD = self.state {
                self.dav_i = true;
                self.eoi_i = true;
                self.data_index += 1;
                if self.old_rom && self.data_index == self.storage.load_data_len() {
                    self.state = IeeeState::IDLE;
                }
            }
        }
        self.ndac_o = flag;
    }

    fn nrfd_in(&self) -> bool {
        self.nrfd_i && self.nrfd_o
    }

    fn nrfd_out(&mut self, flag: bool) {
        if !self.nrfd_o && flag {
            // Positive transition of NRFD.  Put data on bus.
            if let IeeeState::LOAD = self.state {
                if self.data_index < self.storage.load_data_len() {
                    self.dio = self.storage.load_data_byte(self.data_index) ^ 0xff;
                    self.dav_i = false;
                    if self.data_index == self.storage.load_data_len() - 1 {
                        self.eoi_i = false;
                    }
                }
            }
        }
        self.nrfd_o = flag;
    }

    fn eoi_in(&self) -> bool {
        self.eoi_i && self.eoi_o
    }

    fn eoi_out(&mut self, flag: bool) {
        self.eoi_o = flag;
    }

    fn atn_out(&mut self, flag: bool) {
        if self.atn && !flag {
            self.ndac_i = false;
        } else if !self.atn && flag {
            if let IeeeState::LOAD = self.state {
                if self.nrfd_o {
                    // put 1st byte on bus
                    self.dio = self.storage.load_data_byte(0) ^ 0xff;
                    self.dav_i = false;
                }
            }
        }
        self.atn = flag;
    }

    fn dav_out(&mut self, flag: bool) {
        if self.dav_o && !flag {
            // Negative transition of DAV.
            self.ndac_i = true;
            self.nrfd_i = false;
            self.data_in(self.dio ^ 0xff);
        } else if !self.dav_o && flag {
            // Positive transition of DAV.
            self.ndac_i = false;
            self.nrfd_i = true;
        }
        self.dav_o = flag;
    }

    fn dav_in(&self) -> bool {
        self.dav_i && self.dav_o
    }

    fn srq_in(&self) -> bool {
        self.srq
    }
}
