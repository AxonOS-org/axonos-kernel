//! Data Watchpoint and Trace (DWT) Cycle Counter
//!
//! Production implementation using real DWT registers.
//! DWT base: 0xE000_1000 (Cortex-M4 internal)

/// DWT register addresses
const DWT_BASE: u32 = 0xE000_1000;
const DWT_CTRL: *mut u32 = (DWT_BASE + 0x00) as *mut u32;
const DWT_CYCCNT: *mut u32 = (DWT_BASE + 0x04) as *mut u32;

/// DWT cycle counter interface
pub struct Dwt;

impl Dwt {
    /// Create and enable DWT
    pub fn new() -> Self {
        unsafe {
            // Enable DWT tracing (DEMCR.TRCENA)
            let demcr = 0xE000_EDFC as *mut u32;
            core::ptr::write_volatile(demcr, core::ptr::read_volatile(demcr) | (1 << 24));

            // Enable CYCCNT
            core::ptr::write_volatile(DWT_CTRL, core::ptr::read_volatile(DWT_CTRL) | 1);
        }
        Self
    }

    /// Read current cycle count (32-bit, wraps at 2^32)
    ///
    /// Resolution: ~5.95 ns at 168 MHz [L1]
    /// For 64-bit monotonic: handle overflow in software
    pub fn cycle_count(&self) -> u64 {
        unsafe {
            // Read CYCCNT (32-bit)
            let cycles = core::ptr::read_volatile(DWT_CYCCNT) as u64;
            cycles
        }
    }

    /// Read with overflow handling (requires periodic sampling)
    pub fn cycle_count_64(&self, overflow_count: u32) -> u64 {
        let low = self.cycle_count() as u64;
        ((overflow_count as u64) << 32) | low
    }

    /// Convert cycles to microseconds
    pub fn cycles_to_us(&self, cycles: u64) -> u32 {
        // cycles / 168 (cycles per µs at 168 MHz)
        (cycles / 168) as u32
    }

    /// Convert microseconds to cycles
    pub fn us_to_cycles(&self, us: u32) -> u64 {
        (us as u64) * 168
    }

    /// Measure function execution time
    pub fn measure<F, R>(&self, f: F) -> (R, u32)
    where
        F: FnOnce() -> R,
    {
        let start = self.cycle_count();
        let result = f();
        let end = self.cycle_count();

        // Handle 32-bit wraparound
        let elapsed = if end >= start {
            end - start
        } else {
            (0xFFFF_FFFFu64 - start) + end + 1
        };

        (result, self.cycles_to_us(elapsed))
    }

    /// Reset cycle counter
    pub fn reset(&self) {
        unsafe {
            core::ptr::write_volatile(DWT_CYCCNT, 0);
        }
    }
}

impl Default for Dwt {
    fn default() -> Self {
        Self::new()
    }
}
