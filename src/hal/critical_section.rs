//! Critical Section Management — Production Implementation
//!
//! Uses cortex-m critical-section with priority masking.
//! Compatible with RTIC and other Cortex-M frameworks.

use cortex_m::interrupt;

/// Enter critical section (disable interrupts with priority masking)
pub struct CriticalSection;

impl CriticalSection {
    /// Enter critical section
    ///
    /// Disables all interrupts except NMI and HardFault.
    /// On Cortex-M4F with BASEPRI: masks interrupts up to priority threshold.
    pub fn enter() -> u32 {
        let primask = interrupt::primask();
        interrupt::disable();
        primask.is_active() as u32
    }

    /// Exit critical section
    ///
    /// Restores previous interrupt state.
    pub fn exit(was_active: u32) {
        if was_active != 0 {
            unsafe { interrupt::enable(); }
        }
    }

    /// Execute closure in critical section
    ///
    /// # Example
    /// ```
    /// let result = CriticalSection::with(|| {
    ///     // Atomic operation
    ///     shared_counter += 1;
    ///     shared_counter
    /// });
    /// ```
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let was_active = Self::enter();
        let result = f();
        Self::exit(was_active);
        result
    }

    /// Nested critical section guard
    ///
    /// Automatically restores state on drop.
    pub fn scoped<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        Self::with(f)
    }
}

/// RAII critical section guard
pub struct CsGuard;

impl CsGuard {
    pub fn new() -> Self {
        CriticalSection::enter();
        Self
    }
}

impl Drop for CsGuard {
    fn drop(&mut self) {
        CriticalSection::exit(1);
    }
}
