//! SPSC Ring Buffer Tests
//!
//! Verify Theorem 6.3 (memory ordering correctness).

use axonos_kernel::ringbuf::*;

#[test]
fn test_push_pop_single() {
    let ring = SpscRingBuffer::<u32>::new();
    ring.try_push(42).unwrap();
    assert_eq!(ring.try_pop().unwrap(), 42);
}

#[test]
fn test_fifo_order() {
    let ring = SpscRingBuffer::<u32>::new();

    for i in 0..10 {
        ring.try_push(i).unwrap();
    }

    for i in 0..10 {
        assert_eq!(ring.try_pop().unwrap(), i);
    }
}

#[test]
fn test_capacity_limit() {
    let ring = SpscRingBuffer::<u32>::new();
    let cap = 64;

    // Fill to capacity
    for i in 0..cap {
        assert!(ring.try_push(i as u32).is_ok());
    }

    // Next push fails
    assert!(ring.try_push(999).is_err());

    // Pop one
    assert_eq!(ring.try_pop().unwrap(), 0);

    // Now push succeeds
    assert!(ring.try_push(999).is_ok());
}

#[test]
fn test_empty_underrun() {
    let ring = SpscRingBuffer::<u32>::new();
    assert!(ring.try_pop().is_err());
}

#[test]
fn test_len_tracking() {
    let ring = SpscRingBuffer::<u32>::new();
    assert_eq!(ring.len(), 0);

    ring.try_push(1).unwrap();
    assert_eq!(ring.len(), 1);

    ring.try_push(2).unwrap();
    assert_eq!(ring.len(), 2);

    ring.try_pop().unwrap();
    assert_eq!(ring.len(), 1);
}
