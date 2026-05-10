//! ADS1299 ADC Driver — Production Implementation
//!
//! 8-channel 24-bit ADC at 250 SPS via SPI3.
//! TI datasheet: SBAS499

use crate::config;
use crate::platform::spi::{SpiController, SPI3_CONTROLLER};
use crate::platform::gpio::{GpioPin, GPIO_PA0};

/// ADS1299 register addresses
const ADS1299_REG_ID: u8 = 0x00;
const ADS1299_REG_CONFIG1: u8 = 0x01;
const ADS1299_REG_CONFIG2: u8 = 0x02;
const ADS1299_REG_CONFIG3: u8 = 0x03;
const ADS1299_REG_LOFF: u8 = 0x04;
const ADS1299_REG_CH1SET: u8 = 0x05;
const ADS1299_REG_CH2SET: u8 = 0x06;
const ADS1299_REG_CH3SET: u8 = 0x07;
const ADS1299_REG_CH4SET: u8 = 0x08;
const ADS1299_REG_CH5SET: u8 = 0x09;
const ADS1299_REG_CH6SET: u8 = 0x0A;
const ADS1299_REG_CH7SET: u8 = 0x0B;
const ADS1299_REG_CH8SET: u8 = 0x0C;
const ADS1299_REG_BIAS_SENSP: u8 = 0x0D;
const ADS1299_REG_BIAS_SENSN: u8 = 0x0E;
const ADS1299_REG_LOFF_SENSP: u8 = 0x0F;
const ADS1299_REG_LOFF_SENSN: u8 = 0x10;
const ADS1299_REG_LOFF_FLIP: u8 = 0x11;
const ADS1299_REG_LOFF_STATP: u8 = 0x12;
const ADS1299_REG_LOFF_STATN: u8 = 0x13;
const ADS1299_REG_GPIO: u8 = 0x14;
const ADS1299_REG_MISC1: u8 = 0x15;
const ADS1299_REG_MISC2: u8 = 0x16;
const ADS1299_REG_CONFIG4: u8 = 0x17;

/// ADS1299 commands
const ADS1299_CMD_WAKEUP: u8 = 0x02;
const ADS1299_CMD_STANDBY: u8 = 0x04;
const ADS1299_CMD_RESET: u8 = 0x06;
const ADS1299_CMD_START: u8 = 0x08;
const ADS1299_CMD_STOP: u8 = 0x0A;
const ADS1299_CMD_RDATAC: u8 = 0x10; // Read data continuous
const ADS1299_CMD_SDATAC: u8 = 0x11; // Stop read data continuous
const ADS1299_CMD_RDATA: u8 = 0x12;  // Read data

/// ADS1299 configuration
pub struct Ads1299Config {
    /// Sampling rate [SPS]
    pub sampling_rate: u32,
    /// Number of channels (1-8)
    pub channels: u8,
    /// PGA gain
    pub gain: Ads1299Gain,
    /// Input mode
    pub input_mode: Ads1299InputMode,
    /// Enable bias drive
    pub bias_drive: bool,
    /// Lead-off detection
    pub lead_off: bool,
}

/// PGA gain settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ads1299Gain {
    Gain1 = 0b000,
    Gain2 = 0b001,
    Gain4 = 0b010,
    Gain6 = 0b011,
    Gain8 = 0b100,
    Gain12 = 0b101,
    Gain24 = 0b110,
}

/// Input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ads1299InputMode {
    Normal,
    Shorted,
    TestSignal,
}

impl Default for Ads1299Config {
    fn default() -> Self {
        Self {
            sampling_rate: 250,
            channels: 8,
            gain: Ads1299Gain::Gain24,
            input_mode: Ads1299InputMode::Normal,
            bias_drive: true,
            lead_off: false,
        }
    }
}

/// ADS1299 driver state
pub struct Ads1299 {
    config: Ads1299Config,
    /// Double-buffered sample data
    /// Buffer 0: active reading, Buffer 1: processing
    buffer: [[u8; config::SAMPLE_FRAME_BYTES]; 2],
    /// Active buffer index (0 or 1)
    active_buffer: usize,
    /// Sample counter
    sample_count: u64,
    /// Error counter
    error_count: u32,
    /// Last status byte
    last_status: u8,
}

/// ADC status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ads1299Status {
    Idle,
    Converting,
    Error,
}

impl Ads1299 {
    /// Create new ADS1299 driver
    pub fn new(config: Ads1299Config) -> Self {
        Self {
            config,
            buffer: [[0u8; config::SAMPLE_FRAME_BYTES]; 2],
            active_buffer: 0,
            sample_count: 0,
            error_count: 0,
            last_status: 0,
        }
    }

    /// Initialize ADS1299 with full configuration
    ///
    /// Sequence:
    /// 1. Reset
    /// 2. Stop continuous read
    /// 3. Configure registers
    /// 4. Start continuous read
    /// 5. Start conversion
    pub fn init(&mut self) {
        // 1. Reset
        self.send_command(ADS1299_CMD_RESET);

        // Wait for reset (18 tCLK)
        cortex_m::asm::delay(100);

        // 2. Stop continuous read mode
        self.send_command(ADS1299_CMD_SDATAC);
        cortex_m::asm::delay(10);

        // 3. Configure registers
        self.configure_registers();

        // 4. Start continuous read mode
        self.send_command(ADS1299_CMD_RDATAC);
        cortex_m::asm::delay(10);
    }

    /// Configure all ADS1299 registers
    fn configure_registers(&mut self) {
        // CONFIG1: Data rate
        // DR[2:0] = 110 (250 SPS)
        self.write_register(ADS1299_REG_CONFIG1, 0x96);

        // CONFIG2: Test signals
        // Default: internal reference
        self.write_register(ADS1299_REG_CONFIG2, 0xC0);

        // CONFIG3: Reference and bias
        // PD_REFBUF = 1, BIAS_MEAS = 0, BIASREF_INT = 1, PD_BIAS = 1
        self.write_register(ADS1299_REG_CONFIG3, 0xEC);

        // Configure channels
        let chset = (self.config.gain as u8) << 4;
        for ch in 0..self.config.channels {
            self.write_register(ADS1299_REG_CH1SET + ch, chset | 0x01); // Power up, normal input
        }

        // Power down unused channels
        for ch in self.config.channels..8 {
            self.write_register(ADS1299_REG_CH1SET + ch, 0x81); // Power down
        }

        // BIAS_SENSP/BIAS_SENSN: Enable bias on all active channels
        if self.config.bias_drive {
            let bias_mask = (1 << self.config.channels) - 1;
            self.write_register(ADS1299_REG_BIAS_SENSP, bias_mask);
            self.write_register(ADS1299_REG_BIAS_SENSN, bias_mask);
        }

        // CONFIG4: Lead-off detection
        let config4 = if self.config.lead_off { 0x02 } else { 0x00 };
        self.write_register(ADS1299_REG_CONFIG4, config4);
    }

    /// Send command byte
    fn send_command(&self, cmd: u8) {
        SPI3_CONTROLLER.cs_low();
        cortex_m::asm::delay(2); // t_CSSC

        SPI3_CONTROLLER.send_command(cmd as u16);

        cortex_m::asm::delay(2); // t_SCCS
        SPI3_CONTROLLER.cs_high();
    }

    /// Write register
    fn write_register(&self, reg: u8, value: u8) {
        SPI3_CONTROLLER.cs_low();
        cortex_m::asm::delay(2);

        // WREG command: 0x40 | reg, then number of registers - 1
        SPI3_CONTROLLER.send_command((0x40 | reg) as u16);
        SPI3_CONTROLLER.send_command(0x00); // 1 register
        SPI3_CONTROLLER.send_command(value as u16);

        cortex_m::asm::delay(2);
        SPI3_CONTROLLER.cs_high();

        // t_SCCS = 2 tCLK minimum
        cortex_m::asm::delay(4);
    }

    /// Read register
    fn read_register(&self, reg: u8) -> u8 {
        SPI3_CONTROLLER.cs_low();
        cortex_m::asm::delay(2);

        // RREG command: 0x20 | reg, then number of registers - 1
        SPI3_CONTROLLER.send_command((0x20 | reg) as u16);
        SPI3_CONTROLLER.send_command(0x00); // 1 register

        // Read dummy byte (first byte after command is don't care)
        let _ = SPI3_CONTROLLER.transfer(0x00);

        // Read actual register value
        let value = SPI3_CONTROLLER.transfer(0x00) as u8;

        cortex_m::asm::delay(2);
        SPI3_CONTROLLER.cs_high();

        value
    }

    /// Start continuous conversion
    pub fn start_conversion(&self) {
        self.send_command(ADS1299_CMD_START);
    }

    /// Stop conversion
    pub fn stop_conversion(&self) {
        self.send_command(ADS1299_CMD_STOP);
    }

    /// Read sample frame (called from DMA interrupt handler)
    ///
    /// Returns reference to completed buffer
    pub fn read_frame(&mut self) -> &[u8] {
        // Switch to other buffer for next DMA transfer
        self.active_buffer = 1 - self.active_buffer;
        &self.buffer[1 - self.active_buffer]
    }

    /// Convert raw bytes to i32 samples
    ///
    /// ADS1299 output: 24-bit two's complement, MSB first
    pub fn decode_frame(frame: &[u8]) -> [i32; 8] {
        let mut samples = [0i32; 8];

        // First 3 bytes are status (optional)
        // Then 8 channels × 3 bytes each
        let offset = 3; // Skip status bytes

        for ch in 0..8 {
            let idx = offset + ch * 3;
            if idx + 2 < frame.len() {
                // 24-bit two's complement, big-endian
                samples[ch] = ((frame[idx] as i32) << 16)
                              | ((frame[idx + 1] as i32) << 8)
                              | (frame[idx + 2] as i32);

                // Sign extend if negative (bit 23 set)
                if samples[ch] & 0x800000 != 0 {
                    samples[ch] |= !0xFFFFFF;
                }
            }
        }

        samples
    }

    /// Decode with status byte check
    pub fn decode_frame_with_status(frame: &[u8]) -> ([i32; 8], u8) {
        let samples = Self::decode_frame(frame);
        let status = if !frame.is_empty() { frame[0] } else { 0 };
        (samples, status)
    }

    /// Handle DMA half-transfer interrupt
    ///
    /// Called when first half of buffer is full
    pub fn handle_half_transfer(&mut self) {
        // Buffer 0 is ready for processing
        self.sample_count += 1;
    }

    /// Handle DMA full-transfer interrupt
    ///
    /// Called when entire buffer is full
    pub fn handle_full_transfer(&mut self) {
        // Buffer 1 is ready for processing
        // Switch buffers
        self.active_buffer = 1 - self.active_buffer;
        self.sample_count += 1;
    }

    /// Get sample count
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Get error count
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Check if lead-off detected
    pub fn lead_off_detected(&self) -> bool {
        (self.last_status & 0x04) != 0
    }

    /// Get active buffer address for DMA
    pub fn buffer_address(&self) -> u32 {
        self.buffer[self.active_buffer].as_ptr() as u32
    }

    /// Get buffer size in 32-bit words
    pub fn buffer_size_words(&self) -> u16 {
        (config::SAMPLE_FRAME_BYTES / 4) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_frame() {
        // Status byte + 8 channels × 3 bytes
        let frame = [
            0x00, // Status
            0x00, 0x00, 0x01, // ch0: 1
            0x00, 0x00, 0x02, // ch1: 2
            0x00, 0x00, 0x03, // ch2: 3
            0x00, 0x00, 0x04, // ch3: 4
            0x00, 0x00, 0x05, // ch4: 5
            0x00, 0x00, 0x06, // ch5: 6
            0x00, 0x00, 0x07, // ch6: 7
            0x00, 0x00, 0x08, // ch7: 8
        ];
        let samples = Ads1299::decode_frame(&frame);
        assert_eq!(samples, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_negative_sample() {
        let frame = [0x00, 0xFF, 0xFF, 0xFF]; // -1 in 24-bit two's complement
        let samples = Ads1299::decode_frame(&frame);
        assert_eq!(samples[0], -1);
    }

    #[test]
    fn test_large_positive() {
        let frame = [0x00, 0x7F, 0xFF, 0xFF]; // Max positive 24-bit
        let samples = Ads1299::decode_frame(&frame);
        assert_eq!(samples[0], 8_388_607);
    }

    #[test]
    fn test_large_negative() {
        let frame = [0x00, 0x80, 0x00, 0x00]; // Max negative 24-bit
        let samples = Ads1299::decode_frame(&frame);
        assert_eq!(samples[0], -8_388_608);
    }
}
