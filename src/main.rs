#![no_std]
#![no_main]

use rp2040_boot2;
#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use hal::entry;
use defmt_rtt as _;
use panic_probe as _;

use embedded_hal::digital::{ OutputPin };



use rp2040_hal::{self as hal };

use hal::{
    clocks::init_clocks_and_plls,
    pac,
    sio::Sio,
    watchdog::Watchdog
};

mod ym2149;
use crate::ym2149::{ * };

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    //let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let external_xtal_freq_hz = 12_000_000u32;
    let _clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // turn on onboard led to give any sign of life
    let mut led = pins.gpio25.into_push_pull_output();

    led.set_high();

    // pins
    let bc1 = pins.gpio9.into_push_pull_output();
    let bdir = pins.gpio10.into_push_pull_output();

    let pins = [
        pins.gpio1.into_push_pull_output().into_dyn_pin(),
        pins.gpio2.into_push_pull_output().into_dyn_pin(),
        pins.gpio3.into_push_pull_output().into_dyn_pin(),
        pins.gpio4.into_push_pull_output().into_dyn_pin(),
        pins.gpio5.into_push_pull_output().into_dyn_pin(),
        pins.gpio6.into_push_pull_output().into_dyn_pin(),
        pins.gpio7.into_push_pull_output().into_dyn_pin(),
        pins.gpio8.into_push_pull_output().into_dyn_pin()
    ];

    let mut data_bus = DataBus::new(pins);

    data_bus.write_u8(0b10100101);

    let mut chip = YM2149::new(
        data_bus,
        2_000_000,
        bc1,
        bdir
    );

    chip.set_mode(Mode::INACTIVE);

    loop {}
}
