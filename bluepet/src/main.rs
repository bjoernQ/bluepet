#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m_rt::entry;
use mos6502::Cpu;
use pet::Ram;

mod keyboard;
mod storage;
mod video;

use storage::FlashStorage;

use stm32f1xx_hal::spi::{Mode, Phase, Polarity, Spi};
use stm32f1xx_hal::{pac, prelude::*};

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::MODE_0;

use spi_memory::series25::Flash;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // Get access to the core peripherals from the cortex-m crate
    let mut cp = cortex_m::Peripherals::take().unwrap();
    // Get access to the device specific peripherals from the peripheral access crate
    let dp = pac::Peripherals::take().unwrap();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(72.mhz())
        .pclk1(36.mhz())
        .pclk2(72.mhz())
        .freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
    let (pa15, pb3, pb4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);

    // configure video pins
    let _pb8 = gpiob.pb8.into_push_pull_output(&mut gpiob.crh);
    let _pb9 = gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh); // timer controlled

    // configure keyboard pins
    let mut pa0 = gpioa.pa0.into_push_pull_output(&mut gpioa.crl);
    let mut pa1 = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
    let mut pa2 = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);
    let mut pa3 = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
    let mut pa4 = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let mut pa5 = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);
    let mut pa6 = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
    let mut pa7 = gpioa.pa7.into_push_pull_output(&mut gpioa.crl);

    let pb0 = gpiob.pb0.into_pull_down_input(&mut gpiob.crl);
    let pb1 = gpiob.pb1.into_pull_down_input(&mut gpiob.crl);
    let pb10 = gpiob.pb10.into_pull_down_input(&mut gpiob.crh);
    let pb11 = gpiob.pb11.into_pull_down_input(&mut gpiob.crh);
    let pa8 = gpioa.pa8.into_pull_down_input(&mut gpioa.crh);
    let pa10 = gpioa.pa10.into_pull_down_input(&mut gpioa.crh);
    let pa11 = gpioa.pa11.into_pull_down_input(&mut gpioa.crh);
    let pa15 = pa15.into_pull_down_input(&mut gpioa.crh);
    let pb3 = pb3.into_pull_down_input(&mut gpiob.crl);

    // PB4 = shift
    let pb4 = pb4.into_pull_down_input(&mut gpiob.crl);

    // configure SPI for SPI Flash
    let cs = {
        let mut cs = gpioa.pa9.into_push_pull_output(&mut gpioa.crh);
        cs.set_high().unwrap(); // deselect
        cs
    };

    let pins = (
        gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh),
        gpiob.pb14.into_floating_input(&mut gpiob.crh),
        gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh),
    );

    let spi = Spi::spi2(dp.SPI2, pins, MODE_0, 4.mhz(), clocks, &mut rcc.apb1);

    // for testing use the onboard led
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    let spi_flash = Flash::init(spi, cs).unwrap();

    //spi_flash.erase_all();
    let mut file_storage = FlashStorage::new(spi_flash);

    // prepare video stuff
    video::init_video(&mut cp, dp.TIM4, dp.TIM1);

    // init the emulator stuff
    let mut ram = [0u8; 8192];

    let mem = unsafe { Ram::new(&mut ram, &mut video::VID_RAM, &mut file_storage) };
    let mut cpu = Cpu::new(&mem);
    cpu.reset();

    // emulation
    let mut cycle_cnt: u64 = 0;
    let mut tick_cntr = 0u32;

    let mut keyboard_cnt = 0;

    led.set_high().unwrap_or_default(); // on board LED off

    // start video output
    video::start_video();

    loop {
        if cycle_cnt == 0 {
            cycle_cnt = cpu.step();
            cycle_cnt -= 1;
        } else {
            cycle_cnt -= 1;
        }

        tick_cntr += 1;
        if tick_cntr >= 1000 {
            tick_cntr = 0;
            let trigger_irq = mem.io.borrow_mut().tick();
            if trigger_irq {
                cpu.trigger_irq();
            }
        }

        keyboard_cnt += 1;
        if keyboard_cnt > 20830 {
            keyboard_cnt = 0;

            keyboard::handle_keyboard(
                &mut pa0,
                &mut pa1,
                &mut pa2,
                &mut pa3,
                &mut pa4,
                &mut pa5,
                &mut pa6,
                &mut pa7,
                &pb0,
                &pb1,
                &pb10,
                &pb11,
                &pa8,
                &pa10,
                &pa11,
                &pa15,
                &pb3,
                &pb4,
                &mut (mem.io.borrow_mut().keyboard),
            );
        }
    }
}
