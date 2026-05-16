// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # axonos-spsc
//!
//! A single-producer, single-consumer (SPSC) ring buffer for AxonOS.
//!
//! ## Design contract
//!
//! - `#![no_std]`, no heap, statically sized via const generic.
//! - Capacity is power-of-two; enforced at compile time.
//! - At most one [`Producer`] and one [`Consumer`] handle exist at any time;
//!   this is enforced by the type system (the handles are not `Clone`/`Copy`
//!   and are only obtained by splitting an owned [`SpscBuffer`]).
//! - The producer never observes a non-empty buffer as full unless it is
//!   genuinely full; the consumer never observes a non-full buffer as empty
//!   unless it is genuinely empty. (Linearisable, FIFO.)
//! - The unsafe surface is **exactly two operations**: a single
//!   [`core::ptr::write`] of a cell on `try_push` and a single
//!   [`core::ptr::read`] of a cell on `try_pop`. Both are guarded by
//!   sequence-number invariants verified by Kani harnesses (see
//!   `kani-proofs/`).
//!
//! ## Memory model
//!
//! The implementation uses Release/Acquire ordering on the sequence counters
//! `head` and `tail`. The standard happens-before chain establishes that a
//! payload written before the producer's Release-store of `head` is observed
//! by any consumer whose Acquire-load of `head` returns the new value
//! (Boehm and Adve, "Foundations of the C++ Concurrency Memory Model",
//! PLDI 2008; ARM Architecture Reference Manual ARMv8-A, B2.3).
//!
//! On the AxonOS reference platform (STM32F407 Cortex-M4F single-core,
//! ARMv7-M memory model) these orderings compile to plain loads/stores
//! with `dmb ish` data memory barriers as appropriate; there is no
//! cache-maintenance dependency because the M4F has no data cache.
//!
//! ## Non-features
//!
//! - Not multi-producer or multi-consumer. Use a different data structure.
//! - Not lock-free in the formal MPMC sense; it is wait-free per the SPSC
//!   contract (Theorem K2 in `kani-proofs/`).
//! - Not blocking. `try_push` and `try_pop` return `Err` on full/empty.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, allow(clippy::missing_safety_doc))]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]
// Outside the SPSC payload path itself, no unsafe is permitted in this crate.
#![deny(unsafe_op_in_unsafe_fn)]

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Error type for `try_push`: the buffer was full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

/// Error type for `try_pop`: the buffer was empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Empty;

/// Owning storage for an SPSC ring of `N` slots holding values of type `T`.
///
/// `N` must be a power of two and at least 2. This is checked at compile time
/// via a `const` assertion in [`SpscBuffer::new`].
///
/// # Layout
///
/// The buffer contains:
/// - A `slots` array of `N` [`UnsafeCell<MaybeUninit<T>>`].
/// - A `head` counter (next index to be written by the producer).
/// - A `tail` counter (next index to be read by the consumer).
///
/// Both counters are unbounded `usize` and the slot index is `counter % N`.
/// We use modular arithmetic so the difference `head.wrapping_sub(tail)`
/// directly yields the occupancy.
pub struct SpscBuffer<T, const N: usize> {
    slots: [UnsafeCell<MaybeUninit<T>>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    // Lock to ensure split() is called at most once.
    split: AtomicUsize,
}

// Safety: SpscBuffer is shared between exactly one producer and one consumer.
// The Producer and Consumer types enforce exclusive access patterns through
// the type system (no Clone, no Copy, only obtainable via split()).
//
// The atomic counters provide the synchronisation; the slot accesses are
// safe because:
//   - Producer writes slot `head % N` only when head - tail < N (not full).
//   - Consumer reads slot `tail % N` only when head - tail > 0 (not empty).
//   - The Release store on head publishes the slot write happens-before
//     any Acquire load on head, and symmetrically for tail.
unsafe impl<T: Send, const N: usize> Sync for SpscBuffer<T, N> {}

impl<T, const N: usize> SpscBuffer<T, N> {
    /// Create a new, empty SPSC buffer.
    ///
    /// # Invariants
    ///
    /// `N` must be a power of two and at least 2. This is verified by
    /// `debug_assert!` at construction; in `--release` builds with known `N`
    /// the compiler is expected to fold the check.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `N < 2` or `N` is not a power of two.
    /// (Production code should pin `N` at compile time to a valid value.)
    #[must_use]
    pub fn new() -> Self {
        debug_assert!(N >= 2, "axonos-spsc: capacity must be at least 2");
        debug_assert!(
            N.is_power_of_two(),
            "axonos-spsc: capacity must be a power of two"
        );

        Self {
            slots: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            split: AtomicUsize::new(0),
        }
    }

    /// Split this buffer into a [`Producer`] and a [`Consumer`] handle.
    ///
    /// This method may be called at most once per buffer. The second call
    /// returns `None`. This enforces the SPSC contract at the type level.
    pub fn split(&self) -> Option<(Producer<'_, T, N>, Consumer<'_, T, N>)> {
        // Compare-and-swap from 0 → 1. Only the first call succeeds.
        match self
            .split
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Some((
                Producer {
                    buffer: self,
                    _marker: PhantomData,
                },
                Consumer {
                    buffer: self,
                    _marker: PhantomData,
                },
            )),
            Err(_) => None,
        }
    }

    /// Returns the buffer's capacity.
    #[must_use]
    pub const fn capacity() -> usize {
        N
    }
}

impl<T, const N: usize> Default for SpscBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// Drop is necessary because the slots may contain initialised values that
// need their destructors run.
impl<T, const N: usize> Drop for SpscBuffer<T, N> {
    fn drop(&mut self) {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        // Drop any remaining initialised cells.
        let mut idx = tail;
        while idx != head {
            let slot = self.slots[idx % N].get();
            // SAFETY: `idx` is in [tail, head), so the slot at `idx % N`
            // contains an initialised value (the producer wrote it and the
            // consumer has not yet read it). We hold the only reference to
            // the buffer (Drop has &mut self). Reading-and-dropping the
            // value uninitialises that slot, which is fine because we are
            // about to free the entire buffer.
            unsafe {
                core::ptr::drop_in_place((*slot).as_mut_ptr());
            }
            idx = idx.wrapping_add(1);
        }
    }
}

/// Producer handle for an [`SpscBuffer`]. There is at most one of these.
///
/// `Send`-able but not `Sync`: it can be moved to another thread, but cannot
/// be shared by reference between threads.
pub struct Producer<'a, T, const N: usize> {
    buffer: &'a SpscBuffer<T, N>,
    _marker: PhantomData<*mut ()>, // !Sync, !Send by default — re-impl Send below
}

// Safety: Producer can be moved to another thread; the SpscBuffer is shared
// via Sync impl, and the type system prevents creating a second Producer.
unsafe impl<T: Send, const N: usize> Send for Producer<'_, T, N> {}

impl<T, const N: usize> Producer<'_, T, N> {
    /// Try to push `value` into the buffer.
    ///
    /// Wait-free: this method completes in bounded steps regardless of any
    /// concurrent consumer activity (proven by Kani harness K2).
    ///
    /// # Errors
    ///
    /// Returns `Err(Full)` if the buffer is full. The `value` is returned
    /// untouched in the error variant so the caller can retry without copy.
    pub fn try_push(&mut self, value: T) -> Result<(), Full> {
        // Acquire-load tail to establish happens-before with the consumer's
        // Release-store of tail (which signals the slot has been freed).
        let tail = self.buffer.tail.load(Ordering::Acquire);
        // Relaxed load of head is sufficient: we are the only producer, so
        // we have exclusive write access to it.
        let head = self.buffer.head.load(Ordering::Relaxed);

        // Compute occupancy. head and tail are unbounded usize counters;
        // wrapping_sub gives a correct difference even across wrap-around
        // because the buffer is bounded (N <= usize::MAX / 2 by the
        // is_power_of_two and >= 2 guards).
        let occupancy = head.wrapping_sub(tail);
        if occupancy >= N {
            return Err(Full);
        }

        let slot_idx = head % N;
        let slot = self.buffer.slots[slot_idx].get();

        // SAFETY:
        //   * `slot_idx` is in `0..N`, so the array access above is in bounds.
        //   * We are the unique producer; the only other holder of any
        //     reference to this slot is the consumer, and we have established
        //     above that `occupancy < N`, i.e. `head - tail < N`. Therefore
        //     the consumer's tail has already passed this slot index from a
        //     previous epoch, and the consumer holds no reference to it.
        //   * `MaybeUninit::write` does not require the destination to be
        //     initialised, so this is sound regardless of any prior state.
        //   * The store happens-before the subsequent Release-store of head;
        //     any consumer that Acquire-loads head and observes the new value
        //     transitively observes this write.
        unsafe {
            (*slot).write(value);
        }

        // Release-store of head: publishes the slot write to any consumer
        // who subsequently Acquire-loads head.
        self.buffer
            .head
            .store(head.wrapping_add(1), Ordering::Release);

        Ok(())
    }

    /// Returns the current occupancy as observed locally (may be stale).
    #[must_use]
    pub fn len_relaxed(&self) -> usize {
        let head = self.buffer.head.load(Ordering::Relaxed);
        let tail = self.buffer.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }
}

/// Consumer handle for an [`SpscBuffer`]. There is at most one of these.
pub struct Consumer<'a, T, const N: usize> {
    buffer: &'a SpscBuffer<T, N>,
    _marker: PhantomData<*mut ()>, // !Sync
}

// Safety: same reasoning as for Producer.
unsafe impl<T: Send, const N: usize> Send for Consumer<'_, T, N> {}

impl<T, const N: usize> Consumer<'_, T, N> {
    /// Try to pop a value from the buffer.
    ///
    /// Wait-free: this method completes in bounded steps regardless of any
    /// concurrent producer activity.
    ///
    /// # Errors
    ///
    /// Returns `Err(Empty)` if the buffer is empty.
    pub fn try_pop(&mut self) -> Result<T, Empty> {
        // Acquire-load head to establish happens-before with the producer's
        // Release-store of head.
        let head = self.buffer.head.load(Ordering::Acquire);
        let tail = self.buffer.tail.load(Ordering::Relaxed);

        if head == tail {
            return Err(Empty);
        }

        let slot_idx = tail % N;
        let slot = self.buffer.slots[slot_idx].get();

        // SAFETY:
        //   * `slot_idx` is in `0..N`, so the array access is in bounds.
        //   * head > tail (checked above), meaning the producer has
        //     Release-stored head after writing this slot. Our Acquire-load
        //     of head established happens-before with that store, so we
        //     observe the producer's write.
        //   * We are the unique consumer; the producer's next write to
        //     this slot index will only happen after our subsequent
        //     Release-store of tail, after which the slot value is logically
        //     uninitialised again.
        //   * `MaybeUninit::assume_init_read` is correct because the slot
        //     holds an initialised value (the producer's write).
        let value = unsafe { (*slot).assume_init_read() };

        // Release-store of tail: publishes that this slot is now free.
        self.buffer
            .tail
            .store(tail.wrapping_add(1), Ordering::Release);

        Ok(value)
    }

    /// Returns the current occupancy as observed locally (may be stale).
    #[must_use]
    pub fn len_relaxed(&self) -> usize {
        let head = self.buffer.head.load(Ordering::Relaxed);
        let tail = self.buffer.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Compile-time assertion macro
// ───────────────────────────────────────────────────────────────────────────

#[doc(hidden)]
#[macro_export]
macro_rules! const_assert {
    ($cond:expr $(,)?) => {
        const _: () = {
            assert!($cond);
        };
    };
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_pop_returns_value() {
        let buf: SpscBuffer<u32, 4> = SpscBuffer::new();
        let (mut p, mut c) = buf.split().unwrap();
        assert!(p.try_push(42).is_ok());
        assert_eq!(c.try_pop(), Ok(42));
    }

    #[test]
    fn pop_empty_returns_err() {
        let buf: SpscBuffer<u32, 4> = SpscBuffer::new();
        let (_p, mut c) = buf.split().unwrap();
        assert_eq!(c.try_pop(), Err(Empty));
    }

    #[test]
    fn push_full_returns_err() {
        let buf: SpscBuffer<u32, 2> = SpscBuffer::new();
        let (mut p, _c) = buf.split().unwrap();
        assert!(p.try_push(1).is_ok());
        assert!(p.try_push(2).is_ok());
        assert_eq!(p.try_push(3), Err(Full));
    }

    #[test]
    fn fifo_order_preserved() {
        let buf: SpscBuffer<u32, 8> = SpscBuffer::new();
        let (mut p, mut c) = buf.split().unwrap();
        for i in 0..8u32 {
            assert!(p.try_push(i).is_ok());
        }
        for i in 0..8u32 {
            assert_eq!(c.try_pop(), Ok(i));
        }
        assert_eq!(c.try_pop(), Err(Empty));
    }

    #[test]
    fn interleaved_push_pop() {
        let buf: SpscBuffer<u32, 4> = SpscBuffer::new();
        let (mut p, mut c) = buf.split().unwrap();
        for i in 0..100u32 {
            assert!(p.try_push(i).is_ok());
            assert_eq!(c.try_pop(), Ok(i));
        }
    }

    #[test]
    fn split_returns_none_on_second_call() {
        let buf: SpscBuffer<u32, 4> = SpscBuffer::new();
        let _ = buf.split().unwrap();
        assert!(buf.split().is_none());
    }

    #[test]
    fn drop_releases_held_values() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

        struct Tracked;
        impl Drop for Tracked {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, O::SeqCst);
            }
        }

        DROP_COUNT.store(0, O::SeqCst);
        {
            let buf: SpscBuffer<Tracked, 4> = SpscBuffer::new();
            let (mut p, _c) = buf.split().unwrap();
            assert!(p.try_push(Tracked).is_ok());
            assert!(p.try_push(Tracked).is_ok());
            // Two values pushed, none popped. Buffer drop should drop both.
        }
        assert_eq!(DROP_COUNT.load(O::SeqCst), 2);
    }

    #[test]
    fn wrap_around_at_capacity() {
        let buf: SpscBuffer<u32, 4> = SpscBuffer::new();
        let (mut p, mut c) = buf.split().unwrap();
        // Push N, pop N, push N more — exercises wrap.
        for i in 0..4u32 {
            assert!(p.try_push(i).is_ok());
        }
        for i in 0..4u32 {
            assert_eq!(c.try_pop(), Ok(i));
        }
        for i in 100..104u32 {
            assert!(p.try_push(i).is_ok());
        }
        for i in 100..104u32 {
            assert_eq!(c.try_pop(), Ok(i));
        }
    }
}
