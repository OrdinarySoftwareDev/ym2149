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
//mod ym2149f;

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

//use crate::ym2149f::YM2149F;

pub enum Mode {
    INACTIVE,
    //READ,
    WRITE,
    ADDRESS
}

pub enum Register {
    AFreq8bitFinetone,
    AFreq4bitRoughtone,
    BFreq8bitFinetone,
    BFreq4bitRoughtone,
    CFreq8bitFinetone,
    CFreq4bitRoughtone,
    NoiseFreq5bit,
    IoPortMixerSettings,
    ALevel,
    BLevel,
    CLevel,
    EFreq8bitFineAdj,
    EFreq8bitRoughAdj,
    EShape,
    DataIoA,
    DataIoB
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

    set_mode(&mut bdir, &mut bc1, Mode::INACTIVE);

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

    write_register!(7, 0b11111110); //  0 = enabled
    write_register!(8, 0b00001111);

    let mut c: i32 = 0x001;
    let mut dir = true;

    delay.delay_ms(100);

    loop {
        let rough = c & 0x0FF;
        let fine = c & 0xF00;

        write_register!(0, rough);
        write_register!(1, (fine >> 8) & 0xF);

        c += (dir as i32) * 2 - 1;

        if (c == 0x0FF) || (c == 0x000) { // 0xFF = 490Hz, 0x00 = inf Hz, fT = fMaster / 16*TP, fMaster = fT*16*TP, fMaster ~= 490*16*255 ~= 1999200
            dir = !dir;
            led.set_high();
            delay.delay_ms(250);
            led.set_low();
            delay.delay_ms(250);
        }

        delay.delay_ms(10);

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
}
