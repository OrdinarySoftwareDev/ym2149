//! Example code that tests all functions of the crate across all channels.
//!
//! That includes:
//! - The modes INACTIVE, ACCESS, WRITE (READ is not yet impld)
//! - Tone generation (sweep)
//! - Noise generation (sweep)
//! - Envelopes
//! - Sending / receiving data through the I/O ports
#![no_std]
#![no_main]

// Bootloader
use rp2040_boot2;
#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

// Deps
use defmt_rtt as _;
use panic_halt as _;

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use rp2040_hal::{self as hal};

use hal::{clocks::init_clocks_and_plls, pac, sio::Sio, watchdog::Watchdog};

// The actual ym2149 HAL crate
use ym2149::*;

#[hal::entry]
fn main() -> ! {
    // Default configuration
    let mut pac = pac::Peripherals::take().unwrap();
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

    let mut timer = rp2040_hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Turn on the LED to give any sign of life (optional)
    let mut led = pins.gpio25.into_push_pull_output();
    led.set_high();

    // Frequency (in Hz, u32) of the clock the chip is connected to (Pin 22 on the YM2149)
    let master_clock_freq: u32 = 2_000_000;

    // DynPins for the 8-bit data bus (LSB, pin D0 to MSB, pin D7)
    let data_pins = [
        pins.gpio1.into_push_pull_output().into_dyn_pin(),
        pins.gpio2.into_push_pull_output().into_dyn_pin(),
        pins.gpio3.into_push_pull_output().into_dyn_pin(),
        pins.gpio4.into_push_pull_output().into_dyn_pin(),
        pins.gpio5.into_push_pull_output().into_dyn_pin(),
        pins.gpio6.into_push_pull_output().into_dyn_pin(),
        pins.gpio7.into_push_pull_output().into_dyn_pin(),
        pins.gpio8.into_push_pull_output().into_dyn_pin(),
    ];

    // Initialize a DataBus
    let mut data_bus = DataBus::new(data_pins);
    data_bus.write_u8(0); // Write 0b0000_0000 as a safety measure

    // Bus control decoder pins
    let bc1 = pins.gpio9.into_push_pull_output();
    let bdir = pins.gpio10.into_push_pull_output();

    // Build the chip by passing:
    let mut chip = YM2149::new(
        data_bus,          // - A variable w/ type that implements the `OutputBus` trait
        master_clock_freq, // - The frequency of the master clock
        bc1,               // - The GPIO pin connected to BC1
        bdir,              // - The GPIO pin connected to BDIR
    );

    // Set the chip's mode to `Inactive`
    chip.set_mode(Mode::INACTIVE);
    // Configure the mixer according to the datasheet (the docs for IoPortMixerSettings also explain this process)
    chip.write_register(Register::IoPortMixerSettings, 0b11111110);

    // Reset the chip (optional but recommended)
    let mut reset_pin = pins.gpio11.into_push_pull_output();

    reset_pin.set_low();
    timer.delay_ms(10);
    reset_pin.set_high();
    timer.delay_ms(10);

    // Sweep code
    let mut c: u16 = 0x001;

    loop {
        // Set channel A's volume to 0x0F (there are only 4 bits dedicated to channel levels)
        chip.volume(AudioChannel::A, 0xF);
        // Play a tone on channel A with TP = c
        // The TP consists of 12 bits: 4 bits for 'rough', and 8 bits for 'fine' adjustment.
        if c < 0xA00 {
            chip.tone(AudioChannel::A, c);
            c += 0x002;
            timer.delay_ms(5);
        } else {
            timer.delay_ms(500);
            c = 0x001;
        }
    }
}
