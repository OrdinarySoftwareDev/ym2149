#![no_std]
#![no_main]

use core::pin;

use rp2040_boot2;
#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use hal::entry;
use defmt_rtt as _;
use panic_probe as _;

use pwm_freq::PwmFreq;


use rp2040_hal::{self as hal, pac::io_bank0::gpio};

use hal::{
    clocks::init_clocks_and_plls,
    pac,
    sio::Sio,
    gpio::{ PinState, PinGroup },
    watchdog::Watchdog,
    Clock
};

use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;
use embedded_hal::digital::OutputPin;

mod ym2149f;
use crate::ym2149f::YM2149F;

pub enum Mode {
    INACTIVE,
    //READ,
    WRITE,
    ADDRESS
}

fn set_mode(
    bdir: &mut impl OutputPin,
    bc1: &mut impl OutputPin,
    mode: Mode
){
    use PinState::{*};

    let arr: [PinState; 3] = match mode {
        //MODE:: (...) => [BDIR, BC2, BC1],
        Mode::INACTIVE => [Low, High, Low],
        //Mode::READ => [false, true, true],
        Mode::WRITE => [High, High, Low],
        Mode::ADDRESS => [High, High, High]
    };

    bdir.set_state(arr[0]).unwrap();
    //bc2.set_state(arr[1]).unwrap();
    bc1.set_state(arr[2]).unwrap();
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    //let core = pac::CorePeripherals::take().unwrap();
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

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sysclkfreq = clocks.system_clock.freq().to_Hz();
    let mut delay = rp2040_hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    let pwm: &mut rp2040_hal::pwm::Slice<rp2040_hal::pwm::Pwm1, rp2040_hal::pwm::FreeRunning> =
        &mut pwm_slices.pwm1;
    pwm.set_top(0);
    pwm.set_ph_correct();
    pwm.enable();

    // Output channel A on PWM0 to GPIO 0
    let channel = &mut pwm.channel_a;
    channel.output_to(pins.gpio18);
    delay.delay_ns(100_000_000);

    // Target 2MHz
    let target_freq = 20_000;
    let pwm_freq = PwmFreq::new(sysclkfreq, target_freq).unwrap();
    let pwm_config = pwm_freq.get_config();

    pwm.channel_b.set_duty_cycle(pwm_config.top / 2).unwrap();
    pwm.set_top(pwm_config.top);
    pwm.set_div_int(pwm_config.div.0);
    pwm.set_div_frac(pwm_config.div.1);

    // turn on onboard led to give any sign of life
    let mut led = pins.gpio25.into_push_pull_output();

    // pins
    let mut bc1 = pins.gpio9.into_push_pull_output();
    let mut bdir = pins.gpio10.into_push_pull_output();

    let mut data_bus = PinGroup::new()
        .add_pin(pins.gpio1.into_push_pull_output()) // LSB, DA0
        .add_pin(pins.gpio2.into_push_pull_output()) // ...
        .add_pin(pins.gpio3.into_push_pull_output()) // ...
        .add_pin(pins.gpio4.into_push_pull_output()) // ...
        .add_pin(pins.gpio5.into_push_pull_output()) // ...
        .add_pin(pins.gpio6.into_push_pull_output()) // ...
        .add_pin(pins.gpio7.into_push_pull_output()) // ...
        .add_pin(pins.gpio8.into_push_pull_output()); // MSB, DA8

    data_bus.set_u32(0x00);

    let chip = YM2149F::new(bc1, bdir)

    macro_rules! write_to_bus {
        ($value:expr) => {
            data_bus.set_u32(($value * 2) as u32);
        };
    }

    macro_rules! write_register {
        ($register:expr, $value:expr) => {
            set_mode(&mut bdir, &mut bc1, Mode::ADDRESS);
            write_to_bus!($register);
            set_mode(&mut bdir, &mut bc1, Mode::INACTIVE);
            set_mode(&mut bdir, &mut bc1, Mode::WRITE);
            write_to_bus!($value);
            set_mode(&mut bdir, &mut bc1, Mode::INACTIVE);
        };
    }

    loop {}
}
