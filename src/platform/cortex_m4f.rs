//! STM32F407 Cortex-M4F Platform Initialization
//!
//! Production-grade initialization with real register writes.
//! Based on RM0090 Reference Manual.

use crate::config;
use cortex_m::peripheral::{Peripherals, SCB, DWT, NVIC};
use cortex_m::peripheral::scb::SystemHandler;

/// RCC register block base address
const RCC_BASE: u32 = 0x4002_3800;
const RCC_CR: *mut u32 = (RCC_BASE + 0x00) as *mut u32;
const RCC_PLLCFGR: *mut u32 = (RCC_BASE + 0x04) as *mut u32;
const RCC_CFGR: *mut u32 = (RCC_BASE + 0x08) as *mut u32;
const RCC_AHB1ENR: *mut u32 = (RCC_BASE + 0x30) as *mut u32;
const RCC_APB2ENR: *mut u32 = (RCC_BASE + 0x44) as *mut u32;

/// Flash interface register base
const FLASH_BASE: u32 = 0x4002_3C00;
const FLASH_ACR: *mut u32 = (FLASH_BASE + 0x00) as *mut u32;

/// GPIOA base address
const GPIOA_BASE: u32 = 0x4002_0000;
const GPIOA_MODER: *mut u32 = (GPIOA_BASE + 0x00) as *mut u32;
const GPIOA_ODR: *mut u32 = (GPIOA_BASE + 0x14) as *mut u32;
const GPIOA_BSRR: *mut u32 = (GPIOA_BASE + 0x18) as *mut u32;

/// Cortex-M4F platform initialization
pub struct CortexM4f;

impl CortexM4f {
    /// Full system initialization
    ///
    /// Configures:
    /// - Flash wait states (5 WS for 168 MHz)
    /// - PLL: HSE 8 MHz -> 168 MHz (PLLM=8, PLLN=336, PLLP=2)
    /// - ART accelerator
    /// - DWT cycle counter
    /// - GPIOA clock
    pub fn init() {
        unsafe {
            // 1. Configure Flash latency (5 wait states at 168 MHz) [RM0090 §3.3.3]
            // LATENCY = 5 (bits 2:0), PRFTEN = 1 (bit 8), ICEN = 1 (bit 9), DCEN = 1 (bit 10)
            core::ptr::write_volatile(FLASH_ACR, 0x0000_0707);

            // Wait for flash latency to be applied
            while (core::ptr::read_volatile(FLASH_ACR) & 0x7) != 5 {}

            // 2. Enable HSE oscillator
            let rcc_cr = core::ptr::read_volatile(RCC_CR);
            core::ptr::write_volatile(RCC_CR, rcc_cr | (1 << 16)); // HSEON

            // Wait for HSE ready
            while (core::ptr::read_volatile(RCC_CR) & (1 << 17)) == 0 {}

            // 3. Configure PLL
            // PLLM=8, PLLN=336, PLLP=2, PLLQ=7, PLLSRC=HSE
            // PLLCFGR = 0x0740_3408
            core::ptr::write_volatile(RCC_PLLCFGR, 0x0740_3408);

            // 4. Enable PLL
            let rcc_cr = core::ptr::read_volatile(RCC_CR);
            core::ptr::write_volatile(RCC_CR, rcc_cr | (1 << 24)); // PLLON

            // Wait for PLL ready
            while (core::ptr::read_volatile(RCC_CR) & (1 << 25)) == 0 {}

            // 5. Switch to PLL as system clock
            let rcc_cfgr = core::ptr::read_volatile(RCC_CFGR);
            // SW = 10 (PLL), HPRE = 0 (AHB not divided), PPRE1 = 4 (APB1 /4), PPRE2 = 2 (APB2 /2)
            core::ptr::write_volatile(RCC_CFGR, (rcc_cfgr & !0x3) | 0x2 | (4 << 10) | (2 << 13));

            // Wait for switch
            while (core::ptr::read_volatile(RCC_CFGR) & 0xC) != 0x8 {}

            // 6. Enable ART accelerator (already done via FLASH_ACR)

            // 7. Enable GPIOA clock
            let ahb1enr = core::ptr::read_volatile(RCC_AHB1ENR);
            core::ptr::write_volatile(RCC_AHB1ENR, ahb1enr | (1 << 0)); // GPIOAEN

            // 8. Enable DWT cycle counter via DWT_CTRL
            let dwt_ctrl = 0xE000_1000 as *mut u32;
            core::ptr::write_volatile(dwt_ctrl, core::ptr::read_volatile(dwt_ctrl) | 1);
        }
    }

    /// Get CPU frequency [Hz]
    pub fn cpu_hz() -> u32 {
        config::CPU_HZ
    }

    /// Get Flash wait states
    pub fn flash_wait_states() -> u8 {
        unsafe { (core::ptr::read_volatile(FLASH_ACR) & 0x7) as u8 }
    }

    /// Check if ART accelerator enabled
    pub fn art_enabled() -> bool {
        unsafe { (core::ptr::read_volatile(FLASH_ACR) & (1 << 9)) != 0 }
    }

    /// Enter sleep mode (WFI)
    pub fn sleep() {
        cortex_m::asm::wfi();
    }

    /// Configure PA0 and PA1 as outputs for L3 validation
    pub fn configure_l3_gpio() {
        unsafe {
            // PA0 = output, PA1 = output
            let moder = core::ptr::read_volatile(GPIOA_MODER);
            // Clear mode bits for PA0 and PA1 (bits 1:0 and 3:2)
            // Set to 01 (General purpose output mode)
            core::ptr::write_volatile(GPIOA_MODER, (moder & !0xF) | 0x5);
        }
    }

    /// Set PA0 high (epoch entry marker)
    pub fn gpio_pa0_set() {
        unsafe {
            core::ptr::write_volatile(GPIOA_BSRR, 1 << 0); // BS0
        }
    }

    /// Set PA0 low
    pub fn gpio_pa0_clear() {
        unsafe {
            core::ptr::write_volatile(GPIOA_BSRR, 1 << (0 + 16)); // BR0
        }
    }

    /// Set PA1 high (pipeline complete marker)
    pub fn gpio_pa1_set() {
        unsafe {
            core::ptr::write_volatile(GPIOA_BSRR, 1 << 1); // BS1
        }
    }

    /// Set PA1 low
    pub fn gpio_pa1_clear() {
        unsafe {
            core::ptr::write_volatile(GPIOA_BSRR, 1 << (1 + 16)); // BR1
        }
    }

    /// Toggle PA0
    pub fn gpio_pa0_toggle() {
        unsafe {
            let odr = core::ptr::read_volatile(GPIOA_ODR);
            core::ptr::write_volatile(GPIOA_ODR, odr ^ (1 << 0));
        }
    }
}
