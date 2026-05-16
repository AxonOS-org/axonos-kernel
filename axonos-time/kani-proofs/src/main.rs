// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # Kani BMC harnesses for axonos-time
//!
//! Verifies the saturating-arithmetic and monotonicity invariants of the
//! [`Instant`]/[`Micros`] types. These are foundational: every higher-
//! layer scheduler property depends on the time arithmetic being
//! overflow-safe and direction-preserving.

#![cfg_attr(kani, no_std)]

#[cfg(kani)]
use axonos_time::{Instant, Micros, MockClock, MonotonicClock};

// ───────────────────────────────────────────────────────────────────────────
// T1: Instant::add_micros never decreases
// ───────────────────────────────────────────────────────────────────────────

/// **T1.** For any `Instant t` and `Micros d`, `t.add_micros(d) >= t`.
///
/// This is the monotonicity property that the scheduler relies on:
/// adding a non-negative duration to a timestamp never moves it
/// backwards.
#[cfg(kani)]
#[kani::proof]
fn time_t1_add_micros_monotone() {
    let t_us: u64 = kani::any();
    let d_us: u32 = kani::any();

    let t = Instant(t_us);
    let d = Micros(d_us);

    let later = t.add_micros(d);
    assert!(later >= t, "T1: add_micros must never decrease Instant");
}

// ───────────────────────────────────────────────────────────────────────────
// T2: saturating_since is non-negative
// ───────────────────────────────────────────────────────────────────────────

/// **T2.** For any two instants `a, b`, `b.saturating_since(a) >= Micros::ZERO`.
///
/// Trivial by type (Micros is unsigned), but the saturation behaviour at
/// `b < a` is the substantive content: saturate to zero rather than
/// underflow.
#[cfg(kani)]
#[kani::proof]
fn time_t2_saturating_since_non_negative() {
    let a_us: u64 = kani::any();
    let b_us: u64 = kani::any();

    let a = Instant(a_us);
    let b = Instant(b_us);

    let elapsed = b.saturating_since(a);
    // Trivially true by Micros being non-negative, but check that the
    // computation does not panic on any input.
    assert!(elapsed.as_micros() <= u32::MAX);

    // Substantive: if a > b, elapsed is zero.
    if a_us > b_us {
        assert!(
            elapsed == Micros::ZERO,
            "T2: backward direction saturates to zero"
        );
    }
}

// ───────────────────────────────────────────────────────────────────────────
// T3: saturating_since exact-equals when in-range
// ───────────────────────────────────────────────────────────────────────────

/// **T3.** When the difference between two instants fits in `u32`,
/// `saturating_since` returns the exact difference (no quantisation, no
/// rounding).
#[cfg(kani)]
#[kani::proof]
fn time_t3_saturating_since_exact_when_in_range() {
    let a_us: u64 = kani::any();
    let delta: u32 = kani::any();

    // Constrain a so a + delta does not overflow u64. With delta <= u32::MAX
    // and a_us <= u64::MAX - u32::MAX, a + delta is always within u64.
    kani::assume(a_us <= u64::MAX - u32::MAX as u64);

    let a = Instant(a_us);
    let b = Instant(a_us + u64::from(delta));

    let elapsed = b.saturating_since(a);
    assert!(
        elapsed == Micros(delta),
        "T3: exact difference when in u32 range"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// T4: saturating_since saturates at large difference
// ───────────────────────────────────────────────────────────────────────────

/// **T4.** When the difference exceeds `u32::MAX`, `saturating_since`
/// returns `Micros::MAX` (not a truncation, not an overflow).
#[cfg(kani)]
#[kani::proof]
fn time_t4_saturating_since_saturates_at_large_diff() {
    let a_us: u64 = kani::any();
    let extra: u64 = kani::any();

    // Tight bounds: prove the saturating behaviour for a representative
    // band just above u32::MAX. Full u64 space is unnecessary and explodes
    // the solver.
    kani::assume(a_us <= 1_000);
    kani::assume(extra > u32::MAX as u64);
    kani::assume(extra <= u32::MAX as u64 + 1_000);

    let a = Instant(a_us);
    let b = Instant(a_us + extra);

    let elapsed = b.saturating_since(a);
    assert!(
        elapsed == Micros::MAX,
        "T4: large differences saturate to Micros::MAX"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// T5: Micros saturating_add is monotone
// ───────────────────────────────────────────────────────────────────────────

/// **T5.** For any `Micros a, b`, `a.saturating_add(b) >= a` and `>= b`.
#[cfg(kani)]
#[kani::proof]
fn time_t5_micros_add_monotone() {
    let a_us: u32 = kani::any();
    let b_us: u32 = kani::any();

    let a = Micros(a_us);
    let b = Micros(b_us);

    let sum = a.saturating_add(b);
    assert!(sum >= a, "T5: sum >= a");
    assert!(sum >= b, "T5: sum >= b");
}

// ───────────────────────────────────────────────────────────────────────────
// T6: MockClock now() reflects the most recent advance
// ───────────────────────────────────────────────────────────────────────────

/// **T6.** After `MockClock::starting_at(t).advance(d).now()`, the
/// observed instant is exactly `t.add_micros(d)`.
#[cfg(kani)]
#[kani::proof]
fn time_t6_mock_clock_advance_observable() {
    let t_us: u64 = kani::any();
    let d_us: u32 = kani::any();

    // Constrain to keep arithmetic well-defined.
    kani::assume(t_us <= u64::MAX - u32::MAX as u64);

    let c = MockClock::starting_at(Instant(t_us));
    c.advance(Micros(d_us));
    let observed = c.now();

    assert!(
        observed == Instant(t_us.saturating_add(u64::from(d_us))),
        "T6: MockClock advance is observable"
    );
}

fn main() {
    // This binary exists to host Kani harnesses. Under non-Kani builds,
    // it is inert; under cargo kani, the harness functions below are run.
    #[cfg(not(kani))]
    eprintln!("axonos-time: Kani harness collection. Run with: cargo kani");
}
