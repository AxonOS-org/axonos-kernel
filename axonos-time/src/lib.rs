// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # axonos-time
//!
//! Monotonic time abstraction for the AxonOS kernel.
//!
//! ## Scope
//!
//! This crate provides:
//!
//! - The [`Instant`] type — a 64-bit microsecond timestamp on a
//!   monotonic, non-decreasing time axis with no defined relation to
//!   wall-clock time.
//! - The [`Micros`] type — a 32-bit microsecond duration.
//! - The [`MonotonicClock`] trait — the abstraction over hardware time
//!   sources. Implementations bind the trait to a concrete clock
//!   (Cortex-M DWT, RISC-V `mcycle`, `std::time::Instant`, or a mock).
//! - The [`MockClock`] type — a controllable in-memory clock for testing
//!   and bounded model checking.
//! - The [`StdClock`] type (behind the `std` feature) — backed by
//!   `std::time::Instant` for host-side testing.
//!
//! Hardware-specific clocks (DWT, mcycle, system tick) are *not* provided
//! by this crate. Each consumer crate or kernel binary is expected to
//! implement [`MonotonicClock`] against its specific hardware. The
//! trait is the integration point; the implementations are
//! deliberately not coupled into a single repository.
//!
//! ## Design discipline
//!
//! - All arithmetic is saturating. No silent wrap-around, no panic on
//!   overflow.
//! - All construction is `const fn` where possible.
//! - No allocator, no `unsafe`, no dependencies.
//! - The hot-path operation `now()` is documented as wait-free; the
//!   trait does not specify performance but contract-level wait-freedom
//!   is the expected pattern.
//!
//! ## Wire format
//!
//! [`Instant`] is the same `u64`-microsecond representation used by the
//! `timestamp_us` field of [`IntentObservation`](https://github.com/AxonOS-org/axonos-rfcs/blob/main/rfcs/0006-intent-wire-format-abi.md)
//! per RFC-0006. An `Instant` value above
//! [`Instant::SESSION_MAX_REASONABLE`] indicates either a corrupted
//! input or a session that has exceeded the documented operational
//! envelope (≈ 8.9 years at microsecond resolution); receivers should
//! reject such inputs per the RFC.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

// AtomicU64 doesn't exist on 32-bit ARM (Cortex-M4F, etc.). MockClock
// is a test helper anyway — gate it behind `target_has_atomic = "64"`
// so embedded targets simply don't see it.
#[cfg(target_has_atomic = "64")]
use core::sync::atomic::{AtomicU64, Ordering};

// ───────────────────────────────────────────────────────────────────────────
// Micros (duration)
// ───────────────────────────────────────────────────────────────────────────

/// A microsecond-resolution duration, non-negative, up to ≈ 71 minutes.
///
/// `Micros` is sized `u32` because virtually every real-time deadline in
/// the AxonOS pipeline fits comfortably within 32 bits. The maximum
/// representable duration (`u32::MAX` microseconds) is approximately
/// 71 minutes, which exceeds any closed-loop deadline by orders of
/// magnitude. For long absolute time spans, use the difference of two
/// [`Instant`] values, which produces a `Micros` that saturates to
/// `Micros::MAX` if the underlying span exceeds the `u32` range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Micros(pub u32);

impl Micros {
    /// Zero duration.
    pub const ZERO: Self = Self(0);

    /// Maximum representable duration (≈ 71 minutes).
    pub const MAX: Self = Self(u32::MAX);

    /// One microsecond.
    pub const ONE: Self = Self(1);

    /// Construct from a `u32` microsecond count.
    #[must_use]
    pub const fn from_micros(us: u32) -> Self {
        Self(us)
    }

    /// Construct from a `u32` millisecond count, saturating at `Micros::MAX`.
    #[must_use]
    pub const fn from_millis(ms: u32) -> Self {
        Self(ms.saturating_mul(1_000))
    }

    /// Returns the underlying `u32` microsecond count.
    #[must_use]
    pub const fn as_micros(self) -> u32 {
        self.0
    }

    /// Saturating addition.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction. Result is non-negative.
    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Multiply by an integer factor, saturating.
    #[must_use]
    pub const fn saturating_mul(self, factor: u32) -> Self {
        Self(self.0.saturating_mul(factor))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Instant (absolute time)
// ───────────────────────────────────────────────────────────────────────────

/// A monotonic absolute time, in microseconds since some implementation-
/// defined epoch (typically boot or session start).
///
/// The wire format is `u64` little-endian per RFC-0006. The
/// representation is large enough to span ≈ 584 000 years at
/// microsecond resolution; in practice an `Instant` value above
/// [`Self::SESSION_MAX_REASONABLE`] indicates either input corruption
/// or an unsupported operational regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Instant(pub u64);

impl Instant {
    /// The reference origin. Implementation defined per clock; typically
    /// boot or session start.
    pub const ZERO: Self = Self(0);

    /// The maximum reasonable session duration: 2^48 microseconds,
    /// approximately 8.9 years.
    ///
    /// Receivers should reject any timestamp exceeding this bound per
    /// RFC-0006. This is the same constant exposed as
    /// `SESSION_MAX_REASONABLE_US` in `axonos-sdk`.
    pub const SESSION_MAX_REASONABLE: Self = Self(1u64 << 48);

    /// Returns the underlying `u64` microsecond count.
    #[must_use]
    pub const fn as_micros(self) -> u64 {
        self.0
    }

    /// Returns this instant plus a duration. Saturates at `u64::MAX` on
    /// overflow; in practice no AxonOS session approaches that bound.
    #[must_use]
    pub const fn add_micros(self, d: Micros) -> Self {
        Self(self.0.saturating_add(d.0 as u64))
    }

    /// Returns the duration since `earlier`, saturating at zero if
    /// `earlier` is in the future of `self`, and saturating at
    /// `Micros::MAX` if the difference exceeds `u32::MAX` microseconds.
    ///
    /// Saturation at zero is deliberate: a real-time scheduler must not
    /// observe negative durations under any input. If the clock is
    /// non-monotonic (which would be a bug in the implementation), the
    /// scheduler interprets the violation as a zero-elapsed reading, not
    /// as an underflow.
    #[must_use]
    pub const fn saturating_since(self, earlier: Self) -> Micros {
        let diff = self.0.saturating_sub(earlier.0);
        if diff > u32::MAX as u64 {
            Micros::MAX
        } else {
            // The cast is safe: `diff <= u32::MAX`.
            #[allow(clippy::cast_possible_truncation)]
            Micros(diff as u32)
        }
    }

    /// Returns whether this instant is within the documented session
    /// envelope (≤ [`Self::SESSION_MAX_REASONABLE`]).
    ///
    /// Per RFC-0006, receivers MUST reject any timestamp exceeding this
    /// bound. Implementations of [`MonotonicClock`] that produce values
    /// outside this envelope are reporting either a corrupted state or
    /// a configuration error.
    #[must_use]
    pub const fn is_within_session_envelope(self) -> bool {
        self.0 <= Self::SESSION_MAX_REASONABLE.0
    }
}

// ───────────────────────────────────────────────────────────────────────────
// MonotonicClock trait
// ───────────────────────────────────────────────────────────────────────────

/// The abstraction over a monotonic time source.
///
/// ## Contract
///
/// An implementation of `MonotonicClock` SHALL satisfy:
///
/// 1. **Monotonicity.** For any two calls to `now()` on the same clock
///    instance in program order, the second call returns an `Instant`
///    `>=` the first.
/// 2. **Resolution.** The resolution is implementation-defined, but
///    SHALL be at most 1 microsecond on the AxonOS reference hardware
///    (STM32F407 at 168 MHz: DWT cycle counter gives ≈ 5.95 ns
///    resolution, which is finer than required).
/// 3. **Wait-freedom on the hot path.** `now()` SHOULD complete in
///    bounded steps independent of any other concurrent activity. The
///    trait does not specify a numeric bound, but the AxonOS reference
///    DWT implementation completes in ≤ 6 CPU cycles.
/// 4. **No allocation.** Implementations SHALL NOT allocate from the
///    heap on `now()`.
///
/// ## Implementations
///
/// This crate provides:
/// - [`MockClock`] — a controllable in-memory clock for testing.
/// - [`StdClock`] — a host-side clock backed by `std::time::Instant`
///   (under the `std` feature).
///
/// Hardware-specific implementations (Cortex-M DWT, RISC-V `mcycle`)
/// are deliberately left to consumer crates. An example DWT
/// implementation is given in the crate-level documentation.
pub trait MonotonicClock {
    /// Returns the current monotonic instant.
    fn now(&self) -> Instant;

    /// Returns the duration elapsed since `earlier`.
    ///
    /// Default implementation calls `now()` and computes
    /// `now.saturating_since(earlier)`. Implementations may override
    /// this if the underlying hardware provides a more efficient
    /// elapsed-time primitive.
    #[inline]
    fn elapsed_since(&self, earlier: Instant) -> Micros {
        self.now().saturating_since(earlier)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// MockClock — controllable clock for tests and BMC
// ───────────────────────────────────────────────────────────────────────────

/// A controllable in-memory clock for testing.
///
/// Uses `AtomicU64` so that the clock can be advanced from one thread
/// and read from another without `&mut self`. This matches the typical
/// pattern of a scheduler that holds an `&dyn MonotonicClock` and an
/// ISR that advances the clock.
#[derive(Debug, Default)]
#[cfg(target_has_atomic = "64")]
pub struct MockClock {
    micros: AtomicU64,
}

#[cfg(target_has_atomic = "64")]
impl MockClock {
    /// Create a new mock clock starting at `Instant::ZERO`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            micros: AtomicU64::new(0),
        }
    }

    /// Create a mock clock starting at the given instant.
    #[must_use]
    pub const fn starting_at(initial: Instant) -> Self {
        Self {
            micros: AtomicU64::new(initial.0),
        }
    }

    /// Advance the clock by the given duration. Saturating.
    pub fn advance(&self, duration: Micros) {
        // Read-modify-write with relaxed ordering. The clock has no
        // synchronisation responsibility — it merely reports time.
        let current = self.micros.load(Ordering::Relaxed);
        let next = current.saturating_add(u64::from(duration.0));
        self.micros.store(next, Ordering::Relaxed);
    }

    /// Set the clock to a specific instant (test convenience only).
    pub fn set_to(&self, instant: Instant) {
        self.micros.store(instant.0, Ordering::Relaxed);
    }
}

#[cfg(target_has_atomic = "64")]
impl MonotonicClock for MockClock {
    fn now(&self) -> Instant {
        Instant(self.micros.load(Ordering::Relaxed))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// StdClock — host-side clock (gated by `std` feature)
// ───────────────────────────────────────────────────────────────────────────

#[cfg(feature = "std")]
mod std_clock {
    use super::{Instant, MonotonicClock};

    /// A host-side clock backed by `std::time::Instant`.
    ///
    /// Available only with the `std` feature. Intended for integration
    /// tests that run on the host, not for embedded use.
    #[derive(Debug)]
    pub struct StdClock {
        epoch: std::time::Instant,
    }

    impl StdClock {
        /// Create a new `StdClock` anchored at the current `std::time::Instant`.
        #[must_use]
        pub fn new() -> Self {
            Self {
                epoch: std::time::Instant::now(),
            }
        }
    }

    impl Default for StdClock {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MonotonicClock for StdClock {
        fn now(&self) -> Instant {
            // The elapsed duration as a u128 nanoseconds, divided to microseconds.
            let elapsed_ns = self.epoch.elapsed().as_nanos();
            // Saturating cast: 2^64 us ≈ 584 000 years; well beyond any test.
            let us = u64::try_from(elapsed_ns / 1_000).unwrap_or(u64::MAX);
            Instant(us)
        }
    }
}

#[cfg(feature = "std")]
pub use std_clock::StdClock;

// ───────────────────────────────────────────────────────────────────────────
// Example DWT implementation (documentation only, not compiled)
// ───────────────────────────────────────────────────────────────────────────

/// Example implementation of [`MonotonicClock`] using the ARM Cortex-M
/// Data Watchpoint and Trace cycle counter.
///
/// This implementation is shown in the crate documentation as a
/// reference; consumers wanting DWT-backed timing should copy this code
/// into a `#[cfg(target_arch = "arm")]`-gated module in their own crate.
/// We do not ship this implementation here because it would require a
/// `cortex-m` dependency that is meaningless on host targets.
///
/// ```ignore
/// use axonos_time::{Instant, MonotonicClock};
/// use cortex_m::peripheral::DWT;
///
/// pub struct DwtClock {
///     // CPU frequency in MHz, for cycle-to-microsecond conversion.
///     // STM32F407 reference: 168.
///     cpu_mhz: u32,
/// }
///
/// impl DwtClock {
///     /// Construct a DWT-backed clock. Requires that the trace unit and
///     /// cycle counter have been enabled via
///     /// `DWT::enable_cycle_counter` during system init.
///     pub const fn new(cpu_mhz: u32) -> Self {
///         Self { cpu_mhz }
///     }
/// }
///
/// impl MonotonicClock for DwtClock {
///     fn now(&self) -> Instant {
///         // DWT::cycle_count() is a 32-bit cycle counter; it wraps after
///         // approximately 25 seconds at 168 MHz. Production code must
///         // wrap-extend this to 64 bits using a wrap-tracking helper.
///         let cycles = DWT::cycle_count();
///         let us = u64::from(cycles) / u64::from(self.cpu_mhz);
///         Instant(us)
///     }
/// }
/// ```
///
/// In a production AxonOS deployment the wrap-tracking is handled by a
/// 32→64-bit counter that increments an extension word in a low-priority
/// interrupt running at least every half-period of the 32-bit counter.
/// The example above omits this for clarity; see the AxonOS
/// `axonos-time-dwt` integration crate (forthcoming) for the production
/// pattern.
pub mod dwt_example {}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micros_construction_and_conversion() {
        assert_eq!(Micros::from_micros(42).as_micros(), 42);
        assert_eq!(Micros::from_millis(7).as_micros(), 7_000);
        assert_eq!(Micros::ZERO.as_micros(), 0);
        assert_eq!(Micros::MAX.as_micros(), u32::MAX);
    }

    #[test]
    fn micros_arithmetic_saturates() {
        assert_eq!(
            Micros::MAX.saturating_add(Micros::ONE),
            Micros::MAX,
            "add must saturate at u32::MAX"
        );
        assert_eq!(
            Micros::ZERO.saturating_sub(Micros::ONE),
            Micros::ZERO,
            "sub must saturate at zero"
        );
        assert_eq!(
            Micros::MAX.saturating_mul(2),
            Micros::MAX,
            "mul must saturate"
        );
    }

    #[test]
    fn instant_zero_and_envelope() {
        assert_eq!(Instant::ZERO.as_micros(), 0);
        assert!(Instant::ZERO.is_within_session_envelope());
        assert!(Instant::SESSION_MAX_REASONABLE.is_within_session_envelope());
        assert!(!Instant(Instant::SESSION_MAX_REASONABLE.0 + 1).is_within_session_envelope());
    }

    #[test]
    fn instant_add_micros() {
        let t = Instant::ZERO.add_micros(Micros(1_000));
        assert_eq!(t.as_micros(), 1_000);
    }

    #[test]
    fn instant_saturating_since_normal() {
        let a = Instant(1_000);
        let b = Instant(3_500);
        assert_eq!(b.saturating_since(a), Micros(2_500));
    }

    #[test]
    fn instant_saturating_since_negative_yields_zero() {
        let a = Instant(3_500);
        let b = Instant(1_000);
        // earlier > later → saturate to zero (non-decreasing scheduler
        // sees a zero elapsed reading, not an underflow).
        assert_eq!(b.saturating_since(a), Micros::ZERO);
    }

    #[test]
    fn instant_saturating_since_large_difference() {
        let a = Instant(0);
        let b = Instant(u64::from(u32::MAX) + 1);
        // Difference exceeds u32, must saturate to Micros::MAX.
        assert_eq!(b.saturating_since(a), Micros::MAX);
    }

    #[test]
    fn mock_clock_starts_at_zero() {
        let c = MockClock::new();
        assert_eq!(c.now(), Instant::ZERO);
    }

    #[test]
    fn mock_clock_advances() {
        let c = MockClock::new();
        c.advance(Micros::from_millis(5));
        assert_eq!(c.now(), Instant(5_000));
        c.advance(Micros::from_millis(2));
        assert_eq!(c.now(), Instant(7_000));
    }

    #[test]
    fn mock_clock_elapsed_since() {
        let c = MockClock::new();
        let start = c.now();
        c.advance(Micros(450));
        assert_eq!(c.elapsed_since(start), Micros(450));
    }

    #[test]
    fn mock_clock_monotonic() {
        let c = MockClock::new();
        let a = c.now();
        c.advance(Micros(100));
        let b = c.now();
        c.advance(Micros(100));
        let d = c.now();
        assert!(a <= b);
        assert!(b <= d);
    }

    #[test]
    fn mock_clock_starting_at_custom_instant() {
        let c = MockClock::starting_at(Instant(50_000));
        assert_eq!(c.now(), Instant(50_000));
    }

    #[cfg(feature = "std")]
    #[test]
    fn std_clock_monotonic() {
        let c = StdClock::new();
        let a = c.now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = c.now();
        assert!(b >= a);
    }
}
