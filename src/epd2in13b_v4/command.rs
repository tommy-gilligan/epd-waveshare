//! SPI Commands for the Waveshare 2.13"B V4 E-Ink Display
use crate::traits;

extern crate bit_field;
use bit_field::BitField;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum Command {
    DriverOutputControl = 0x01,
    DeepSleepMode = 0x10,
    DataEntryModeSetting = 0x11,
    SwReset = 0x12,
    TemperatureSensorRead = 0x18,
    ActiveDisplayUpdateSequence = 0x20,
    DisplayUpdateControl = 0x21,
    WriteRam = 0x24,
    WriteRamRed = 0x26,
    BorderWaveformControl = 0x3C,
    SetRamXAddressStartEndPosition = 0x44,
    SetRamYAddressStartEndPosition = 0x45,
    SetRamXAddressCounter = 0x4E,
    SetRamYAddressCounter = 0x4F,
}

impl traits::Command for Command {
    /// Returns the address of the command
    fn address(self) -> u8 {
        self as u8
    }
}

#[allow(dead_code, clippy::enum_variant_names)]
pub(crate) enum DataEntryModeIncr {
    XDecrYDecr = 0x0,
    XIncrYDecr = 0x1,
    XDecrYIncr = 0x2,
    XIncrYIncr = 0x3,
}

#[allow(dead_code)]
pub(crate) enum DataEntryModeDir {
    XDir = 0x0,
    YDir = 0x4,
}

pub(crate) struct DriverOutput {
    pub scan_is_linear: bool,
    pub scan_g0_is_first: bool,
    pub scan_dir_incr: bool,

    pub width: u16,
}

impl DriverOutput {
    pub fn to_bytes(&self) -> [u8; 3] {
        [
            self.width as u8,
            (self.width >> 8) as u8,
            *0u8.set_bit(0, !self.scan_dir_incr)
                .set_bit(1, !self.scan_g0_is_first)
                .set_bit(2, !self.scan_is_linear),
        ]
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum BorderWaveFormVbd {
    Gs = 0x0,
    FixLevel = 0x1,
    Vcom = 0x2,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum BorderWaveFormFixLevel {
    Vss = 0x0,
    Vsh1 = 0x1,
    Vsl = 0x2,
    Vsh2 = 0x3,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum BorderWaveFormGs {
    Lut0 = 0x0,
    Lut1 = 0x1,
    Lut2 = 0x2,
    Lut3 = 0x3,
}

pub(crate) struct BorderWaveForm {
    pub vbd: BorderWaveFormVbd,
    pub fix_level: BorderWaveFormFixLevel,
    pub gs_trans: BorderWaveFormGs,
}

impl BorderWaveForm {
    pub fn to_u8(&self) -> u8 {
        *0u8.set_bits(6..8, self.vbd as u8)
            .set_bits(4..6, self.fix_level as u8)
            .set_bits(0..2, self.gs_trans as u8)
    }
}
