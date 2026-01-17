use cortex_m::{ asm::delay };

use embedded_hal::digital::{ OutputPin, PinState };
use PinState::{ Low, High };

use rp2040_hal::{ self as hal, gpio::AnyPin };
use hal::{
    gpio::{ PinGroup }
};

pub struct DataBus<Output> {
    pub pins: [Output; 8]
}

impl<Output> DataBus<Output>
where
    Output: AnyPin
{
    pub fn new(pins_arg: [Output; 8]) -> Self {
        Self {
            pins: pins_arg
        }
    }

    pub fn as_pg(&mut self) -> PinGroup {
        let n = PinGroup::new();
        for pin in self.pins {
            n.add_pin(pin);
        }
        n
    }
}

/// A device-specific HAL for the YM2149F PSG chip.
pub struct YM2149<BC1, BDIR>
where
    BC1: OutputPin,
    BDIR: OutputPin,
{
    data_bus: DataBus<>,
    master_clock_frequency: u32,
    bc1: BC1,
    bdir: BDIR
}

/// One of the 16 registers (0-15) of the YM2149F sound chip.
///
/// Used to select which register to write / read.
/// Each register controls different aspects of tone generation, noise, mixing,
/// amplitude, and envelope.
///
/// Check the datasheet / docs for detailed information.
#[repr(u8)]
pub enum Register {
    /// Frequency of channel A: 8 bit fine tone adjustment
    AFreq8bitFinetone,
    /// Frequency of channel A: 4 bit rough tone adjustment
    ///
    /// `Mask: 0x0F`
    AFreq4bitRoughtone,

    /// Frequency of channel B: 8 bit fine tone adjustment
    BFreq8bitFinetone,
    /// Frequency of channel B: 4 bit rough tone adjustment
    ///
    /// `Mask: 0x0F`
    BFreq4bitRoughtone,

    /// Frequency of channel C: 8 bit fine tone adjustment
    CFreq8bitFinetone,
    /// Frequency of channel C: 4 bit rough tone adjustment
    ///
    /// `Mask: 0x0F`
    CFreq4bitRoughtone,

    /// Frequency of noise: 5 bit noise frequency
    ///
    /// `Mask: 0x1F`
    NoiseFreq5bit,

    /// **I/O Port and mixer settings**
    ///
    /// From the datasheet:
    /// - Sound is output when '0' is written to the register.
    /// - Selection of input/output for the I/O ports is determined by bits B7 and B6 of register R7.
    /// - Input is selected when '0' is written to the register bits.
    ///
    /// Bit:    | B7  | B6  | B5  | B4  | B3  | B2  | B1  | B0  |
    /// --------|-----|-----|-----|-----|-----|-----|-----|-----|
    /// Type:   | I/O | I/O |Noise|Noise|Noise|Tone |Tone |Tone |
    /// Channel:| IOB | IOA |  C  |  B  |  A  |  C  |  B  |  A  |
    ///
    ///
    /// **Example:**
    /// ```no_run
    /// // Enables only channel A, with IOA and IOB functioning as inputs.
    /// chip.write_register(
    ///     Registers::IoPortMixerSettings,
    ///     0b00111110
    /// );
    /// ```
    IoPortMixerSettings,

    /// **Level of channel A**
    /// ---
    /// **Level control** (formats identical for ALevel, BLevel and CLevel)
    ///
    /// From the datasheet:
    /// - Mode M selects whether the level is fixed (when M = 0) or variable (M = 1).
    /// - When M = 0, the level is determined from one of 16 by level selection signals L3, L2, L1, and L0 which compromise the lower four bits.
    /// - When M = 1, the level is determined by the 5 bit output of E4, E3, E2, E1, and E0 of the envelope generator of the SSG.
    ///
    /// | B7 (MSB)  | B6  | B5  | B4  | B3  | B2  | B1  | B0  |
    /// |-----------|-----|-----|-----|-----|-----|-----|-----|
    /// | N/A       | N/A | N/A |  M  | L3  | L2  | L1  | L0  |
    ALevel,

    /// **Level of channel B**
    ///
    /// Same format as [ALevel](#alevel)
    BLevel,

    /// **Level of channel C**
    ///
    /// Same format as [ALevel](#alevel)
    CLevel,

    /// Frequency of envelope: 8 bit fine adjustment
    EFreq8bitFineAdj,
    /// Frequency of envelope: 8 bit fough adjustment
    EFreq8bitRoughAdj,
    /// Shape of envelope
    EShape,
    /// Data of I/O port A
    DataIoA,
    /// Data of I/O port B
    DataIoB
}

/// The four main modes of the bus control decoder.
pub enum Mode {
    /// DA7~DA0 has high impedance.
    INACTIVE,
    /// DA7~DA0 set to output mode, and contents of register currently being addressed are output.
    READ,
    /// DA7~DA0 set to input mode, and data is written to register currently being addressed.
    WRITE,
    /// DA7~DA0 set to input mode, and address is fetched from register array.
    ADDRESS
}

impl Mode {
    pub const STATES: [(PinState, PinState, PinState); 4] = [
        (Low, High, Low),  // INACTIVE
        (High, Low, Low), // READ
        (Low, Low, High), // WRITE
        (High, High, High), // ADDRESS
    ];

    pub fn pin_states(self) -> &'static (PinState, PinState, PinState) {
        &Self::STATES[self as usize]
    }
}


pub enum AudioChannel {
    A,
    B,
    C
}

impl AudioChannel {
    pub fn tone(&mut self, period: u16) {
        let tp: [u8; 2] = period.to_le_bytes();
        self.write_register(Register::AFreq8bitFinetone, tp[0]); // Fine tone, 8 bits
        self.write_register(Register::AFreq4bitRoughtone, tp[1]); // Rough tone, 4 bits
    }

    pub fn tone_hz(&mut self, channel: AudioChannel, frequency: u32) {
        let tp: [u8; 4] = (self.master_clock_frequency / (16 * frequency)).to_le_bytes();

        self.write_register(Register::AFreq8bitFinetone, tp[0]); // Fine tone, 8 bits
        self.write_register(Register::AFreq4bitRoughtone, tp[1]); // Rough tone, 4 bits
        // The remaining bytes are IGNORED
    }
}

impl <BC1, BDIR>YM2149<BC1, BDIR>
where
    BC1: OutputPin,
    BDIR: OutputPin
{
    pub fn new(data_bus: PinGroup, master_clock_frequency: u32, bc1: BC1, bdir: BDIR) -> Self {
        Self {
            data_bus: data_bus,
            master_clock_frequency: master_clock_frequency,
            bc1,
            bdir
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        let (bdir, _, bc1) = *mode.pin_states();
        self.bdir.set_state(bdir).unwrap();
        self.bc1.set_state(bc1).unwrap();
    }

    pub fn write_register(&mut self, register: Register, value: u8) {
        self.set_mode(Mode::ADDRESS);
        self.data_bus.set_u32(register);
        self.set_mode(Mode::INACTIVE);
        self.set_mode(Mode::WRITE);
        self.write_data_bus(value);
        self.set_mode(Mode::INACTIVE);
    }

    pub fn toneA(&mut self, period: u16) {
        let tp: [u8; 2] = period.to_le_bytes();
        self.write_register(Register::AFreq8bitFinetone, tp[0]); // Fine tone, 8 bits
        self.write_register(Register::AFreq4bitRoughtone, tp[1]); // Rough tone, 4 bits
    }

    pub fn enableA(&mut self) {
        self.write_register(Register::IoPortMixerSettings, 0b00111110);
    }

    pub fn volumeA(&mut self, volume: u8) {
        self.write_register(Register::ALevel, volume & 0x0F);
    }

    pub fn tone_hz(&mut self, channel: AudioChannel, frequency: u32) {
        let tp: [u8; 4] = (self.master_clock_frequency / (16 * frequency)).to_le_bytes();

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
