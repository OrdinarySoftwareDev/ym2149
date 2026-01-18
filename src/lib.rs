/*
 *
 *
 *  █████ █████ ██████   ██████  ████████  ████  █████ █████   ████████
 * ░░███ ░░███ ░░██████ ██████  ███░░░░███░░███ ░░███ ░░███   ███░░░░███
 *  ░░███ ███   ░███░█████░███ ░░░    ░███ ░███  ░███  ░███ █░███   ░███
 *   ░░█████    ░███░░███ ░███    ███████  ░███  ░███████████░░█████████
 *    ░░███     ░███ ░░░  ░███   ███░░░░   ░███  ░░░░░░░███░█ ░░░░░░░███
 *     ░███     ░███      ░███  ███      █ ░███        ░███░  ███   ░███
 *     █████    █████     █████░██████████ █████       █████ ░░████████
 *    ░░░░░    ░░░░░     ░░░░░ ░░░░░░░░░░ ░░░░░       ░░░░░   ░░░░░░░░
 *
 *                   (c) vw.dvw 2026, MIT or Apache-2.0
 *
*/

//! RP2040 HAL driver for YM2149 SSG / sound chip.
//!
//! # Example
//! See `examples/*.rs` for full usage.
//!
//! **When in doubt, check the specsheet!**
#![no_std]
#![no_main]
use core::{
    convert::{ From, Into }
};

use embedded_hal::digital::{ OutputPin, PinState };
use PinState::{ High, Low };
use rp2040_hal::gpio::{ DynPinId, FunctionSio, Pin, PullDown, SioOutput};

/// Helper trait that lets you configure any sort of output bus.
/// It abstracts writing 8-bit values to various bus implementations.
///
/// Example:
/// ```no_run
/// use embedded_hal::digital::PinState::{ High, Low };
/// use rp2040_hal::gpio::{ DynPinId, FunctionSio, Pin, PullDown, SioOutput};
///
/// impl OutputBus for DataBus<Pin<DynPinId, FunctionSio<SioOutput>, PullDown>> {
///     fn write_u8(&mut self, data: u8) {
///         for bit in 0..8 {
///             let state = if (data >> bit) & 1 == 1 {
///                 High
///             } else {
///                 Low
///             };
///             let _ = self.pins[bit].set_state(state);
///         }
///     }
/// }
/// ```
pub trait OutputBus {
    fn write_u8(&mut self, data: u8);
}


/// This struct makes an array of length 8 for any type that implements OutputPin.
pub struct DataBus<T> {
    pins: [T; 8]
}

impl<T> DataBus<T>
where
    T: OutputPin,
{
    pub fn new(pins: [T; 8]) -> Self {
        Self { pins }
    }
}

impl OutputBus for DataBus<Pin<DynPinId, FunctionSio<SioOutput>, PullDown>> {
    fn write_u8(&mut self, data: u8) {
        for bit in 0..8 {
            let state = if (data >> bit) & 1 == 1 {
                High
            } else {
                Low
            };
            let _ = self.pins[bit].set_state(state);
        }
    }
}


/// An error related to note parsing.
pub enum NoteParseError {
    InvalidLength,
    InvalidAccidental,
    InvalidNote,
    OctaveOutOfRange
}

/// A YM2149 chip struct.
/// Below is the simplest example code you need to build one:
/// ```no_run
/// // Frequency (in Hz, u32) of the clock the chip is connected to (Pin 22 on the YM2149)
/// let master_clock_freq: u32 = 2_000_000;
///
/// // DynPins for the 8-bit data bus (LSB, pin D0 to MSB, pin D7)
/// let data_pins = [
///     pins.gpio1.into_push_pull_output().into_dyn_pin(),
///     pins.gpio2.into_push_pull_output().into_dyn_pin(),
///     pins.gpio3.into_push_pull_output().into_dyn_pin(),
///     pins.gpio4.into_push_pull_output().into_dyn_pin(),
///     pins.gpio5.into_push_pull_output().into_dyn_pin(),
///     pins.gpio6.into_push_pull_output().into_dyn_pin(),
///     pins.gpio7.into_push_pull_output().into_dyn_pin(),
///     pins.gpio8.into_push_pull_output().into_dyn_pin()
/// ];
/// // Initialize a DataBus
/// let mut data_bus = DataBus::new(data_pins);
/// data_bus.write_u8(0); // Write 0b0000_0000 as a safety measure
///
/// // Bus control decoder pins (BC2 is redundant - connect it to VCC)
/// let bc1 = pins.gpio9.into_push_pull_output();
/// let bdir = pins.gpio10.into_push_pull_output();
///
/// // Build the chip by passing:
/// let mut chip = YM2149::new(
///     data_bus,           // - A variable of type that implements the `OutputBus` trait
///     master_clock_freq,  // - The frequency of the master clock
///     bc1,                // - GPIO pin connected to BC1
///     bdir                // - GPIO pin connected to BDIR
/// );
/// ```
pub struct YM2149<DATABUS, BC1, BDIR>
where
    DATABUS: OutputBus,
    BC1: OutputPin,
    BDIR: OutputPin,
{
    data_bus: DATABUS,
    master_clock_frequency: u32,
    bc1: BC1,
    bdir: BDIR
}

/// One of the 16 registers (0-15) of the YM2149 sound chip.
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
    /// // Enables only channel A, with IOA and IOB functioning as outputs.
    /// chip.write_register(
    ///     Registers::IoPortMixerSettings,
    ///     0b11111110
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
    /// Frequency of envelope: 8 bit rough adjustment
    EFreq8bitRoughAdj,
    /// Shape of envelope
    EShape,
    /// Data of I/O port A
    DataIoA,
    /// Data of I/O port B
    DataIoB
}

impl From<Register> for u8 {
    fn from(value: Register) -> Self {
        value as u8
    }
}

/// The four modes of the bus control decoder.
///
/// Bus control decoder table, no redundancy:
///
/// | Mode         | BDIR | BC2 | BC1 |
/// | ------------ | ---- | --- | --- |
/// | **INACTIVE** |  0   |  1  |  0  |
/// | **READ**     |  0   |  1  |  1  |
/// | **WRITE**    |  1   |  1  |  0  |
/// | **ADDRESS**  |  1   |  1  |  1  |
#[repr(u8)]
pub enum Mode {
    /// DA7~DA0 has high impedance.
    INACTIVE,
    /// DA7~DA0 set to output mode, and contents of register currently being addressed are output.
    ///
    /// ---
    /// ### Warning!
    ///
    /// Mode::READ makes the chip output 5V to the data bus. It is **STRONGLY** recommended
    /// to use a level shifter in order to prevent permanent damage to your board.
    READ,
    /// DA7~DA0 set to input mode, and data is written to register currently being addressed.
    WRITE,
    /// DA7~DA0 set to input mode, and address is fetched from register array.
    ADDRESS
}

impl Mode {
    pub const STATES: [(PinState, PinState, PinState); 4] = [
        (Low, High, Low),  // INACTIVE
        (Low, High, High), // READ
        (High, High, Low), // WRITE
        (High, High, High), // ADDRESS
    ];

    /// Returns an appropriate array of `PinState`s.
    fn pin_states(self) -> (PinState, PinState, PinState) {
        Self::STATES[self as usize]
    }
}

/// One of the 3 analog audio channels (A, B, C) of the YM2149.
#[derive(Debug, Clone, Copy)]
pub enum AudioChannel {
    /// ANALOG CHANNEL A (Pin 4)
    A,
    /// ANALOG CHANNEL B (Pin 3)
    B,
    /// ANALOG CHANNEL C (Pin 38)
    C
}

impl <DATABUS, BC1, BDIR> YM2149<DATABUS, BC1, BDIR>
where
    DATABUS: OutputBus,
    BC1: OutputPin,
    BDIR: OutputPin
{
    /// Create a new struct for the YM2149.
    pub fn new(data_bus: DATABUS, master_clock_frequency: u32, bc1: BC1, bdir: BDIR) -> Self {
        Self {
            data_bus,
            master_clock_frequency,
            bc1,
            bdir
        }
    }

    /// Set the [mode](#Mode) of the chip.
    ///
    /// Example:
    /// ```no_run
    /// // Build the chip by passing:
    /// let mut chip = YM2149::new(
    ///     data_bus,           // - A variable of type that implements the `OutputBus` trait
    ///     master_clock_freq,  // - The frequency of the master clock
    ///     bc1,                // - The GPIO pin connected to BC1
    ///     bdir                // - The GPIO pin connected to BDIR
    /// );
    ///
    /// // Set the chip's mode to `Inactive`
    /// chip.set_mode(Mode::INACTIVE);
    /// ```
    pub fn set_mode(&mut self, mode: Mode) {
        let (bdir, _, bc1) = mode.pin_states();
        self.bdir.set_state(bdir).unwrap();
        self.bc1.set_state(bc1).unwrap();
    }

    /// Write to one of the chip's 16 registers.
    /// You can pass either a [YM2149::Register](#Register) or u8 for this purpose.
    ///
    /// The `register` parameter should be in the range of `0..15`.
    /// In case it isn't, the compiler will warn you of this and
    /// its' value will be clamped by the following line:
    /// ```no_run
    /// let r: u8 = register.into().clamp(0, 15);
    /// ```
    /// Example:
    /// ```no_run
    /// // Configure the mixer according to the datasheet
    /// chip.write_register(Register::IoPortMixerSettings, 0b11111110);
    /// ```
    pub fn write_register<T: Into<u8>>(&mut self, register: T, value: u8) {
        let r: u8 = register.into().clamp(0, 15);

        self.set_mode(Mode::ADDRESS);
        self.data_bus.write_u8(r);
        self.set_mode(Mode::INACTIVE);
        self.set_mode(Mode::WRITE);
        self.data_bus.write_u8(value);
        self.set_mode(Mode::INACTIVE);
    }

    /// Play a tone with a TP of `period` on an [AudioChannel](#AudioChannel).
    ///
    /// The formula for the frequency is
    /// ``f = fMaster / (16 * TP)``, where:
    ///     - f: target frequency
    ///     - fMaster: master clock frequency
    ///     - TP: tone period
    pub fn tone(&mut self, channel: AudioChannel, period: u16) {
        let bytes: [u8; 2] = period.to_le_bytes();
        let register_pair_index = channel as u8 * 2;

        self.write_register(register_pair_index, bytes[0]); // Fine tone, 8 bits
        self.write_register(register_pair_index + 1, bytes[1]); // Rough tone, 4 bits
    }

    /// Play a tone of a given frequency in Hz on an [AudioChannel](#AudioChannel).
    pub fn tone_hz(&mut self, channel: AudioChannel, frequency: u32) {
        let tp: u32 = self.master_clock_frequency / (16 * frequency);
        self.tone(channel, tp as u16); // Take lowest 16 bits
    }

    /// Set the frequency of the noise generator.
    pub fn set_noise_freq(&mut self, frequency: u8) {
        self.write_register(6, frequency & 0x1F);
    }

    /// Set the volume of an [AudioChannel](#AudioChannel).
    ///
    /// **Note:** The channel level registers store 5 bits of data per channel.
    ///
    /// ---
    ///
    /// From the datasheet:
    /// - Mode M selects whether the level is fixed (when M = 0) or variable (M = 1).
    /// - When M = 0, the level is determined from one of 16 by level selection signals L3, L2, L1, and L0 which compromise the lower four bits.
    /// - When M = 1, the level is determined by the 5 bit output of E4, E3, E2, E1, and E0 of the envelope generator of the SSG.
    ///
    /// | B7 (MSB)  | B6  | B5  | B4  | B3  | B2  | B1  | B0  |
    /// |-----------|-----|-----|-----|-----|-----|-----|-----|
    /// | N/A       | N/A | N/A |  M  | L3  | L2  | L1  | L0  |
    pub fn volume(&mut self, channel: AudioChannel, volume: u8) {
        self.write_register(8 + channel as u8, volume & 0x1F);
    }



    // ============================================================
    // ========================= THE VOID =========================
    // ============================================================
    // (All you'll find here is unimplemented / todo functionality)

    #[allow(unused)]
    /// Reads a value from a given register and outputs it to the data bus.
    ///
    /// ---
    /// # Warning!
    ///
    /// Mode::READ makes the chip output 5V to the data bus. It is **STRONGLY** recommended
    /// to use a level shifter in order to prevent permanent damage to your board.
    ///
    /// This method is **unimplemented** *(at least, not for now...)*
    ///
    /// Feel free to try implementing it yourself, at your own risk.
    fn read(&mut self, register: Register) -> u8 {
        unimplemented!("Mode::READ and .read() are not yet usable.");
    }

    #[allow(unused)]
    /// Play a note `(note_s: &'static str)` on an [AudioChannel](#AudioChannel).
    fn note(&mut self, channel: AudioChannel, note_s: &'static str) -> Result<(), NoteParseError> {
        todo!(".note() is not yet implemented: Unfinished code");
        // Code below is unreachable
        if note_s.len() < 2 || note_s.len() > 3 { return Err(NoteParseError::InvalidLength); }
        let semitones_from_a4: u32; // NOTE TO SELF: f = f0 * 2 ^ (n / 12) | f0 - reference pitch, n - semitones away from ref.
        Ok(())
    }

    // TODO: Envelope & I/O control
}
