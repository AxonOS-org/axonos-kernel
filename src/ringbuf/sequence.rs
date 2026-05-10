//! Sequence Number Protocol
//!
//! Definition 6.1: Slot i of capacity-N ring (N = 2^k) has state:
//! - Free: seq_i = i
//! - Published: seq_i = i + 1
//! - Consumed: seq_i = i + N

use core::sync::atomic::{AtomicUsize, Ordering};

/// Sequence number for ring buffer slot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceNumber(pub usize);

impl SequenceNumber {
    /// Check if slot is Free (seq == index)
    pub fn is_free(self, index: usize) -> bool {
        self.0 == index
    }

    /// Check if slot is Published (seq == index + 1)
    pub fn is_published(self, index: usize) -> bool {
        self.0 == index + 1
    }

    /// Check if slot is Consumed (seq == index + N)
    pub fn is_consumed(self, index: usize, capacity: usize) -> bool {
        self.0 == index + capacity
    }

    /// Transition: Free -> Published
    pub fn publish(self) -> Self {
        SequenceNumber(self.0 + 1)
    }

    /// Transition: Published -> Consumed
    pub fn consume(self, capacity: usize) -> Self {
        SequenceNumber(self.0 + capacity - 1)
    }

    /// Get slot index from sequence number
    pub fn slot_index(self, capacity: usize) -> usize {
        self.0 % capacity
    }
}

/// Atomic sequence number with Release-Acquire ordering
pub struct AtomicSequence {
    inner: AtomicUsize,
}

impl AtomicSequence {
    /// Create new atomic sequence
    pub const fn new(value: usize) -> Self {
        Self {
            inner: AtomicUsize::new(value),
        }
    }

    /// Load with Acquire ordering (consumer side)
    ///
    /// synchronizes-with: producer's Release store
    pub fn load_acquire(&self) -> SequenceNumber {
        SequenceNumber(self.inner.load(Ordering::Acquire))
    }

    /// Store with Release ordering (producer side)
    ///
    /// Establishes synchronizes-with relationship with consumer's Acquire load
    pub fn store_release(&self, seq: SequenceNumber) {
        self.inner.store(seq.0, Ordering::Release);
    }

    /// Compare-and-swap with Acquire-Release ordering
    pub fn compare_exchange(
        &self,
        current: SequenceNumber,
        new: SequenceNumber,
    ) -> Result<SequenceNumber, SequenceNumber> {
        match self.inner.compare_exchange(
            current.0,
            new.0,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(prev) => Ok(SequenceNumber(prev)),
            Err(prev) => Err(SequenceNumber(prev)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_transitions() {
        let seq = SequenceNumber(5);
        assert!(seq.is_free(5));
        assert!(!seq.is_published(5));

        let published = seq.publish();
        assert!(published.is_published(5));

        let consumed = published.consume(64);
        assert!(consumed.is_consumed(5, 64));
    }

    #[test]
    fn test_slot_index() {
        let seq = SequenceNumber(70);
        assert_eq!(seq.slot_index(64), 6);
    }
}
