//! Stimulation Interlock — Production Implementation
//!
//! DC5: Safe-idle on M4F heartbeat loss ≤ 12 ms [L2]
//!
//! Hardware interlock that cuts stimulation power when:
//! 1. M4F heartbeat lost (> 12 ms)
//! 2. Consent withdrawn
//! 3. DC1 deadline miss detected
//!
//! The interlock controls a GPIO pin that drives a safety relay
//! or MOSFET controlling the stimulation current source.

use super::{ConsentFsm, ConsentState};
use crate::platform::gpio::{GpioPin, GPIO_PC13};

/// GPIO for stimulation enable (active high)
/// PC13 on STM32F4 Discovery, or dedicated safety pin on custom board
const STIM_ENABLE_PIN: GpioPin = GPIO_PC13;

/// Stimulation interlock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterlockState {
    /// Stimulation active (all safety checks passed)
    Active,
    /// Safe-idle (stimulation disabled, recoverable)
    SafeIdle,
    /// Error state (requires manual reset)
    Error,
}

/// Stimulation interlock
pub struct Interlock {
    state: InterlockState,
    /// Consecutive heartbeat misses
    heartbeat_misses: u32,
    /// Maximum allowed misses before safe-idle
    max_heartbeat_misses: u32,
    /// Watchdog timer value [ms]
    watchdog_ms: u32,
    /// Stimulation GPIO configured
    gpio_configured: bool,
}

impl Interlock {
    /// Create new interlock
    ///
    /// Starts in SafeIdle state — stimulation disabled until explicitly enabled.
    pub fn new() -> Self {
        Self {
            state: InterlockState::SafeIdle,
            heartbeat_misses: 0,
            max_heartbeat_misses: 3, // 3 × 4ms epoch = 12ms
            watchdog_ms: 0,
            gpio_configured: false,
        }
    }

    /// Initialize GPIO for stimulation control
    pub fn init_gpio(&mut self) {
        STIM_ENABLE_PIN.configure_output();
        STIM_ENABLE_PIN.set_low(); // Start with stimulation disabled
        self.gpio_configured = true;
    }

    /// Activate safe-idle (disable stimulation)
    ///
    /// Called on:
    /// - DC5 heartbeat timeout
    /// - Consent withdrawal
    /// - DC1 deadline miss
    pub fn activate_safe_idle() {
        STIM_ENABLE_PIN.set_low();
    }

    /// Check if stimulation is safe to enable
    pub fn is_safe(&self, consent: &ConsentFsm, heartbeat_valid: bool) -> bool {
        consent.is_stimulation_allowed() && heartbeat_valid && self.gpio_configured
    }

    /// Update interlock state based on safety inputs
    ///
    /// Call this every epoch (4 ms) with current safety status.
    pub fn update(&mut self, consent: &ConsentFsm, heartbeat_valid: bool) {
        if !self.gpio_configured {
            return;
        }

        match self.state {
            InterlockState::Active => {
                if !self.is_safe(consent, heartbeat_valid) {
                    // Safety violation — enter safe-idle
                    self.state = InterlockState::SafeIdle;
                    STIM_ENABLE_PIN.set_low();

                    if !heartbeat_valid {
                        self.heartbeat_misses += 1;
                        if self.heartbeat_misses >= self.max_heartbeat_misses {
                            self.state = InterlockState::Error;
                        }
                    }
                } else {
                    self.heartbeat_misses = 0;
                }
            }
            InterlockState::SafeIdle => {
                if self.is_safe(consent, heartbeat_valid) {
                    // All checks passed — can re-enable
                    self.state = InterlockState::Active;
                    STIM_ENABLE_PIN.set_high();
                    self.heartbeat_misses = 0;
                }
            }
            InterlockState::Error => {
                // Requires manual reset via reset()
                STIM_ENABLE_PIN.set_low();
            }
        }
    }

    /// Current state
    pub fn state(&self) -> InterlockState {
        self.state
    }

    /// Check if stimulation is currently active
    pub fn is_stimulating(&self) -> bool {
        self.state == InterlockState::Active
    }

    /// Reset interlock (requires manual confirmation)
    ///
    /// Only valid from Error state.
    pub fn reset(&mut self) {
        self.state = InterlockState::SafeIdle;
        self.heartbeat_misses = 0;
        self.watchdog_ms = 0;
        STIM_ENABLE_PIN.set_low();
    }

    /// Force enable stimulation (for testing only)
    #[cfg(test)]
    pub fn force_enable(&mut self) {
        self.state = InterlockState::Active;
        STIM_ENABLE_PIN.set_high();
    }

    /// Force disable stimulation (for testing only)
    #[cfg(test)]
    pub fn force_disable(&mut self) {
        self.state = InterlockState::SafeIdle;
        STIM_ENABLE_PIN.set_low();
    }
}

/// Global interlock instance (singleton for safety-critical path)
static mut INTERLOCK: Option<Interlock> = None;

/// Initialize global interlock
pub fn init_interlock() {
    unsafe {
        INTERLOCK = Some(Interlock::new());
        if let Some(ref mut il) = INTERLOCK {
            il.init_gpio();
        }
    }
}

/// Get mutable reference to global interlock
///
/// # Safety
/// Must be called after init_interlock().
pub unsafe fn interlock_mut() -> &'static mut Interlock {
    INTERLOCK.as_mut().unwrap()
}
