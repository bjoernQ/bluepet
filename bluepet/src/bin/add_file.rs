#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m_rt::entry;

use bluepet::storage::FlashStorage;
use pet::io::Storage;

use stm32f1xx_hal::spi::{Mode, Phase, Polarity, Spi};
use stm32f1xx_hal::{pac, prelude::*};

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::MODE_0;

use spi_memory::prelude::*;
use spi_memory::series25::Flash;

const FILEDATA: &[u8; 1992] = include_bytes!("../../data/pet2001-logo.prg");

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // Get access to the core peripherals from the cortex-m crate
    let _cp = cortex_m::Peripherals::take().unwrap();
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
    let (_pa15,_pb33,_pb4b4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);

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
    let mut spi_flash = Flash::init(spi, cs).unwrap();

    spi_flash.erase_all().unwrap_or_default();
    let mut file_storage = FlashStorage::new(spi_flash);


    let filename = "HELLO";

    file_storage.start_filename();
    
    for c in filename.chars() {
        file_storage.next_filename_byte(c as u8);
    }

    file_storage.start_save();

    let mut i = 0;
    for b in FILEDATA {
        file_storage.save_data_byte(i, *b);
        i += 1;
    }
    file_storage.end_save();

    rprintln!("done. writen file as {}", filename);

    file_storage.ensure_directory();
    for entry in &file_storage.directory {
        rprintln!("{:?}", entry);
    }
    loop {}
}
