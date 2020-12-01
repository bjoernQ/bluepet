use cortex_m::Peripherals;
use stm32f1xx_hal::pac::{interrupt, Interrupt};
use stm32f1xx_hal::{pac};

use core::mem::MaybeUninit;

static mut TIMER_TIM4: MaybeUninit<pac::TIM4> = MaybeUninit::uninit();
static mut TIMER_TIM1: MaybeUninit<pac::TIM1> = MaybeUninit::uninit();

pub static mut VID_RAM: [u8; 2048] = [0u8; 2048];

static mut SCANLINE_PIXELS: [u8; 40] = [0u8; 40];

static mut CHAR_ROM_RAM: [u8; 2048] = [0u8; 2048];

const ISR_OVERHEAD_CORRECTION: u16 = 110;

pub fn init_video(cp: &mut Peripherals, tim4: pac::TIM4, tim1: pac::TIM1) {
    unsafe {
        for i in 0..2048 {
            CHAR_ROM_RAM[i] = CHAR_ROM[i];
        }
    }

    unsafe {
        pac::NVIC::unmask(Interrupt::TIM4);
        pac::NVIC::unmask(Interrupt::TIM1_UP);

        cp.NVIC.set_priority(Interrupt::TIM4, 16);
        cp.NVIC.set_priority(Interrupt::TIM1_UP, 32);
    }

    unsafe {
        (*pac::RCC::ptr())
            .apb1enr
            .modify(|_, w| w.tim4en().set_bit());

        (*pac::RCC::ptr())
            .apb2enr
            .modify(|_, w| w.tim1en().set_bit());
    }

    // configure TIM4
    configure_tim4(&tim4);

    // configure TIM1
    configure_tim1(&tim1);

    // make timer accessible from the isr
    unsafe {
        let timer_static = TIMER_TIM4.as_mut_ptr();
        *timer_static = tim4;

        let timer_static = TIMER_TIM1.as_mut_ptr();
        *timer_static = tim1;
    }
}

pub fn start_video() {
    schedule(HALF_SCANLINE_ARR, SHORT_SYNC_CRR);
}

#[inline(always)]
fn schedule(new_arr: u16, new_crr: u16) {
    unsafe {
        let tim1 = TIMER_TIM1.as_mut_ptr();
        (*tim1).cnt.write(|w| w.bits(0));
        (*tim1).arr.modify(|_, w| w.arr().bits(new_arr - 15)); // TODO to const find a good value / maybe even -0
                                                               // start timer
        (*tim1).cr1.modify(|_, w| {
            w.cen().set_bit() // START!
        });

        let tim = TIMER_TIM4.as_mut_ptr();
        (*tim).arr.modify(|_, w| w.arr().bits(new_arr));
        (*tim).ccr4.write(|w| w.bits(new_crr as u32));
        // start anew
        (*tim).cr1.modify(|_, w| w.cen().set_bit());
    }
}

fn configure_tim4(tim: &stm32f1xx_hal::pac::TIM4) {
    tim.arr.modify(|_, w| {
        w.arr().bits(0) // right value in schedule
    });

    tim.ccr4.modify(|_, w| {
        w.ccr().bits(0) // right value in schedule
    });

    tim.psc.modify(|_, w| {
        w.psc().bits(0) // no prescaler
    });

    // pwm mode etc
    tim.ccmr2_output_mut().modify(|_, w| {
        w.oc4m()
            .pwm_mode2() // pwm mode low/high
            .oc4pe()
            .clear_bit() // disable output compare preload
            .oc4fe()
            .set_bit() // enable fast mode
            .cc4s()
            .output()
    });

    // output enable channel 4
    tim.ccer.modify(|_, w| w.cc4e().set_bit());

    // enable update interrupt
    tim.dier.modify(|_, w| w.uie().set_bit());

    // The psc register is buffered, so we trigger an update event to update it
    // Sets the URS bit to prevent an interrupt from being triggered by the UG bit
    tim.cr1.modify(|_, w| w.urs().set_bit());
    tim.egr.write(|w| w.ug().set_bit());
    tim.cr1.modify(|_, w| w.urs().clear_bit());

    tim.cr1.modify(|_, w| {
        w.cms()
            .bits(0b00) // center aligned etc.
            .dir()
            .clear_bit() // upcounting
            .opm()
            .set_bit() // one shot / one pulse
    });
}

fn configure_tim1(tim: &stm32f1xx_hal::pac::TIM1) {
    tim.cnt.write(|w| unsafe { w.bits(0) });

    tim.arr.modify(|_, w| w.arr().bits(0)); // right value in schedule

    tim.psc.modify(|_, w| {
        w.psc().bits(0) // no prescaler
    });

    // enable update interrupt
    tim.dier.modify(|_, w| w.uie().set_bit());

    // The psc register is buffered, so we trigger an update event to update it
    // Sets the URS bit to prevent an interrupt from being triggered by the UG bit
    tim.cr1.modify(|_, w| w.urs().set_bit());
    tim.egr.write(|w| w.ug().set_bit());
    tim.cr1.modify(|_, w| w.urs().clear_bit());

    // start timer
    tim.cr1.modify(|_, w| {
        w.cms()
            .bits(0b00) // center aligned etc.
            .dir()
            .clear_bit() // upcounting
            .opm() // one shot / one pulse
            .enabled()
    });
}

static mut IDX: usize = 0usize;

const START_AT_SCANLINE: usize = 80;
const STOP_AT_SCANLINE: usize = START_AT_SCANLINE + 200;

#[interrupt]
fn TIM4() {
    unsafe {
        let tim = TIMER_TIM4.as_mut_ptr();
        // clear timer interrupt
        (*tim).sr.modify(|_, w| w.uif().clear_bit());

        let has_pixels = DATA[IDX].has_pixels;
        let new_arr = DATA[IDX].arr;
        let new_crr = DATA[IDX].ccr;
        schedule(new_arr, new_crr);

        if has_pixels && IDX >= START_AT_SCANLINE && IDX < STOP_AT_SCANLINE {
            // copy pixel data (1 bit = 1 pixel) to prepare it to be shown on screen
            let y = IDX - START_AT_SCANLINE;
            let line = y / 8;
            let vid_ram_idx = line as isize * 40;

            let scanline_mod = y % 8;

            asm!(
                "loop:",

                "mov {char_rom_ptr}, {char_rom_base}",
                "ldrb {chr_data},[{chr_data_ptr}]", // char from vidram
                "tst {chr_data},#0b10000000",
                "and {chr_data},#0b01111111",
                "lsl {chr_data}, #3",
                "orr {chr_data},{scanline_mod}",
                "add {char_rom_ptr},{chr_data}",
                "ldrb {chr_data},[{char_rom_ptr}]",

                "beq not_inverse",
                "eor {chr_data},#255",
                "not_inverse:",

                "strb {chr_data}, [{pxl_data_ptr}]",

                "add {pxl_data_ptr}, #1",
                "add {chr_data_ptr}, #1",

                "subs {pxl_count}, #1",
                "bne loop",

                chr_data_ptr = in(reg) VID_RAM.as_ptr().offset(vid_ram_idx),
                pxl_data_ptr = inout(reg)  SCANLINE_PIXELS.as_mut_ptr() => _,
                pxl_count = in(reg) 40,
                char_rom_base = in(reg) CHAR_ROM_RAM.as_ptr(),
                scanline_mod = in(reg) scanline_mod,
                char_rom_ptr = out(reg) _,
                chr_data = out(reg) _,
            );

            draw_pxls();
        }

        IDX += 1;
        if IDX >= DATA.len() {
            IDX = 0;
        }
    };
}

#[inline(always)]
fn draw_pxls() {
    unsafe {
        asm!(
            "tim4_busy_loop:",
            "ldrh {cntr},[{tim4_cnt}]",
            "cmp {cntr}, {cntr_dst}",
            "blo tim4_busy_loop",

            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",




            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "ldrb {xchr_data},[{xpxl_data_ptr}]",

            "tst {xchr_data}, #128",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #64",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #32",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #16",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #8",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #4",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "nop",
            "nop",

            "tst {xchr_data}, #2",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",
            "add {xpxl_data_ptr}, #1",

            "tst {xchr_data}, #1",
            "ite eq",
            "moveq {gpio_write_data},#0x1000000",
            "movne {gpio_write_data},#0x100",
            "str {gpio_write_data}, [{gpio_reg}]",


            "mov {gpio_write_data},#0x1000000",
            "str {gpio_write_data}, [{gpio_reg}]",

            xpxl_data_ptr = in(reg)  SCANLINE_PIXELS.as_mut_ptr(),
            gpio_write_data = out(reg) _,
            xchr_data = out(reg) _,
            gpio_reg = in(reg) 0x40010c10,
            cntr = in(reg) 0,
            cntr_dst = in(reg) 1270, // START AT THIS TIM4 CNT VALUE
            tim4_cnt = in(reg) 0x4000_0824 // TIM4 CNT

        );
    }
}

#[interrupt]
fn TIM1_UP() {
    unsafe {
        let tim1 = TIMER_TIM1.as_mut_ptr();
        (*tim1).sr.modify(|_, w| w.uif().clear_bit());
        asm!("wfi");
    }
}

struct Data {
    arr: u16,
    ccr: u16,
    has_pixels: bool,
}

const FULL_SCANLINE_ARR: u16 = 2307 * 2 - ISR_OVERHEAD_CORRECTION; // 64
const HALF_SCANLINE_ARR: u16 = 2305 - ISR_OVERHEAD_CORRECTION;

const BROAD_SYNC_CRR: u16 = 1966 - ISR_OVERHEAD_CORRECTION; // 4.7
const SHORT_SYNC_CRR: u16 = 182 - ISR_OVERHEAD_CORRECTION; // 2.35
const H_SYNC_CRR: u16 = 344 - ISR_OVERHEAD_CORRECTION; // 4.7
                                                       // front porch (after pixel data) 1.64
                                                       // back purch (before pixel data, after h-sync) 5.7

const _DATA: [Data; 4] = [
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    // scanline 6 - 23
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
];

const DATA: [Data; 312 + 5 + 3] = [
    // scanline 1 - 5
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: BROAD_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: BROAD_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: BROAD_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: BROAD_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: BROAD_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    // scanline 6 - 23
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: false,
    },
    // scanline 24 - 309
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    Data {
        arr: FULL_SCANLINE_ARR,
        ccr: H_SYNC_CRR,
        has_pixels: true,
    },
    // scanline 310 - 312
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
    Data {
        arr: HALF_SCANLINE_ARR,
        ccr: SHORT_SYNC_CRR,
        has_pixels: false,
    },
];

const CHAR_ROM: &'static [u8] = include_bytes!("../../char_rom/characters-2.901447-10.bin");
