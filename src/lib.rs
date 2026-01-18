//! RP2040 HAL driver for YM2149 sound chip.
//!
//! # Example
//! See `examples/sweep.rs` for full usage.
#![no_std]
#![no_main]

pub mod ym2149;
#[allow(unused_imports)]
pub use ym2149::*;
