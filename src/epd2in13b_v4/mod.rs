//! A simple Drlever for the Waveshare 2.13" B V4 E-Ink Display via SPI
//! More information on this display can be found at the [Waveshare Wiki](https://www.waveshare.com/wiki/Pico-ePaper-2.13-B)
//! This driver was build and tested for 250x122, 2.13inch E-Ink display HAT for Raspberry Pi, three-color, SPI interface
//!
//! # Example for the 2.13" E-Ink Display
//!
//!```rust, no_run
//!# use embedded_hal_mock::*;
//!# fn main() -> Result<(), MockError> {
//!use embedded_graphics::{prelude::*, primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder}};
//!use epd_waveshare::{epd2in13b_v4::*, prelude::*};
//!#
//!# let expectations = [];
//!# let mut spi = spi::Mock::new(&expectations);
//!# let expectations = [];
//!# let cs_pin = pin::Mock::new(&expectations);
//!# let busy_in = pin::Mock::new(&expectations);
//!# let dc = pin::Mock::new(&expectations);
//!# let rst = pin::Mock::new(&expectations);
//!# let mut delay = delay::MockNoop::new();
//!
//!// Setup EPD
//!let mut epd = Epd2in13b::new(&mut spi, cs_pin, busy_in, dc, rst, &mut delay, None)?;
//!
//!// Use display graphics from embedded-graphics
//!// This display is for the black/white/chromatic pixels
//!let mut tricolor_display = Display2in13b::default();
//!
//!// Use embedded graphics for drawing a black line
//!let _ = Line::new(Point::new(0, 120), Point::new(0, 200))
//!    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
//!    .draw(&mut tricolor_display);
//!
//!// We use `chromatic` but it will be shown as red/yellow
//!let _ = Line::new(Point::new(15, 120), Point::new(15, 200))
//!    .into_styled(PrimitiveStyle::with_stroke(TriColor::Chromatic, 1))
//!    .draw(&mut tricolor_display);
//!
//!// Display updated frame
//!epd.update_color_frame(
//!    &mut spi,
//!    &mut delay,
//!    &tricolor_display.bw_buffer(),
//!    &tricolor_display.chromatic_buffer()
//!)?;
//!epd.display_frame(&mut spi, &mut delay)?;
//!
//!// Set the EPD to sleep
//!epd.sleep(&mut spi, &mut delay)?;
//!# Ok(())
//!# }
//!```
// Original Waveforms from Waveshare
use embedded_hal::{
    blocking::{delay::*, spi::Write},
    digital::v2::{InputPin, OutputPin},
};

use crate::buffer_len;
use crate::color::TriColor;
use crate::interface::DisplayInterface;
use crate::traits::{ InternalWiAdditions, RefreshLut, WaveshareDisplay, WaveshareThreeColorDisplay, };

pub(crate) mod command;
use self::command::{
    BorderWaveForm, BorderWaveFormFixLevel, BorderWaveFormGs, BorderWaveFormVbd, Command,
    DataEntryModeDir, DataEntryModeIncr, DeepSleepMode, DisplayUpdateControl2, DriverOutput,
    GateDrivingVoltage, I32Ext, SourceDrivingVoltage, Vcom,
};

pub(crate) mod constants;
use self::constants::{LUT_FULL_UPDATE, LUT_PARTIAL_UPDATE};

/// Full size buffer for use with the 2.13" v4 EPD
#[cfg(feature = "graphics")]
pub type Display2in13b = crate::graphics::Display<
    WIDTH,
    HEIGHT,
    false,
    { buffer_len(WIDTH as usize, HEIGHT as usize) * 2 },
    TriColor,
>;

/// Width of the display.
pub const WIDTH: u32 = 122;

/// Height of the display
pub const HEIGHT: u32 = 250;

/// Default Background Color
pub const DEFAULT_BACKGROUND_COLOR: TriColor = TriColor::White;
const IS_BUSY_LOW: bool = false;

/// Epd2in13b (V4) driver
///
pub struct Epd2in13b<SPI, CS, BUSY, DC, RST, DELAY> {
    /// Connection Interface
    interface: DisplayInterface<SPI, CS, BUSY, DC, RST, DELAY>,

    /// Background Color
    background_color: TriColor,
}

impl<SPI, CS, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd2in13b<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // HW reset
        self.interface.reset(delay, 10_000, 10_000);

        self.wait_until_idle(spi, delay)?;
        self.interface.cmd(spi, Command::SwReset)?;
        self.wait_until_idle(spi, delay)?;

        self.set_driver_output(
            spi,
            DriverOutput {
                scan_is_linear: true,
                scan_g0_is_first: true,
                scan_dir_incr: true,
                width: (HEIGHT - 1) as u16,
            },
        )?;

        // These 2 are the reset values
        // self.set_dummy_line_period(spi, 0x30)?;
        // self.set_gate_scan_start_position(spi, 0)?;

        self.set_data_entry_mode(spi, DataEntryModeIncr::XIncrYIncr, DataEntryModeDir::XDir)?;

        // Use simple X/Y auto increase
        self.set_ram_area(spi, 0, 0, WIDTH - 1, HEIGHT - 1)?;
        self.set_ram_address_counters(spi, delay, 0, 0)?;

        // self.interface
        //     .cmd_with_data(spi, Command::BorderWaveform, &[0x05])?;
        //     the following evaluates to 0x03 i'm pretty sure, not sure if that's what we really
        //     want
        self.set_border_waveform(
            spi,
            command::BorderWaveForm {
                vbd: BorderWaveFormVbd::Gs,
                fix_level: BorderWaveFormFixLevel::Vss,
                gs_trans: BorderWaveFormGs::Lut3,
            },
        )?;
        // self.set_vcom_register(spi, (-21).vcom())?;

        // self.set_gate_driving_voltage(spi, 190.gate_driving_decivolt())?;
        // self.set_source_driving_voltage(
        //     spi,
        //     150.source_driving_decivolt(),
        //     50.source_driving_decivolt(),
        //     (-150).source_driving_decivolt(),
        // )?;

        // self.set_gate_line_width(spi, 10)?;

        // self.set_lut(spi, delay, Some(self.refresh))?;

        self.interface
            .cmd_with_data(spi, Command::TemperatureSensorRead, &[0x80])?;

        self.interface
            .cmd_with_data(spi, Command::DisplayUpdateControl1, &[0x80, 0x80])?;

        self.wait_until_idle(spi, delay)?;

        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareThreeColorDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd2in13b<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{

    fn update_color_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        black: &[u8],
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.update_achromatic_frame(spi, delay, black)?;
        self.update_chromatic_frame(spi, delay, chromatic)
    }

    /// Update only the black/white data of the display.
    ///
    /// Finish by calling `update_chromatic_frame`.
    fn update_achromatic_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        black: &[u8],
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.interface.cmd(spi, Command::WriteRam)?;
        self.interface.data(spi, black)?;
        Ok(())
    }

    /// Update only chromatic data of the display.
    ///
    /// This data takes precedence over the black/white data.
    fn update_chromatic_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.interface.cmd(spi, Command::WriteRamRed)?;
        self.interface.data(spi, chromatic)?;

        self.wait_until_idle(spi, delay)?;
        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd2in13b<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    type DisplayColor = TriColor;
    fn new(
        spi: &mut SPI,
        cs: CS,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self, SPI::Error> {
        let mut epd = Epd2in13b {
            interface: DisplayInterface::new(cs, busy, dc, rst, delay_us),
            background_color: DEFAULT_BACKGROUND_COLOR,
        };

        epd.init(spi, delay)?;
        Ok(epd)
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn sleep(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;

        // All sample code enables and disables analog/clocks...
        self.set_display_update_control_2(
            spi,
            DisplayUpdateControl2::new()
                .enable_analog()
                .enable_clock()
                .disable_analog()
                .disable_clock(),
        )?;
        self.command(spi, Command::MasterActivation)?;

        self.set_sleep_mode(spi, DeepSleepMode::Normal)?;
        Ok(())
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        assert!(buffer.len() == buffer_len(WIDTH as usize, HEIGHT as usize));
        self.set_ram_area(spi, 0, 0, WIDTH - 1, HEIGHT - 1)?;
        self.set_ram_address_counters(spi, delay, 0, 0)?;

        self.cmd_with_data(spi, Command::WriteRam, buffer)?;

        if true {
            // Always keep the base buffer equal to current if not doing partial refresh.
            self.set_ram_area(spi, 0, 0, WIDTH - 1, HEIGHT - 1)?;
            self.set_ram_address_counters(spi, delay, 0, 0)?;

            self.command(spi, Command::WriteRamRed)?;
            self.interface.data_x_times(
                spi,
                self.background_color.get_byte_value(),
                buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
            )?;
        }
        Ok(())
    }

    /// Updating only a part of the frame is not supported when using the
    /// partial refresh feature. The function will panic if called when set to
    /// use partial refresh.
    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        assert!((width * height / 8) as usize == buffer.len());

        // This should not be used when doing partial refresh. The RAM_RED must
        // be updated with the last buffer having been displayed. Doing partial
        // update directly in RAM makes this update impossible (we can't read
        // RAM content). Using this function will most probably make the actual
        // display incorrect as the controler will compare with something
        // incorrect.
        assert!(true);

        self.set_ram_area(spi, x, y, x + width, y + height)?;
        self.set_ram_address_counters(spi, delay, x, y)?;

        self.cmd_with_data(spi, Command::WriteRam, buffer)?;

        if true {
            // Always keep the base buffer equals to current if not doing partial refresh.
            self.set_ram_area(spi, x, y, x + width, y + height)?;
            self.set_ram_address_counters(spi, delay, x, y)?;

            self.cmd_with_data(spi, Command::WriteRamRed, buffer)?;
        }

        Ok(())
    }

    /// Never use directly this function when using partial refresh, or also
    /// keep the base buffer in syncd using `set_partial_base_buffer` function.
    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
            self.set_display_update_control_2(
                spi,
                DisplayUpdateControl2::new()
                    .enable_clock()
                    .enable_analog()
                    .load_lut()
                    .load_temp()
                    .display()
                    .disable_analog()
                    .disable_clock(),
            )?;
        self.command(spi, Command::MasterActivation)?;
        self.wait_until_idle(spi, delay)?;

        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)?;
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        let color = self.background_color.get_byte_value();

        self.set_ram_area(spi, 0, 0, WIDTH - 1, HEIGHT - 1)?;
        self.set_ram_address_counters(spi, delay, 0, 0)?;

        self.command(spi, Command::WriteRam)?;
        self.interface.data_x_times(
            spi,
            color,
            buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
        )?;

        // Always keep the base buffer equals to current if not doing partial refresh.
        if true {
            self.set_ram_area(spi, 0, 0, WIDTH - 1, HEIGHT - 1)?;
            self.set_ram_address_counters(spi, delay, 0, 0)?;

            self.command(spi, Command::WriteRamRed)?;
            self.interface.data_x_times(
                spi,
                color,
                buffer_len(WIDTH as usize, HEIGHT as usize) as u32,
            )?;
        }
        Ok(())
    }

    fn set_background_color(&mut self, background_color: TriColor) {
        self.background_color = background_color;
    }

    fn background_color(&self) -> &TriColor {
        &self.background_color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn set_lut(
        &mut self,
        spi: &mut SPI,
        _delay: &mut DELAY,
        refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        let buffer = match refresh_rate {
            Some(RefreshLut::Full) | None => &LUT_FULL_UPDATE,
            Some(RefreshLut::Quick) => &LUT_PARTIAL_UPDATE,
        };

        self.cmd_with_data(spi, Command::WriteLutRegister, buffer)
    }

    fn wait_until_idle(&mut self, _spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.wait_until_idle(delay, IS_BUSY_LOW);
        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> Epd2in13b<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    fn set_gate_scan_start_position(
        &mut self,
        spi: &mut SPI,
        start: u16,
    ) -> Result<(), SPI::Error> {
        assert!(start <= 295);
        self.cmd_with_data(
            spi,
            Command::GateScanStartPosition,
            &[(start & 0xFF) as u8, ((start >> 8) & 0x1) as u8],
        )
    }

    fn set_border_waveform(
        &mut self,
        spi: &mut SPI,
        borderwaveform: BorderWaveForm,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(
            spi,
            Command::BorderWaveformControl,
            &[borderwaveform.to_u8()],
        )
    }

    fn set_vcom_register(&mut self, spi: &mut SPI, vcom: Vcom) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::WriteVcomRegister, &[vcom.0])
    }

    fn set_gate_driving_voltage(
        &mut self,
        spi: &mut SPI,
        voltage: GateDrivingVoltage,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::GateDrivingVoltageCtrl, &[voltage.0])
    }

    fn set_dummy_line_period(
        &mut self,
        spi: &mut SPI,
        number_of_lines: u8,
    ) -> Result<(), SPI::Error> {
        assert!(number_of_lines <= 127);
        self.cmd_with_data(spi, Command::SetDummyLinePeriod, &[number_of_lines])
    }

    fn set_gate_line_width(&mut self, spi: &mut SPI, width: u8) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::SetGateLineWidth, &[width & 0x0F])
    }

    /// Sets the source driving voltage value
    fn set_source_driving_voltage(
        &mut self,
        spi: &mut SPI,
        vsh1: SourceDrivingVoltage,
        vsh2: SourceDrivingVoltage,
        vsl: SourceDrivingVoltage,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(
            spi,
            Command::SourceDrivingVoltageCtrl,
            &[vsh1.0, vsh2.0, vsl.0],
        )
    }

    /// Prepare the actions that the next master activation command will
    /// trigger.
    fn set_display_update_control_2(
        &mut self,
        spi: &mut SPI,
        value: DisplayUpdateControl2,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DisplayUpdateControl2, &[value.0])
    }

    /// Triggers the deep sleep mode
    fn set_sleep_mode(&mut self, spi: &mut SPI, mode: DeepSleepMode) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DeepSleepMode, &[mode as u8])
    }

    fn set_driver_output(&mut self, spi: &mut SPI, output: DriverOutput) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DriverOutputControl, &output.to_bytes())
    }

    /// Sets the data entry mode (ie. how X and Y positions changes when writing
    /// data to RAM)
    fn set_data_entry_mode(
        &mut self,
        spi: &mut SPI,
        counter_incr_mode: DataEntryModeIncr,
        counter_direction: DataEntryModeDir,
    ) -> Result<(), SPI::Error> {
        let mode = counter_incr_mode as u8 | counter_direction as u8;
        self.cmd_with_data(spi, Command::DataEntryModeSetting, &[mode])
    }

    /// Sets both X and Y pixels ranges
    fn set_ram_area(
        &mut self,
        spi: &mut SPI,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(
            spi,
            Command::SetRamXAddressStartEndPosition,
            &[(start_x >> 3) as u8, (end_x >> 3) as u8],
        )?;

        self.cmd_with_data(
            spi,
            Command::SetRamYAddressStartEndPosition,
            &[
                start_y as u8,
                (start_y >> 8) as u8,
                end_y as u8,
                (end_y >> 8) as u8,
            ],
        )
    }

    /// Sets both X and Y pixels counters when writing data to RAM
    fn set_ram_address_counters(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        x: u32,
        y: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.cmd_with_data(spi, Command::SetRamXAddressCounter, &[(x >> 3) as u8])?;

        self.cmd_with_data(
            spi,
            Command::SetRamYAddressCounter,
            &[y as u8, (y >> 8) as u8],
        )?;
        Ok(())
    }

    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn cmd_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epd_size() {
        assert_eq!(WIDTH, 122);
        assert_eq!(HEIGHT, 250);
        assert_eq!(DEFAULT_BACKGROUND_COLOR, TriColor::White);
    }
}
