use cortex_m::asm::delay;
use defmt::info;

use embedded_hal::digital::{OutputPin, PinState};


pub struct YM2149F<RESET, D0, D1, D2, D3, D4, D5, D6, D7, BC1, BC2, BDIR>
where
    RESET: OutputPin,
    D0: OutputPin,
    D1: OutputPin,
    D2: OutputPin,
    D3: OutputPin,
    D4: OutputPin,
    D5: OutputPin,
    D6: OutputPin,
    D7: OutputPin,
    BC1: OutputPin,
    BC2: OutputPin,
    BDIR: OutputPin
{
    reset: RESET,
    data_pins: (D0, D1, D2, D3, D4, D5, D6, D7),
    bc1: BC1,
    bc2: BC2,
    bdir: BDIR
}

#[repr(u8)]
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

pub enum Mode {
    INACTIVE,
    //READ,
    WRITE,
    ADDRESS
}

macro_rules! set_pins {
    ($value:expr, $pins:expr, $($idx:tt),*) => {
        $(
            $pins.$idx.set_state(
                if ($value >> $idx) & 1 == 1 { PinState::High } else { PinState::Low }
            ).unwrap();
        )*
    };
}

impl <RESET, D0, D1, D2, D3, D4, D5, D6, D7, BC1, BC2, BDIR> YM2149F<RESET, D0, D1, D2, D3, D4, D5, D6, D7, BC1, BC2, BDIR>
where
    RESET: OutputPin,
    D0: OutputPin,
    D1: OutputPin,
    D2: OutputPin,
    D3: OutputPin,
    D4: OutputPin,
    D5: OutputPin,
    D6: OutputPin,
    D7: OutputPin,
    BC1: OutputPin,
    BC2: OutputPin,
    BDIR: OutputPin,
{
    pub fn new(reset: RESET, d0: D0, d1: D1, d2: D2, d3: D3, d4: D4, d5: D5, d6: D6, d7: D7, bc1: BC1, bc2: BC2, bdir: BDIR) -> Self {
        Self {
            reset,
            data_pins: (d0, d1, d2, d3, d4, d5, d6, d7),
            bc1,
            bc2,
            bdir,
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        use PinState::{*};

        let arr: [PinState; 3] = match mode {
            //MODE:: (...) => [BDIR, BC2, BC1],
            Mode::INACTIVE => [Low, High, Low],
            //Mode::READ => [false, true, true],
            Mode::WRITE => [High, High, Low],
            Mode::ADDRESS => [High, High, High]
        };

        self.bdir.set_state(arr[0]).unwrap();
        self.bc2.set_state(arr[1]).unwrap();
        self.bc1.set_state(arr[2]).unwrap();
    }

    pub fn write_data_bus(&mut self, value: u8) {
        info!("Writing to data bus...");
        set_pins!(value, self.data_pins, 7,6,5,4,3,2,1,0);
        //{
            //delay(1_000_000_000);
            //}
    }

    pub fn write_register(&mut self, register: Register, value: u8) {
        info!("Writing to register...");
        self.write_data_bus(register as u8);
        self.set_mode(Mode::ADDRESS);
        self.set_mode(Mode::INACTIVE);
        self.write_data_bus(value);
        self.set_mode(Mode::WRITE);
        self.set_mode(Mode::INACTIVE);
    }

    pub fn toneA(&mut self, period: u16) {
        info!("Playing tone...");
        let tp: [u8; 2] = period.to_le_bytes();
        self.write_register(Register::AFreq8bitFinetone, tp[0]); // Fine tone, 8 bits
        self.write_register(Register::AFreq4bitRoughtone, tp[1]); // Rough tone, 4 bits
    }

    pub fn enableA(&mut self) {
        self.write_register(Register::IoPortMixerSettings, 0b00111110);
    }

    pub fn volumeA(&mut self, volume: u8) {
        self.write_register(Register::ALevel, volume);
    }

    pub fn tone_hz(&mut self, frequency: u32) {
        info!("Playing tone (Hz)...");
        let tp: [u8; 4] = (2_000_000 / (16 * frequency)).to_le_bytes();

        self.write_register(Register::AFreq8bitFinetone, tp[0]); // Fine tone, 8 bits
        self.write_register(Register::AFreq4bitRoughtone, tp[1]); // Rough tone, 4 bits
        // The remaining bytes are IGNORED
    }

    pub fn reset_burst(&mut self) {
        self.reset.set_low().unwrap();
        delay(100);
        self.reset.set_high().unwrap();
    }

}
