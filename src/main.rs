#![no_std]
#![no_main]

use bsp::entry;
use cortex_m::asm::delay;
use defmt_rtt as _;
use panic_probe as _;

mod ym2149f;

use rp_pico::{self as bsp, hal::gpio::PinGroup};
use bsp::hal as hal;
use bsp::pac::Peripherals;

use hal::{
    clocks::{ init_clocks_and_plls },
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use hal::gpio::{DynPinId, FunctionSioOutput, Pin, Pins, PullNone};
use hal::{Clock, clocks::ClocksManager};

use embedded_hal::pwm::SetDutyCycle;
use embedded_hal::digital::OutputPin;

use crate::ym2149f::YM2149F;



#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
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

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    /*macro_rules! ym2149f {
        ($pins:expr, $($gpio:ident),*) => {{
            YM2149F::new(
                $(
                    $pins.$gpio.into_push_pull_output()
                ),*
            )
        }};
    }

    let mut chip = ym2149f!(pins,
        gpio28,
        gpio18, gpio19, gpio20, gpio21, gpio22, gpio12, gpio11, gpio10, // D0 through D7
        gpio26,
        gpio2,
        gpio27
    );*/

    use hal::pwm::{*};

    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut pwm = pwm_slices.pwm0;

    let peripheral_freq = clocks.peripheral_clock.freq().to_Hz();

    let target_freq = 2_000_000u32;
    let div_int = ((peripheral_freq / target_freq) / 16) as u8;

    pwm.set_div_int(div_int);
    pwm.set_div_frac(0);
    pwm.enable();

    let channel = &mut pwm.channel_a;
    channel.output_to(pins.gpio0);
    channel.set_duty_cycle_fraction(1, 2);

    pins.led.into_push_pull_output().set_high();

    let mut data_bus = rp2040_hal::gpio::PinGroup::new()
        .add_pin(pins.gpio1.into_push_pull_output()) // LSB, DA0
        .add_pin(pins.gpio2.into_push_pull_output())
        .add_pin(pins.gpio3.into_push_pull_output())
        .add_pin(pins.gpio4.into_push_pull_output())
        .add_pin(pins.gpio5.into_push_pull_output())
        .add_pin(pins.gpio6.into_push_pull_output())
        .add_pin(pins.gpio7.into_push_pull_output())
        .add_pin(pins.gpio8.into_push_pull_output()); // MSB, DA8



    loop {

    }

    // 00000000 write 0x00 to bus
    // 11100000 write to r7 (io)
    // 10000011 enable channel a, io a/b
    // 00010000 write to r8 (channel a level)
    // 11110000 set level to 0x0F
    // 00000000 write to r0 (channel a freq. fine)
    // 10011000 write 0x19
    // 10000000 write to r1 (channel a freq. rough)
    // 10000000 write 0x01

    /*loop {
        for i in 0..8 { // test sequence
            chip.write_data_bus(2_u8.pow(i) as u8);
        }
    }*/
}
