//! IRQ Controller — Production Implementation
//!
//! NVIC (Nested Vectored Interrupt Controller) for Cortex-M4F.
/// NVIC register base
const NVIC_BASE: u32 = 0xE000_E100;

/// NVIC registers
#[repr(C)]
pub struct NvicRegs {
    iser: [u32; 8],      // 0x000: Interrupt Set Enable
    _reserved0: [u32; 24],
    icer: [u32; 8],      // 0x080: Interrupt Clear Enable
    _reserved1: [u32; 24],
    ispr: [u32; 8],      // 0x100: Interrupt Set Pending
    _reserved2: [u32; 24],
    icpr: [u32; 8],      // 0x180: Interrupt Clear Pending
    _reserved3: [u32; 24],
    iabr: [u32; 8],      // 0x200: Interrupt Active Bit
    _reserved4: [u32; 56],
    ipr: [u32; 60],      // 0x300: Interrupt Priority (240 priorities, 4 per word)
}

const NVIC: *mut NvicRegs = NVIC_BASE as *mut NvicRegs;

/// IRQ number definitions for STM32F407
pub const IRQN_DMA2_STREAM3: u8 = 59;  // DMA2 Stream 3 global interrupt
pub const IRQN_DMA2_STREAM4: u8 = 60;  // DMA2 Stream 4 global interrupt
pub const IRQN_SPI3: u8 = 51;        // SPI3 global interrupt
pub const IRQN_TIM2: u8 = 28;        // TIM2 global interrupt
pub const IRQN_EXTI0: u8 = 6;        // EXTI Line0 interrupt

/// IRQ controller
pub struct IrqController;

impl IrqController {
    /// Enable IRQ in NVIC
    ///
    /// # Arguments
    /// * `irqn` — IRQ number (0-239)
    #[inline(always)]
    pub fn enable(irqn: u8) {
        unsafe {
            let reg = irqn as usize / 32;
            let bit = irqn % 32;
            (*NVIC).iser[reg] = 1 << bit;
        }
    }

    /// Disable IRQ in NVIC
    #[inline(always)]
    pub fn disable(irqn: u8) {
        unsafe {
            let reg = irqn as usize / 32;
            let bit = irqn % 32;
            (*NVIC).icer[reg] = 1 << bit;
        }
    }

    /// Set IRQ priority (0-15, lower = higher priority)
    ///
    /// Cortex-M4F: 4 priority bits implemented (16 levels)
    #[inline(always)]
    pub fn set_priority(irqn: u8, priority: u8) {
        unsafe {
            let reg = irqn as usize / 4;
            let shift = (irqn % 4) * 8;
            // Priority in upper 4 bits of byte (bits [7:4])
            let prio = ((priority & 0x0F) as u32) << (shift + 4);
            let mask = 0xFF << shift;
            (*NVIC).ipr[reg] = ((*NVIC).ipr[reg] & !mask) | prio;
        }
    }

    /// Get IRQ priority
    pub fn get_priority(irqn: u8) -> u8 {
        unsafe {
            let reg = irqn as usize / 4;
            let shift = (irqn % 4) * 8;
            (((*NVIC).ipr[reg] >> (shift + 4)) & 0x0F) as u8
        }
    }

    /// Check if IRQ is active
    pub fn is_active(irqn: u8) -> bool {
        unsafe {
            let reg = irqn as usize / 32;
            let bit = irqn % 32;
            ((*NVIC).iabr[reg] & (1 << bit)) != 0
        }
    }

    /// Check if IRQ is enabled
    pub fn is_enabled(irqn: u8) -> bool {
        unsafe {
            let reg = irqn as usize / 32;
            let bit = irqn % 32;
            // Read back from ISER (reads as 1 if enabled)
            ((*NVIC).iser[reg] & (1 << bit)) != 0
        }
    }

    /// Set pending (software trigger)
    pub fn set_pending(irqn: u8) {
        unsafe {
            let reg = irqn as usize / 32;
            let bit = irqn % 32;
            (*NVIC).ispr[reg] = 1 << bit;
        }
    }

    /// Clear pending
    pub fn clear_pending(irqn: u8) {
        unsafe {
            let reg = irqn as usize / 32;
            let bit = irqn % 32;
            (*NVIC).icpr[reg] = 1 << bit;
        }
    }

    /// Global disable interrupts (set PRIMASK)
    pub fn global_disable() {
        cortex_m::interrupt::disable();
    }

    /// Global enable interrupts (clear PRIMASK)
    pub fn global_enable() {
        unsafe { cortex_m::interrupt::enable(); }
    }

    /// Configure system for real-time operation
    ///
    /// Sets priority grouping to 4 bits (16 priority levels)
    pub fn configure() {
        // Set PRIGROUP to 0b011 (4 bits for preemption, 0 for subpriority)
        // AIRCR.PRIGROUP = 0b011
        unsafe {
            let aircr = 0xE000_ED0C as *mut u32;
            // Write with VECTKEY (0x05FA << 16) | PRIGROUP (0b011 << 8)
            core::ptr::write_volatile(aircr, (0x05FA << 16) | (0b011 << 8));
        }
    }
}

/// Global IRQ controller instance
pub static IRQ_CONTROLLER: IrqController = IrqController;
