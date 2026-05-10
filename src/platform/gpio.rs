//! GPIO Abstraction — Production Implementation
//!
//! Direct register access for STM32F407 GPIO ports.

/// GPIO register block structure
#[repr(C)]
pub struct GpioRegisters {
    moder: u32,      // 0x00: Mode register
    otyper: u32,     // 0x04: Output type
    ospeedr: u32,    // 0x08: Output speed
    pupdr: u32,      // 0x0C: Pull-up/pull-down
    idr: u32,        // 0x10: Input data
    odr: u32,        // 0x14: Output data
    bsrr: u32,       // 0x18: Bit set/reset
    lckr: u32,       // 0x1C: Configuration lock
    afrl: u32,       // 0x20: Alternate function low
    afrh: u32,       // 0x24: Alternate function high
}

/// GPIO port base addresses
const GPIOA_BASE: *mut GpioRegisters = 0x4002_0000 as *mut GpioRegisters;
const GPIOB_BASE: *mut GpioRegisters = 0x4002_0400 as *mut GpioRegisters;
const GPIOC_BASE: *mut GpioRegisters = 0x4002_0800 as *mut GpioRegisters;

/// GPIO pin abstraction
pub struct GpioPin {
    port: *mut GpioRegisters,
    pin: u8,
}

impl GpioPin {
    /// Create new GPIO pin
    ///
    /// # Safety
    /// port must be valid GPIO base address
    pub const unsafe fn new(port: *mut GpioRegisters, pin: u8) -> Self {
        Self { port, pin }
    }

    /// Configure as push-pull output
    pub fn configure_output(&self) {
        unsafe {
            let port = &mut *self.port;
            let bit_pos = self.pin * 2;
            let mask = 0x3 << bit_pos;
            // 01 = General purpose output mode
            port.moder = (port.moder & !mask) | (0x1 << bit_pos);
            // Push-pull (0)
            port.otyper &= !(1 << self.pin);
            // High speed (10)
            port.ospeedr = (port.ospeedr & !mask) | (0x2 << bit_pos);
            // No pull-up/pull-down
            port.pupdr &= !mask;
        }
    }

    /// Configure as input
    pub fn configure_input(&self) {
        unsafe {
            let port = &mut *self.port;
            let bit_pos = self.pin * 2;
            let mask = 0x3 << bit_pos;
            // 00 = Input mode
            port.moder &= !mask;
        }
    }

    /// Set high
    pub fn set_high(&self) {
        unsafe {
            (*self.port).bsrr = 1 << self.pin; // BSx
        }
    }

    /// Set low
    pub fn set_low(&self) {
        unsafe {
            (*self.port).bsrr = 1 << (self.pin + 16); // BRx
        }
    }

    /// Toggle
    pub fn toggle(&self) {
        unsafe {
            let port = &mut *self.port;
            port.odr ^= 1 << self.pin;
        }
    }

    /// Read input state
    pub fn is_high(&self) -> bool {
        unsafe { ((*self.port).idr & (1 << self.pin)) != 0 }
    }

    /// Read output state
    pub fn is_set_high(&self) -> bool {
        unsafe { ((*self.port).odr & (1 << self.pin)) != 0 }
    }
}

/// L3 validation pins (PA0, PA1)
pub const GPIO_PA0: GpioPin = unsafe { GpioPin::new(GPIOA_BASE, 0) };
pub const GPIO_PA1: GpioPin = unsafe { GpioPin::new(GPIOA_BASE, 1) };

/// Status LED pin (PC13 on most STM32F4 Discovery boards)
pub const GPIO_PC13: GpioPin = unsafe { GpioPin::new(GPIOC_BASE, 13) };
