use core::{ops::BitXor, u8};

const REFERENCE_PITCH: f32 = 440.0;

/// One of the 3 analog audio channels (A, B, C) of the YM2149.
#[derive(Debug, Clone, Copy)]
pub enum AudioChannel {
    /// ANALOG CHANNEL A (Pin 4)
    A,
    /// ANALOG CHANNEL B (Pin 3)
    B,
    /// ANALOG CHANNEL C (Pin 38)
    C,
}

#[derive(Debug, Clone, Copy)]
#[repr(i8)]
pub enum BaseNote {
    C = -9,
    D = -7,
    E = -5,
    F = -4,
    G = -2,
    A = 0,
    B = 2,
}

impl From<BaseNote> for f32 {
    fn from(bn: BaseNote) -> f32 { bn as i8 as f32 }
}

#[derive(Debug, Clone, Copy)]
#[repr(i8)]
pub enum Accidental {
    Natural = 0,
    Sharp = 2,
    Flat = -2,
    MicroSharp = 1,
    MicroFlat = -1,
}

impl From<Accidental> for f32 {
    fn from(acc: Accidental) -> f32 { (acc as i8) as f32 / 2.0 }
}

/// A musical note.
#[derive(Debug, Clone, Copy)]
pub struct Note {
    base_note: BaseNote,
    octave: u8,
    accidental: Option<Accidental>,
    offset: f32,
}

impl Note {
    pub fn new(base_note: BaseNote, octave: u8, accidental: Option<Accidental>) -> Self {
        Self {
            base_note: base_note,
            octave: octave,
            accidental: accidental,
            offset: 0.0,
        }
    }

    pub fn transpose(self, semitones: f32) -> Self {
        Self {
            offset: semitones,
            ..self
        }
    }

    pub fn as_hz(&self) -> u32 {
        // NOTE TO SELF: f = f0 * 2 ^ (n / 12) | f0 - reference pitch, n - semitones away from ref.
        use libm::{powf, roundf};

        let distance_a4: f32 =
            f32::from(self.base_note)
            + f32::from(self.accidental.unwrap_or(Accidental::Natural))
            + (self.octave.clamp(0, 14) as f32 - 4.0) * 12.0
            + self.offset;

        roundf(REFERENCE_PITCH * powf(2.0, distance_a4 / 12.0)) as u32
    }
}


/// A helper enum for setting the envelope repetition frequency f_e.
#[derive(Debug, Clone, Copy)]
pub enum EnvelopeFrequency {
    Hertz(u16),
    BeatsPerMinute(u16),
    Integer(u16),
}

impl EnvelopeFrequency {
    pub fn as_ep(self, master_clock_frequency: u32) -> u16 {
        match self {
            Self::Hertz(f_e) => master_clock_frequency.checked_div(256 * (f_e as u32)).unwrap_or(1) as u16,
            Self::BeatsPerMinute(bpm) => 60 * Self::Hertz(bpm).as_ep(master_clock_frequency),
            Self::Integer(x) => x
        }
    }
}

/// A helper enum for setting the envelope's shape.
///
/// To invert the shape use EnvelopeShape::invert().
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BuiltinEnvelopeShape {
    /// Fade out and hold low
    FadeOut = 0b00001001,
    /// Fade in and hold high
    FadeIn = 0b00001101,
    /// Fade in then hold low
    Tooth = 0b00001111,
    /// Fade in every repetition
    Saw = 0b00001100,
    /// Alternate between fade out and fade in
    Triangle = 0b00001110,
}

pub struct RawEnvelope {
    data: [u8; 4096],
    length_beats: u8,
}

impl RawEnvelope {
    fn invert(&mut self) {
        for i in 0..4096 {
            self.data[i] = 0xF - self.data[i];
        }
    }

    fn scale(&mut self, scale: f32) {
        for i in 0..4096 {
            let scaled = (self.data[i] as f32 * scale).clamp(0.0, 255.0) as u8;
            self.data[i] = scaled;
        }
    }

    fn offset(&mut self, offset: i8) {
        for i in 0..4096 {
            self.data[i] += offset as u8;
        }
    }
}

pub enum EnvelopeShape {
    Builtin(BuiltinEnvelopeShape),
    InvertedBuiltin(BuiltinEnvelopeShape),
    RawEnvelope(RawEnvelope),
    CustomU8(u8),
}

impl From<EnvelopeShape> for u8 {
    fn from(value: EnvelopeShape) -> Self {
        use EnvelopeShape::*;

        match value {
            Builtin(builtin) => builtin as u8,
            InvertedBuiltin(builtin) => (builtin as u8).bitxor(0b00000100),
            RawEnvelope(raw) => todo!(),
            CustomU8(n) => n,
        }
    }
}
