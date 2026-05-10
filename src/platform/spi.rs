//! SPI Interface — Production Implementation
//!
//! STM32F407 SPI3 for ADS1299 communication.
//! Based on RM0090 Reference Manual §28.

/// SPI3 register base
const SPI3_BASE: u32 = 0x4000_3C00;

/// SPI registers
#[repr(C)]
pub struct SpiRegs {
    cr1: u16,       // 0x00: Control register 1
    _pad0: u16,
    cr2: u16,       // 0x04: Control register 2
    _pad1: u16,
    sr: u16,        // 0x08: Status register
    _pad2: u16,
    dr: u16,        // 0x0C: Data register
    _pad3: u16,
    crcpr: u16,     // 0x10: CRC polynomial
    _pad4: u16,
    rxcrcr: u16,    // 0x14: RX CRC
    _pad5: u16,
    txcrcr: u16,    // 0x18: TX CRC
    _pad6: u16,
    i2scfgr: u16,   // 0x1C: I2S configuration
    _pad7: u16,
    i2spr: u16,     // 0x20: I2S prescaler
    _pad8: u16,
}

/// SPI3 register access
const SPI3: *mut SpiRegs = SPI3_BASE as *mut SpiRegs;

/// SPI status flags
pub const SPI_SR_RXNE: u16 = 1 << 0;  // Receive buffer not empty
pub const SPI_SR_TXE: u16 = 1 << 1;   // Transmit buffer empty
pub const SPI_SR_CHSIDE: u16 = 1 << 2; // Channel side
pub const SPI_SR_UDR: u16 = 1 << 3;    // Underrun flag
pub const SPI_SR_CRCERR: u16 = 1 << 4; // CRC error
pub const SPI_SR_MODF: u16 = 1 << 5;  // Mode fault
pub const SPI_SR_OVR: u16 = 1 << 6;   // Overrun
pub const SPI_SR_BSY: u16 = 1 << 7;   // Busy
pub const SPI_SR_FRE: u16 = 1 << 8;   // Frame format error

/// SPI CR1 flags
const SPI_CR1_CPHA: u16 = 1 << 0;      // Clock phase
const SPI_CR1_CPOL: u16 = 1 << 1;      // Clock polarity
const SPI_CR1_MSTR: u16 = 1 << 2;      // Master selection
const SPI_CR1_BR_MASK: u16 = 0x7 << 3; // Baud rate mask
const SPI_CR1_SPE: u16 = 1 << 6;       // SPI enable
const SPI_CR1_LSBFIRST: u16 = 1 << 7;  // Frame format
const SPI_CR1_SSI: u16 = 1 << 8;       // Internal slave select
const SPI_CR1_SSM: u16 = 1 << 9;       // Software slave management
const SPI_CR1_RXONLY: u16 = 1 << 10;   // Receive only
const SPI_CR1_DFF: u16 = 1 << 11;      // Data frame format (16-bit)
const SPI_CR1_CRCNEXT: u16 = 1 << 12;  // CRC next
const SPI_CR1_CRCEN: u16 = 1 << 13;    // CRC enable
const SPI_CR1_BIDIOE: u16 = 1 << 14;   // Output enable in bidirectional mode
const SPI_CR1_BIDIMODE: u16 = 1 << 15; // Bidirectional data mode enable

/// SPI CR2 flags
const SPI_CR2_RXDMAEN: u16 = 1 << 0;   // RX DMA enable
const SPI_CR2_TXDMAEN: u16 = 1 << 1;   // TX DMA enable
const SPI_CR2_SSOE: u16 = 1 << 2;      // SS output enable
const SPI_CR2_FRF: u16 = 1 << 4;       // Frame format (TI mode)
const SPI_CR2_ERRIE: u16 = 1 << 5;     // Error interrupt enable
const SPI_CR2_TXEIE: u16 = 1 << 7;     // TX empty interrupt enable
const SPI_CR2_RXNEIE: u16 = 1 << 6;    // RX not empty interrupt enable

/// SPI configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiConfig {
    /// SPI clock prescaler (BR[2:0])
    /// 000: fPCLK/2, 001: /4, 010: /8, 011: /16, 100: /32, 101: /64, 110: /128, 111: /256
    pub baud_rate: u8,
    /// Clock polarity (CPOL)
    pub cpol: bool,
    /// Clock phase (CPHA)
    pub cpha: bool,
    /// Data frame format: false = 8-bit, true = 16-bit
    pub dff_16bit: bool,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            baud_rate: 0b001, // fPCLK/4 = 42 MHz / 4 = 10.5 MHz (safe for ADS1299)
            cpol: false,      // CPOL = 0
            cpha: true,       // CPHA = 1 (ADS1299 requires Mode 1)
            dff_16bit: true,  // 16-bit data frame
        }
    }
}

/// SPI controller
pub struct SpiController;

impl SpiController {
    /// Create and configure SPI3 for ADS1299
    pub fn new() -> Self {
        Self
    }

    /// Initialize SPI3
    ///
    /// Default: Mode 1 (CPOL=0, CPHA=1), 16-bit, master, software CS
    pub fn configure(&self, config: &SpiConfig) {
        unsafe {
            // 1. Disable SPI
            (*SPI3).cr1 &= !SPI_CR1_SPE;

            // 2. Configure CR1
            let mut cr1 = SPI_CR1_MSTR | SPI_CR1_SSM | SPI_CR1_SSI; // Master, software CS

            // Baud rate
            cr1 |= (config.baud_rate as u16 & 0x7) << 3;

            // CPOL
            if config.cpol {
                cr1 |= SPI_CR1_CPOL;
            }

            // CPHA
            if config.cpha {
                cr1 |= SPI_CR1_CPHA;
            }

            // Data frame format
            if config.dff_16bit {
                cr1 |= SPI_CR1_DFF;
            }

            (*SPI3).cr1 = cr1;

            // 3. Configure CR2 (RX DMA enable for automatic DMA transfer)
            (*SPI3).cr2 = SPI_CR2_RXDMAEN;

            // 4. Enable SPI
            (*SPI3).cr1 |= SPI_CR1_SPE;
        }
    }

    /// Send command (blocking)
    pub fn send_command(&self, cmd: u16) {
        unsafe {
            // Wait for TXE
            while ((*SPI3).sr & SPI_SR_TXE) == 0 {}

            // Write data
            (*SPI3).dr = cmd;

            // Wait for RXNE (full duplex)
            while ((*SPI3).sr & SPI_SR_RXNE) == 0 {}

            // Read to clear RXNE
            let _ = (*SPI3).dr;
        }
    }

    /// Send and receive (blocking)
    pub fn transfer(&self, data: u16) -> u16 {
        unsafe {
            // Wait for TXE
            while ((*SPI3).sr & SPI_SR_TXE) == 0 {}

            // Write data
            (*SPI3).dr = data;

            // Wait for RXNE
            while ((*SPI3).sr & SPI_SR_RXNE) == 0 {}

            // Read received data
            (*SPI3).dr
        }
    }

    /// Read data via DMA (non-blocking, DMA handles transfer)
    pub fn read_dma(&self, _buffer: &mut [u32], _len: usize) {
        // DMA is configured to automatically transfer from SPI3 DR to buffer
        // Just ensure SPI RX DMA is enabled
        unsafe {
            (*SPI3).cr2 |= SPI_CR2_RXDMAEN;
        }
    }

    /// Chip select control (software managed via GPIO)
    ///
    /// Note: ADS1299 CS is typically on PA4 or PC0
    pub fn cs_low(&self) {
        // Clear CS via GPIOA_BSRR (bit reset register)
        unsafe {
            let gpioa_bsrr = 0x4002_0018 as *mut u32;
            core::ptr::write_volatile(gpioa_bsrr, 1 << (4 + 16)); // BR4
        }
    }

    pub fn cs_high(&self) {
        unsafe {
            let gpioa_bsrr = 0x4002_0018 as *mut u32;
            core::ptr::write_volatile(gpioa_bsrr, 1 << 4); // BS4
        }
    }

    /// Check if SPI is busy
    pub fn is_busy(&self) -> bool {
        unsafe { ((*SPI3).sr & SPI_SR_BSY) != 0 }
    }

    /// Get status register
    pub fn status(&self) -> u16 {
        unsafe { (*SPI3).sr }
    }

    /// Disable SPI
    pub fn disable(&self) {
        unsafe {
            (*SPI3).cr1 &= !SPI_CR1_SPE;
        }
    }
}

/// Global SPI3 controller instance
pub static SPI3_CONTROLLER: SpiController = SpiController;
