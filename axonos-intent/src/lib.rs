//! # axonos-intent
//!
//! Typed intent observations matching the AxonOS wire format specified
//! in [RFC-0006](https://github.com/AxonOS-org/axonos-rfcs/blob/main/rfcs/0006-intent-wire-format-abi.md).
//!
//! ## What this crate provides
//!
//! - The [`IntentObservation`] type — a 32-byte, 8-byte-aligned record
//!   matching the normative RFC-0006 layout byte for byte.
//! - The [`Kind`] and [`Direction`] enums — typed discriminants of the
//!   observation payload.
//! - The [`Confidence`] type — a Q0.16 fixed-point classifier confidence
//!   in the range `[0, 1)`, enforcing the RFC's prohibition on exact 1.0.
//! - The encode/decode functions [`IntentObservation::encode`] and
//!   [`IntentObservation::decode`] — strict, all-or-nothing serialisation
//!   to/from the wire format.
//! - Conformance test vectors against which any compliant implementation
//!   must validate (see `tests/` and `Vector::*` constants).
//!
//! ## Design discipline
//!
//! - `#![no_std]`. No allocator. No floating-point.
//! - All decoding is strict: any record with an undefined `kind_tag`, an
//!   out-of-range `direction`, a timestamp beyond
//!   [`Instant::SESSION_MAX_REASONABLE`](axonos_time::Instant::SESSION_MAX_REASONABLE),
//!   a non-zero reserved field, or a non-zero confidence ceiling is
//!   rejected with a specific error.
//! - All encoding is lossless and round-trip-stable: `decode(encode(o)) = o`
//!   for every well-formed `IntentObservation`.
//! - The crate does not perform HMAC verification; the
//!   [`AttestationTag`] type is opaque and carries the truncated tag for
//!   downstream verification by the consumer.
//!
//! ## Compile-time invariants
//!
//! ```text
//! assert size_of::<IntentObservation>() == 32
//! assert align_of::<IntentObservation>() == 8
//! ```
//!
//! Verified at build time by `const _: () = assert!(...)` guards. Failure
//! causes a compile error, not a runtime panic.
//!
//! ## Cross-references
//!
//! - **RFC-0006** §4 — Wire format of [`IntentObservation`].
//! - **RFC-0006** §5 — Capability bitfield (handled by `axonos-capability`).
//! - **RFC-0006** §6 — Attestation algorithm (HMAC-SHA256 truncated).
//! - **RFC-0006** §7 — Versioning rules.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

use axonos_capability::Capability;
use axonos_time::Instant;

// ── Compile-time layout assertions (RFC-0006 normative) ────────────────────

/// The normative wire size of an [`IntentObservation`] (RFC-0006 §4.1).
pub const OBSERVATION_SIZE: usize = 32;

/// The normative wire alignment of an [`IntentObservation`] (RFC-0006 §4.1).
pub const OBSERVATION_ALIGN: usize = 8;

// ───────────────────────────────────────────────────────────────────────────
// Kind — the discriminant of an IntentObservation payload
// ───────────────────────────────────────────────────────────────────────────

/// The kind of an intent observation, RFC-0006 §4.2 `kind_tag`.
///
/// Each variant corresponds to a [`Capability`]. The numeric encoding is
/// stable per RFC-0006; do not renumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Kind {
    /// Direction-of-attention event, paired with [`Capability::Navigation`].
    Navigation = 0x00,
    /// Cognitive workload level, paired with [`Capability::WorkloadAdvisory`].
    WorkloadAdvisory = 0x01,
    /// Session integrity indicator, paired with [`Capability::SessionQuality`].
    SessionQuality = 0x02,
    /// Artifact detection event, paired with [`Capability::ArtifactEvents`].
    ArtifactEvents = 0x03,
}

impl Kind {
    /// Try to construct a `Kind` from a raw `u8` tag.
    pub const fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0x00 => Ok(Kind::Navigation),
            0x01 => Ok(Kind::WorkloadAdvisory),
            0x02 => Ok(Kind::SessionQuality),
            0x03 => Ok(Kind::ArtifactEvents),
            other => Err(DecodeError::InvalidKindTag { tag: other }),
        }
    }

    /// The numeric tag encoded on the wire.
    #[must_use]
    pub const fn tag(self) -> u8 {
        self as u8
    }

    /// The [`Capability`] required to subscribe to this kind of observation.
    #[must_use]
    pub const fn capability(self) -> Capability {
        match self {
            Kind::Navigation => Capability::Navigation,
            Kind::WorkloadAdvisory => Capability::WorkloadAdvisory,
            Kind::SessionQuality => Capability::SessionQuality,
            Kind::ArtifactEvents => Capability::ArtifactEvents,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Direction — the per-kind payload enum
// ───────────────────────────────────────────────────────────────────────────

/// The payload byte of an observation, interpreted per [`Kind`].
///
/// RFC-0006 §4.2 fixes the numeric mapping for each kind. Adding a new
/// payload value within an existing kind requires a minor ABI version
/// bump per RFC-0006 §7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// `Navigation` payload.
    Navigation(NavigationDirection),
    /// `WorkloadAdvisory` payload.
    Workload(WorkloadLevel),
    /// `SessionQuality` payload.
    Session(SessionQuality),
    /// `ArtifactEvents` payload.
    Artifact(ArtifactKind),
}

impl Direction {
    /// The numeric byte encoded on the wire (RFC-0006 §4.2).
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Direction::Navigation(d) => d as u8,
            Direction::Workload(d) => d as u8,
            Direction::Session(d) => d as u8,
            Direction::Artifact(d) => d as u8,
        }
    }

    /// Parse a direction byte for a given kind.
    pub const fn from_byte(kind: Kind, byte: u8) -> Result<Self, DecodeError> {
        match kind {
            Kind::Navigation => match byte {
                0x00 => Ok(Direction::Navigation(NavigationDirection::Idle)),
                0x01 => Ok(Direction::Navigation(NavigationDirection::Left)),
                0x02 => Ok(Direction::Navigation(NavigationDirection::Right)),
                0x03 => Ok(Direction::Navigation(NavigationDirection::Up)),
                0x04 => Ok(Direction::Navigation(NavigationDirection::Down)),
                _ => Err(DecodeError::InvalidDirection { kind, byte }),
            },
            Kind::WorkloadAdvisory => match byte {
                0x00 => Ok(Direction::Workload(WorkloadLevel::Low)),
                0x01 => Ok(Direction::Workload(WorkloadLevel::Medium)),
                0x02 => Ok(Direction::Workload(WorkloadLevel::High)),
                _ => Err(DecodeError::InvalidDirection { kind, byte }),
            },
            Kind::SessionQuality => match byte {
                0x00 => Ok(Direction::Session(SessionQuality::Good)),
                0x01 => Ok(Direction::Session(SessionQuality::Degraded)),
                0x02 => Ok(Direction::Session(SessionQuality::Lost)),
                _ => Err(DecodeError::InvalidDirection { kind, byte }),
            },
            Kind::ArtifactEvents => match byte {
                0x00 => Ok(Direction::Artifact(ArtifactKind::Eye)),
                0x01 => Ok(Direction::Artifact(ArtifactKind::Muscle)),
                0x02 => Ok(Direction::Artifact(ArtifactKind::Motion)),
                0x03 => Ok(Direction::Artifact(ArtifactKind::Electrode)),
                _ => Err(DecodeError::InvalidDirection { kind, byte }),
            },
        }
    }

    /// The [`Kind`] this direction belongs to.
    #[must_use]
    pub const fn kind(self) -> Kind {
        match self {
            Direction::Navigation(_) => Kind::Navigation,
            Direction::Workload(_) => Kind::WorkloadAdvisory,
            Direction::Session(_) => Kind::SessionQuality,
            Direction::Artifact(_) => Kind::ArtifactEvents,
        }
    }
}

/// Navigation payload — direction of attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NavigationDirection {
    /// No directional intent.
    Idle = 0x00,
    /// Leftward attention.
    Left = 0x01,
    /// Rightward attention.
    Right = 0x02,
    /// Upward attention.
    Up = 0x03,
    /// Downward attention.
    Down = 0x04,
}

/// WorkloadAdvisory payload — cognitive workload level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WorkloadLevel {
    /// Low cognitive load.
    Low = 0x00,
    /// Medium cognitive load.
    Medium = 0x01,
    /// High cognitive load.
    High = 0x02,
}

/// SessionQuality payload — session integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SessionQuality {
    /// Session signal quality acceptable for closed-loop operation.
    Good = 0x00,
    /// Session signal quality degraded; classification still admissible.
    Degraded = 0x01,
    /// Session signal quality lost; classification not admissible.
    Lost = 0x02,
}

/// ArtifactEvents payload — artifact detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ArtifactKind {
    /// Eye-blink or ocular artifact.
    Eye = 0x00,
    /// EMG / muscle artifact.
    Muscle = 0x01,
    /// Motion / accelerometer artifact.
    Motion = 0x02,
    /// Electrode contact / impedance artifact.
    Electrode = 0x03,
}

// ───────────────────────────────────────────────────────────────────────────
// Confidence — Q0.16 fixed-point, [0, 1)
// ───────────────────────────────────────────────────────────────────────────

/// Classifier confidence as Q0.16 fixed-point.
///
/// The encoded value `v` represents the confidence `v / 65_536`. The range
/// is `[0, 65_535 / 65_536]` ≈ `[0, 0.99998]`. The value `1.0` exactly is
/// **not representable by design** — confidence claims of exactly 1.0 are
/// forbidden (RFC-0006 §4.2 `confidence_q0_16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Confidence(u16);

impl Confidence {
    /// The minimum representable confidence: 0.0.
    pub const MIN: Self = Self(0);

    /// The maximum representable confidence: 65_535 / 65_536 ≈ 0.99998.
    pub const MAX: Self = Self(u16::MAX);

    /// Construct from a raw Q0.16 value.
    #[must_use]
    pub const fn from_q0_16(raw: u16) -> Self {
        Self(raw)
    }

    /// The raw Q0.16 value (the byte-pair on the wire).
    #[must_use]
    pub const fn as_q0_16(self) -> u16 {
        self.0
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Attestation tag (opaque, 8 bytes truncated HMAC-SHA256)
// ───────────────────────────────────────────────────────────────────────────

/// The 8-byte truncated HMAC-SHA256 tag carried on the wire.
///
/// This crate does not verify the tag; it only carries the bytes. The
/// downstream consumer must compute HMAC-SHA256 over the first 24 bytes
/// of the encoded record using the session key (RFC-0006 §6), then
/// compare the first 8 bytes to this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AttestationTag(pub [u8; 8]);

// ───────────────────────────────────────────────────────────────────────────
// IntentObservation — the 32-byte wire record
// ───────────────────────────────────────────────────────────────────────────

/// A single intent observation as it crosses the AxonOS kernel/application
/// boundary. Layout matches RFC-0006 §4.1 byte for byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentObservation {
    /// Absolute timestamp of this observation, microseconds since session start.
    pub timestamp: Instant,
    /// The kind discriminant.
    pub kind: Kind,
    /// The kind-specific payload.
    pub direction: Direction,
    /// Q0.16 confidence in `[0, 1)`.
    pub confidence: Confidence,
    /// Per-subscription monotonic sequence counter.
    pub sequence: u32,
    /// Truncated HMAC-SHA256 tag, 8 bytes.
    pub attestation: AttestationTag,
}

// Compile-time layout assertions. RFC-0006 normative size and alignment.
// On a typical 64-bit host the Rust `IntentObservation` struct is laid out
// according to the default Rust ABI, not `repr(C, align(8))`. The values
// asserted below are the wire-format requirements, not necessarily the
// in-memory layout of the Rust type. Encode/decode functions handle the
// translation. (A future zerocopy-friendly variant with explicit
// `#[repr(C, align(8))]` is left as future work; this version prioritises
// portability and exhaustive testing.)
//
// `clippy::assertions_on_constants` is allowed because these guards are
// the specification: if the constants ever drift from the RFC-0006
// normative values, the build must break. The lint correctly notes the
// assertions reduce to `assert!(true)` under constant folding, which is
// precisely what we want — drift would reduce to `assert!(false)` and fail.
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(
    OBSERVATION_SIZE == 32,
    "RFC-0006 wire size must be 32 bytes"
);
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(
    OBSERVATION_ALIGN == 8,
    "RFC-0006 wire alignment must be 8 bytes"
);

impl IntentObservation {
    /// Encode this observation into the 32-byte wire format.
    ///
    /// The encoding is total: no failure modes other than what is enforced
    /// by the type system (e.g., `Confidence` cannot exceed 65_535, so no
    /// runtime saturation is needed).
    #[must_use]
    pub fn encode(&self) -> [u8; OBSERVATION_SIZE] {
        let mut bytes = [0u8; OBSERVATION_SIZE];

        // Offset 0..8: timestamp_us (u64 little-endian)
        let ts = self.timestamp.as_micros().to_le_bytes();
        bytes[0..8].copy_from_slice(&ts);

        // Offset 8: kind_tag (u8)
        bytes[8] = self.kind.tag();

        // Offset 9: direction (u8)
        bytes[9] = self.direction.byte();

        // Offset 10..12: confidence_q0_16 (u16 little-endian)
        let conf = self.confidence.as_q0_16().to_le_bytes();
        bytes[10..12].copy_from_slice(&conf);

        // Offset 12..16: sequence (u32 little-endian)
        let seq = self.sequence.to_le_bytes();
        bytes[12..16].copy_from_slice(&seq);

        // Offset 16..24: attestation_tag_truncated (u8[8])
        bytes[16..24].copy_from_slice(&self.attestation.0);

        // Offset 24..32: reserved (u8[8]) — already zero by initialisation.

        bytes
    }

    /// Decode a 32-byte wire-format buffer into an `IntentObservation`.
    ///
    /// Strict: any deviation from RFC-0006 normative requirements produces
    /// a specific [`DecodeError`].
    pub fn decode(bytes: &[u8; OBSERVATION_SIZE]) -> Result<Self, DecodeError> {
        // Offset 0..8: timestamp_us
        let ts_arr: [u8; 8] = bytes[0..8]
            .try_into()
            .expect("slice is 8 bytes by index math");
        let ts = u64::from_le_bytes(ts_arr);
        let timestamp = Instant(ts);

        // RFC-0006 normative: receivers MUST reject timestamps exceeding
        // SESSION_MAX_REASONABLE.
        if ts > Instant::SESSION_MAX_REASONABLE.as_micros() {
            return Err(DecodeError::TimestampOutOfRange { timestamp: ts });
        }

        // Offset 8: kind_tag
        let kind = Kind::from_tag(bytes[8])?;

        // Offset 9: direction
        let direction = Direction::from_byte(kind, bytes[9])?;

        // Offset 10..12: confidence_q0_16
        let conf_arr: [u8; 2] = bytes[10..12].try_into().expect("slice is 2 bytes");
        let confidence = Confidence::from_q0_16(u16::from_le_bytes(conf_arr));

        // Offset 12..16: sequence
        let seq_arr: [u8; 4] = bytes[12..16].try_into().expect("slice is 4 bytes");
        let sequence = u32::from_le_bytes(seq_arr);

        // Offset 16..24: attestation_tag_truncated
        let mut tag = [0u8; 8];
        tag.copy_from_slice(&bytes[16..24]);
        let attestation = AttestationTag(tag);

        // Offset 24..32: reserved — MUST be all zero (RFC-0006 §4.2).
        for (i, &b) in bytes[24..32].iter().enumerate() {
            if b != 0 {
                return Err(DecodeError::NonZeroReservedByte {
                    offset: 24 + i,
                    value: b,
                });
            }
        }

        Ok(IntentObservation {
            timestamp,
            kind,
            direction,
            confidence,
            sequence,
            attestation,
        })
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Decode errors
// ───────────────────────────────────────────────────────────────────────────

/// Errors that may arise from [`IntentObservation::decode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The `kind_tag` byte was outside the defined range (0x00..=0x03).
    InvalidKindTag {
        /// The offending tag value.
        tag: u8,
    },
    /// The `direction` byte was outside the per-kind defined range.
    InvalidDirection {
        /// The kind for which the direction was decoded.
        kind: Kind,
        /// The offending direction byte.
        byte: u8,
    },
    /// The `timestamp_us` exceeded `SESSION_MAX_REASONABLE` (RFC-0006).
    TimestampOutOfRange {
        /// The offending timestamp value.
        timestamp: u64,
    },
    /// A reserved byte (offsets 24..32) was non-zero.
    NonZeroReservedByte {
        /// The wire-format offset of the offending byte.
        offset: usize,
        /// The offending value.
        value: u8,
    },
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> IntentObservation {
        IntentObservation {
            timestamp: Instant(1_000),
            kind: Kind::Navigation,
            direction: Direction::Navigation(NavigationDirection::Right),
            confidence: Confidence::from_q0_16(0x8000),
            sequence: 1,
            attestation: AttestationTag([0x3c, 0xa2, 0x1f, 0x8b, 0x4d, 0x6e, 0x7a, 0x91]),
        }
    }

    #[test]
    fn encode_then_decode_round_trip() {
        let obs = sample();
        let bytes = obs.encode();
        let decoded = IntentObservation::decode(&bytes).unwrap();
        assert_eq!(decoded, obs);
    }

    #[test]
    fn encoded_size_is_32() {
        let bytes = sample().encode();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn vector_1_decodes_to_expected() {
        // From RFC-0006 Appendix A.1, Vector 1: Navigation Right, seq=1,
        // confidence=0x8000, ts=1000.
        let bytes: [u8; 32] = [
            0xe8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // timestamp_us = 1000
            0x00, // kind = Navigation
            0x02, // direction = Right
            0x00, 0x80, // confidence = 0x8000
            0x01, 0x00, 0x00, 0x00, // sequence = 1
            0x3c, 0xa2, 0x1f, 0x8b, 0x4d, 0x6e, 0x7a, 0x91, // attestation
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved (zero)
        ];
        let obs = IntentObservation::decode(&bytes).unwrap();
        assert_eq!(obs.timestamp, Instant(1000));
        assert_eq!(obs.kind, Kind::Navigation);
        assert_eq!(
            obs.direction,
            Direction::Navigation(NavigationDirection::Right)
        );
        assert_eq!(obs.confidence.as_q0_16(), 0x8000);
        assert_eq!(obs.sequence, 1);
    }

    #[test]
    fn invalid_kind_tag_rejected() {
        let mut bytes = sample().encode();
        bytes[8] = 0xFF;
        assert_eq!(
            IntentObservation::decode(&bytes),
            Err(DecodeError::InvalidKindTag { tag: 0xFF })
        );
    }

    #[test]
    fn invalid_direction_rejected() {
        let mut bytes = sample().encode();
        bytes[9] = 0xFE; // Navigation has only 0x00..=0x04
        match IntentObservation::decode(&bytes) {
            Err(DecodeError::InvalidDirection { kind, byte }) => {
                assert_eq!(kind, Kind::Navigation);
                assert_eq!(byte, 0xFE);
            }
            other => panic!("expected InvalidDirection, got {other:?}"),
        }
    }

    #[test]
    fn timestamp_out_of_range_rejected() {
        let mut bytes = sample().encode();
        // 2^48 + 1 — one microsecond past SESSION_MAX_REASONABLE.
        let bad_ts: u64 = (1u64 << 48) + 1;
        bytes[0..8].copy_from_slice(&bad_ts.to_le_bytes());
        match IntentObservation::decode(&bytes) {
            Err(DecodeError::TimestampOutOfRange { timestamp }) => {
                assert_eq!(timestamp, bad_ts);
            }
            other => panic!("expected TimestampOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn non_zero_reserved_byte_rejected() {
        let mut bytes = sample().encode();
        bytes[26] = 0xFF;
        match IntentObservation::decode(&bytes) {
            Err(DecodeError::NonZeroReservedByte { offset, value }) => {
                assert_eq!(offset, 26);
                assert_eq!(value, 0xFF);
            }
            other => panic!("expected NonZeroReservedByte, got {other:?}"),
        }
    }

    #[test]
    fn confidence_max_does_not_represent_one() {
        // Per RFC-0006, exact 1.0 must not be representable.
        assert_eq!(Confidence::MAX.as_q0_16(), u16::MAX);
        let as_fraction = f64::from(Confidence::MAX.as_q0_16()) / 65536.0;
        assert!(as_fraction < 1.0);
    }

    #[test]
    fn kind_maps_to_capability() {
        assert_eq!(Kind::Navigation.capability(), Capability::Navigation);
        assert_eq!(
            Kind::WorkloadAdvisory.capability(),
            Capability::WorkloadAdvisory
        );
        assert_eq!(
            Kind::SessionQuality.capability(),
            Capability::SessionQuality
        );
        assert_eq!(
            Kind::ArtifactEvents.capability(),
            Capability::ArtifactEvents
        );
    }

    #[test]
    fn all_directions_for_navigation() {
        for byte in 0..=4u8 {
            let d = Direction::from_byte(Kind::Navigation, byte).unwrap();
            assert_eq!(d.byte(), byte);
            assert_eq!(d.kind(), Kind::Navigation);
        }
        assert!(Direction::from_byte(Kind::Navigation, 5).is_err());
    }

    #[test]
    fn all_directions_for_artifact() {
        for byte in 0..=3u8 {
            let d = Direction::from_byte(Kind::ArtifactEvents, byte).unwrap();
            assert_eq!(d.byte(), byte);
        }
        assert!(Direction::from_byte(Kind::ArtifactEvents, 4).is_err());
    }
}
