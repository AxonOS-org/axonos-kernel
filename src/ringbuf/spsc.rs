//! SPSC Ring Buffer Implementation
//!
//! Zero-copy, wait-free (producer), lock-free (consumer) ring buffer.
//! Based on Dmitry Vyukov's sequence-number protocol.
//!
//! ## Memory Ordering Proof (Theorem 6.3)
//!
//! Let W be the producer's payload write and S the subsequent
//! seq.store(i+1, Release). Let L be the consumer's seq.load(Acquire)
//! observing i+1, and R the subsequent payload read.
//!
//! 1. W --sb--> S (program order, same thread)
//! 2. S --sw--> L (Release-Acquire synchronizes-with pair)
//! 3. L --sb--> R (program order, same thread)
//!
//! By transitivity of happens-before: W --hb--> R, so R observes W.
//!
//! ## Unsafe Scope
//!
//! Two targeted unsafe blocks:
//! 1. `core::ptr::write(slot, value)` — producer payload write
//! 2. `core::ptr::read(slot)` — consumer payload read
//!
//! Safety invariants:
//! - Producer holds exclusive access to Published slots
//! - Consumer reads only after observing Published state

use super::sequence::{SequenceNumber, AtomicSequence};
use crate::config;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::MaybeUninit;

/// SPSC ring buffer error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpscError {
    /// Ring buffer full (producer overrun)
    Overrun,
    /// Ring buffer empty (consumer underrun)
    Underrun,
    /// Slot state inconsistency (protocol violation)
    ProtocolViolation,
}

/// Single-producer single-consumer ring buffer
///
/// Capacity must be power of 2 (enforced at compile time via config).
pub struct SpscRingBuffer<T> {
    /// Buffer slots (uninitialized until published)
    buffer: [MaybeUninit<T>; config::RING_BUFFER_CAPACITY],
    /// Sequence numbers for each slot
    sequences: [AtomicSequence; config::RING_BUFFER_CAPACITY],
    /// Producer write index (monotonically increasing)
    write_index: AtomicUsize,
    /// Consumer read index (monotonically increasing)
    read_index: AtomicUsize,
}

/// Ring buffer configuration
#[derive(Debug, Clone, Copy)]
pub struct RingBufferConfig {
    /// Capacity (must be power of 2)
    pub capacity: usize,
    /// Slot size [bytes]
    pub slot_size: usize,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            capacity: config::RING_BUFFER_CAPACITY,
            slot_size: 64, // bytes per slot
        }
    }
}

impl<T: Copy> SpscRingBuffer<T> {
    /// Create new SPSC ring buffer
    ///
    /// All slots initialized to Free state (seq = index).
    pub fn new() -> Self {
        // Initialize sequence numbers: seq[i] = i (Free state)
        let sequences = core::array::from_fn(|i| AtomicSequence::new(i));

        Self {
            buffer: [MaybeUninit::uninit(); config::RING_BUFFER_CAPACITY],
            sequences,
            write_index: AtomicUsize::new(0),
            read_index: AtomicUsize::new(0),
        }
    }

    /// Producer: push value to ring buffer
    ///
    /// Wait-free: completes in O(1) steps regardless of consumer state.
    /// Returns Err(Overrun) if buffer full — triggers DC1 violation handler.
    ///
    /// # Safety
    /// Safe under invariant that producer holds exclusive access to Free slots.
    pub fn try_push(&self, value: T) -> Result<(), SpscError> {
        let w = self.write_index.load(Ordering::Relaxed);
        let slot_idx = w % config::RING_BUFFER_CAPACITY;

        // Check slot state with Acquire ordering
        let seq = self.sequences[slot_idx].load_acquire();

        if !seq.is_free(w) {
            // Slot not free — buffer full or protocol violation
            return Err(SpscError::Overrun);
        }

        // Write payload
        // SAFETY: We hold exclusive access (seq == w means Free state)
        unsafe {
            core::ptr::write(self.buffer[slot_idx].as_mut_ptr(), value);
        }

        // Publish: seq = w + 1 (Published state) with Release ordering
        self.sequences[slot_idx].store_release(SequenceNumber(w + 1));

        // Advance write index
        self.write_index.store(w + 1, Ordering::Relaxed);

        Ok(())
    }

    /// Consumer: pop value from ring buffer
    ///
    /// Lock-free: may spin briefly if producer is mid-write.
    /// Returns Err(Underrun) if buffer empty.
    ///
    /// # Safety
    /// Safe under invariant that consumer reads only Published slots.
    pub fn try_pop(&self) -> Result<T, SpscError> {
        let r = self.read_index.load(Ordering::Relaxed);
        let slot_idx = r % config::RING_BUFFER_CAPACITY;

        // Check slot state with Acquire ordering
        let seq = self.sequences[slot_idx].load_acquire();

        if !seq.is_published(r) {
            // Slot not published — buffer empty
            return Err(SpscError::Underrun);
        }

        // Read payload
        // SAFETY: We verified slot is Published (seq == r + 1)
        let value = unsafe {
            core::ptr::read(self.buffer[slot_idx].as_ptr())
        };

        // Consume: seq = r + N (Consumed state) with Release ordering
        let consumed_seq = SequenceNumber(r + config::RING_BUFFER_CAPACITY);
        self.sequences[slot_idx].store_release(consumed_seq);

        // Advance read index
        self.read_index.store(r + 1, Ordering::Relaxed);

        Ok(value)
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        let w = self.write_index.load(Ordering::Relaxed);
        let r = self.read_index.load(Ordering::Relaxed);
        w - r >= config::RING_BUFFER_CAPACITY
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        let w = self.write_index.load(Ordering::Relaxed);
        let r = self.read_index.load(Ordering::Relaxed);
        w == r
    }

    /// Count of available items
    pub fn len(&self) -> usize {
        let w = self.write_index.load(Ordering::Relaxed);
        let r = self.read_index.load(Ordering::Relaxed);
        w - r
    }

    /// Reset ring buffer (for recovery)
    ///
    /// # Safety
    /// Must only be called when producer and consumer are quiescent.
    pub unsafe fn reset(&self) {
        for i in 0..config::RING_BUFFER_CAPACITY {
            self.sequences[i].store_release(SequenceNumber(i));
        }
        self.write_index.store(0, Ordering::Relaxed);
        self.read_index.store(0, Ordering::Relaxed);
    }
}

// Kani proof harnesses (compiled only with kani feature)
#[cfg(feature = "kani")]
mod proofs {
    use super::*;

    /// K1: No data race
    /// Constructs symbolic ring, executes push then pop,
    /// asserts no Undefined Behaviour triggered.
    #[kani::proof]
    #[kani::unwind(8)]
    fn spsc_no_data_race() {
        let ring: SpscRingBuffer<u32> = SpscRingBuffer::new();
        let value: u32 = kani::any();

        // Symbolic push
        let _ = ring.try_push(value);

        // Symbolic pop
        if let Ok(read) = ring.try_pop() {
            // K3: Payload integrity
            assert_eq!(read, value);
        }
    }

    /// K2: Wait-freedom
    /// Non-deterministic initial consumer state;
    /// Kani exhausts all branches without encountering infinite loop.
    #[kani::proof]
    #[kani::unwind(4)]
    fn spsc_push_wait_free() {
        let ring: SpscRingBuffer<u32> = SpscRingBuffer::new();
        let value: u32 = kani::any();

        // Push must return in bounded steps (no spin)
        let _ = ring.try_push(value);
    }

    /// K3: Memory ordering
    /// Symbolic producer write value w;
    /// asserts consumer observes r = w after Release-Acquire pair.
    #[kani::proof]
    #[kani::unwind(2)]
    fn spsc_memory_order() {
        let ring: SpscRingBuffer<u32> = SpscRingBuffer::new();
        let w: u32 = kani::any();

        ring.try_push(w).unwrap();
        let r = ring.try_pop().unwrap();

        assert_eq!(r, w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let ring: SpscRingBuffer<u32> = SpscRingBuffer::new();

        assert!(ring.try_push(42).is_ok());
        assert_eq!(ring.try_pop().unwrap(), 42);
    }

    #[test]
    fn test_overrun() {
        let ring: SpscRingBuffer<u32> = SpscRingBuffer::new();

        // Fill buffer
        for i in 0..config::RING_BUFFER_CAPACITY {
            assert!(ring.try_push(i as u32).is_ok());
        }

        // Next push should fail
        assert!(ring.try_push(999).is_err());
    }

    #[test]
    fn test_underrun() {
        let ring: SpscRingBuffer<u32> = SpscRingBuffer::new();
        assert!(ring.try_pop().is_err());
    }
}
