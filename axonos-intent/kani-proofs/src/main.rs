//! # Kani BMC harnesses for axonos-intent
//!
//! Verifies the round-trip and rejection properties of the
//! [`IntentObservation`] wire format against RFC-0006 §4.

#![cfg_attr(kani, no_std)]

#[cfg(kani)]
use axonos_intent::{
    AttestationTag, Confidence, DecodeError, Direction, IntentObservation, Kind,
    NavigationDirection, OBSERVATION_SIZE,
};
#[cfg(kani)]
use axonos_time::Instant;

// ───────────────────────────────────────────────────────────────────────────
// I1: encode → decode is the identity (round-trip)
// ───────────────────────────────────────────────────────────────────────────

/// **I1.** For any well-formed `IntentObservation`, `decode(encode(o)) = o`.
///
/// Establishes that the wire format is round-trip stable. Verified for
/// Navigation/Right with a non-deterministic timestamp, confidence,
/// sequence, and attestation tag.
#[cfg(kani)]
#[kani::proof]
fn intent_i1_round_trip_navigation_right() {
    let ts: u64 = kani::any();
    kani::assume(ts <= Instant::SESSION_MAX_REASONABLE.as_micros());

    let conf_raw: u16 = kani::any();
    let seq: u32 = kani::any();
    let tag: [u8; 8] = kani::any();

    let obs = IntentObservation {
        timestamp: Instant(ts),
        kind: Kind::Navigation,
        direction: Direction::Navigation(NavigationDirection::Right),
        confidence: Confidence::from_q0_16(conf_raw),
        sequence: seq,
        attestation: AttestationTag(tag),
    };

    let bytes = obs.encode();
    let decoded = IntentObservation::decode(&bytes).expect("round trip must succeed");
    assert!(decoded == obs);
}

// ───────────────────────────────────────────────────────────────────────────
// I2: every encode produces exactly 32 bytes
// ───────────────────────────────────────────────────────────────────────────

/// **I2.** The encoded length is always exactly `OBSERVATION_SIZE = 32`.
#[cfg(kani)]
#[kani::proof]
fn intent_i2_encoded_size_is_32() {
    let ts: u64 = kani::any();
    kani::assume(ts <= Instant::SESSION_MAX_REASONABLE.as_micros());

    let obs = IntentObservation {
        timestamp: Instant(ts),
        kind: Kind::Navigation,
        direction: Direction::Navigation(NavigationDirection::Idle),
        confidence: Confidence::MIN,
        sequence: 0,
        attestation: AttestationTag([0; 8]),
    };
    let bytes = obs.encode();
    assert!(bytes.len() == OBSERVATION_SIZE);
    assert!(OBSERVATION_SIZE == 32);
}

// ───────────────────────────────────────────────────────────────────────────
// I3: every kind_tag outside [0, 3] is rejected
// ───────────────────────────────────────────────────────────────────────────

/// **I3.** Any decode with `kind_tag > 3` returns `InvalidKindTag`.
///
/// Critically this means: a prohibited capability cannot be smuggled
/// through a future-reserved tag value. The decoder strictly enforces
/// the RFC-0006 catalogue.
#[cfg(kani)]
#[kani::proof]
fn intent_i3_invalid_kind_tag_rejected() {
    let mut bytes = [0u8; OBSERVATION_SIZE];

    let bad_tag: u8 = kani::any();
    kani::assume(bad_tag > 3);

    bytes[8] = bad_tag;
    // Leave other fields zero — they are valid as defaults.

    let result = IntentObservation::decode(&bytes);
    match result {
        Err(DecodeError::InvalidKindTag { tag }) => assert!(tag == bad_tag),
        _ => assert!(false, "I3: bad kind_tag must be rejected"),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// I4: timestamp beyond SESSION_MAX_REASONABLE is rejected
// ───────────────────────────────────────────────────────────────────────────

/// **I4.** Any decode with `timestamp_us > 2^48` returns `TimestampOutOfRange`.
#[cfg(kani)]
#[kani::proof]
fn intent_i4_timestamp_out_of_range_rejected() {
    let mut bytes = [0u8; OBSERVATION_SIZE];

    let bad_ts: u64 = kani::any();
    kani::assume(bad_ts > Instant::SESSION_MAX_REASONABLE.as_micros());

    bytes[0..8].copy_from_slice(&bad_ts.to_le_bytes());
    // kind_tag and direction are zero — valid Navigation/Idle.

    let result = IntentObservation::decode(&bytes);
    match result {
        Err(DecodeError::TimestampOutOfRange { timestamp }) => assert!(timestamp == bad_ts),
        _ => assert!(false, "I4: out-of-range timestamp must be rejected"),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// I5: every non-zero reserved byte is rejected
// ───────────────────────────────────────────────────────────────────────────

/// **I5.** Any decode with a non-zero byte in offsets 24..32 returns
/// `NonZeroReservedByte`.
///
/// This prevents a producer from smuggling auxiliary data through the
/// reserved field — a structural protection against future protocol
/// drift.
#[cfg(kani)]
#[kani::proof]
fn intent_i5_non_zero_reserved_byte_rejected() {
    let mut bytes = [0u8; OBSERVATION_SIZE];

    let offset_in_reserved: usize = kani::any();
    kani::assume(offset_in_reserved < 8);

    let bad_value: u8 = kani::any();
    kani::assume(bad_value != 0);

    bytes[24 + offset_in_reserved] = bad_value;

    let result = IntentObservation::decode(&bytes);
    match result {
        Err(DecodeError::NonZeroReservedByte { offset, value }) => {
            assert!(offset == 24 + offset_in_reserved);
            assert!(value == bad_value);
        }
        _ => assert!(false, "I5: non-zero reserved byte must be rejected"),
    }
}

fn main() {
    // This binary hosts Kani harnesses. Under non-Kani builds it is inert;
    // under cargo kani, the harness functions above are run.
    #[cfg(not(kani))]
    eprintln!("axonos-intent: Kani harness collection. Run with: cargo kani");
}
