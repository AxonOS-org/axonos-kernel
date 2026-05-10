//! DMA Controller — Production Implementation
//!
//! STM32F407 DMA2 Stream 3 for ADS1299 SPI transfer.
//! Based on RM0090 Reference Manual §9.

/// DMA2 Stream 3 register base
const DMA2_BASE: u32 = 0x4002_6400;
const DMA2_STREAM3_BASE: u32 = DMA2_BASE + 0x58;

/// DMA stream registers
#[repr(C)]
pub struct DmaStreamRegs {
    cr: u32,    // 0x00: Configuration
    ndtr: u32,  // 0x04: Number of data items
    par: u32,   // 0x08: Peripheral address
    m0ar: u32,  // 0x0C: Memory 0 address
    m1ar: u32,  // 0x10: Memory 1 address
    fcr: u32,   // 0x14: FIFO control
}

/// DMA interrupt status registers
const DMA2_LISR: *mut u32 = (DMA2_BASE + 0x00) as *mut u32;
const DMA2_LIFCR: *mut u32 = (DMA2_BASE + 0x08) as *mut u32;

/// DMA configuration flags
pub const DMA_CR_EN: u32 = 1 << 0;
pub const DMA_CR_TCIE: u32 = 1 << 4;   // Transfer complete interrupt enable
pub const DMA_CR_HTIE: u32 = 1 << 3;   // Half transfer interrupt enable
pub const DMA_CR_TEIE: u32 = 1 << 2;   // Transfer error interrupt enable
pub const DMA_CR_DIR_P2M: u32 = 0 << 6; // Peripheral to memory
pub const DMA_CR_DIR_M2P: u32 = 1 << 6; // Memory to peripheral
pub const DMA_CR_CIRC: u32 = 1 << 8;    // Circular mode
pub const DMA_CR_MINC: u32 = 1 << 10;   // Memory increment
pub const DMA_CR_PINC: u32 = 1 << 9;    // Peripheral increment
pub const DMA_CR_PSIZE_8: u32 = 0 << 11;  // Peripheral size 8-bit
pub const DMA_CR_PSIZE_16: u32 = 1 << 11; // Peripheral size 16-bit
pub const DMA_CR_PSIZE_32: u32 = 2 << 11; // Peripheral size 32-bit
pub const DMA_CR_MSIZE_8: u32 = 0 << 13;  // Memory size 8-bit
pub const DMA_CR_MSIZE_16: u32 = 1 << 13; // Memory size 16-bit
pub const DMA_CR_MSIZE_32: u32 = 2 << 13; // Memory size 32-bit
pub const DMA_CR_PL_LOW: u32 = 0 << 16;     // Priority low
pub const DMA_CR_PL_MEDIUM: u32 = 1 << 16;  // Priority medium
pub const DMA_CR_PL_HIGH: u32 = 2 << 16;    // Priority high
pub const DMA_CR_PL_VHIGH: u32 = 3 << 16;   // Priority very high
pub const DMA_CR_CHSEL_0: u32 = 0 << 25;    // Channel 0

/// DMA status flags
pub const DMA_TCIF3: u32 = 1 << 27;  // Stream 3 transfer complete
pub const DMA_HTIF3: u32 = 1 << 26;  // Stream 3 half transfer
pub const DMA_TEIF3: u32 = 1 << 25;  // Stream 3 transfer error
pub const DMA_DMEIF3: u32 = 1 << 24; // Stream 3 direct mode error
pub const DMA_FEIF3: u32 = 1 << 22;  // Stream 3 FIFO error

/// DMA controller
pub struct DmaController;

impl DmaController {
    /// Configure DMA2 Stream 3 for ADS1299 SPI3 RX
    ///
    /// Source 3: DMA bus-matrix arbitration
    /// Worst-case 40-cycle AHB arbitration per word
    /// t_DMA ≤ 6 × 40 / 168MHz = 1.43 µs [L1]
    ///
    /// # Arguments
    /// * `peripheral_addr` — SPI3 DR register address
    /// * `memory_addr` — Destination buffer address
    /// * `length` — Number of 32-bit words to transfer (6 for ADS1299 frame)
    pub fn configure_ads1299_dma(peripheral_addr: u32, memory_addr: u32, length: u16) {
        unsafe {
            let stream = DMA2_STREAM3_BASE as *mut DmaStreamRegs;

            // 1. Disable stream
            (*stream).cr &= !DMA_CR_EN;

            // 2. Wait until disabled
            while ((*stream).cr & DMA_CR_EN) != 0 {}

            // 3. Clear all interrupt flags
            core::ptr::write_volatile(DMA2_LIFCR, DMA_TCIF3 | DMA_HTIF3 | DMA_TEIF3 | DMA_DMEIF3 | DMA_FEIF3);

            // 4. Configure stream
            // Channel 0, Very High priority, 32-bit peripheral/memory, circular, peripheral-to-memory
            (*stream).cr = DMA_CR_CHSEL_0
                | DMA_CR_PL_VHIGH
                | DMA_CR_MSIZE_32
                | DMA_CR_PSIZE_32
                | DMA_CR_MINC
                | DMA_CR_CIRC
                | DMA_CR_TCIE
                | DMA_CR_HTIE
                | DMA_CR_TEIE;

            // 5. Set addresses
            (*stream).par = peripheral_addr;
            (*stream).m0ar = memory_addr;
            (*stream).ndtr = length as u32;

            // 6. FIFO control: direct mode (no FIFO)
            (*stream).fcr = 0;
        }
    }

    /// Start DMA transfer
    pub fn start(&self) {
        unsafe {
            let stream = DMA2_STREAM3_BASE as *mut DmaStreamRegs;
            (*stream).cr |= DMA_CR_EN;
        }
    }

    /// Stop DMA transfer
    pub fn stop(&self) {
        unsafe {
            let stream = DMA2_STREAM3_BASE as *mut DmaStreamRegs;
            (*stream).cr &= !DMA_CR_EN;
        }
    }

    /// Check if transfer complete
    pub fn is_complete(&self) -> bool {
        unsafe {
            (core::ptr::read_volatile(DMA2_LISR) & DMA_TCIF3) != 0
        }
    }

    /// Check if half-transfer complete
    pub fn is_half_complete(&self) -> bool {
        unsafe {
            (core::ptr::read_volatile(DMA2_LISR) & DMA_HTIF3) != 0
        }
    }

    /// Clear transfer complete flag
    pub fn clear_complete(&self) {
        unsafe {
            core::ptr::write_volatile(DMA2_LIFCR, DMA_TCIF3);
        }
    }

    /// Clear half-transfer flag
    pub fn clear_half(&self) {
        unsafe {
            core::ptr::write_volatile(DMA2_LIFCR, DMA_HTIF3);
        }
    }

    /// Get remaining data count
    pub fn remaining(&self) -> u16 {
        unsafe {
            let stream = DMA2_STREAM3_BASE as *mut DmaStreamRegs;
            ((*stream).ndtr & 0xFFFF) as u16
        }
    }

    /// Handle DMA interrupt
    ///
    /// Returns (half_transfer, full_transfer, error)
    pub fn handle_irq(&self) -> (bool, bool, bool) {
        unsafe {
            let lisr = core::ptr::read_volatile(DMA2_LISR);
            let half = (lisr & DMA_HTIF3) != 0;
            let full = (lisr & DMA_TCIF3) != 0;
            let err = (lisr & (DMA_TEIF3 | DMA_DMEIF3 | DMA_FEIF3)) != 0;

            // Clear all flags
            core::ptr::write_volatile(DMA2_LIFCR, DMA_TCIF3 | DMA_HTIF3 | DMA_TEIF3 | DMA_DMEIF3 | DMA_FEIF3);

            (half, full, err)
        }
    }
}

/// Global DMA controller instance
pub static DMA_CONTROLLER: DmaController = DmaController;
