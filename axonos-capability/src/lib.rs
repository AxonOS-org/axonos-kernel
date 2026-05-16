// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # axonos-capability
//!
//! Capability-based application isolation primitives for AxonOS.
//!
//! ## What this crate is
//!
//! This crate is the **policy enforcement path** of the AxonOS kernel. It
//! provides the type-level vocabulary by which the kernel declares what an
//! application is *permitted to observe* about neural data, and the
//! install-time verification function that determines whether an
//! application manifest is admissible.
//!
//! ## Structural data minimisation
//!
//! The central design principle: **prohibited capabilities do not exist as
//! enum variants**. Raw EEG, continuous emotion inference, cognitive
//! profile reads, and re-identification are not denied by runtime check —
//! they are absent from the type system. An application cannot request a
//! capability that does not exist, and the kernel cannot deliver an event
//! whose type is not declared.
//!
//! This is in contrast to the conventional approach of "deny by policy,"
//! where prohibited operations exist at the type level but are denied at
//! runtime by an access-control list. Policy-based denial is breakable
//! by a sufficiently determined application; structural absence is not.
//!
//! ## The four permitted capabilities
//!
//! - [`Capability::Navigation`] — direction-of-attention event (5-class
//!   discrete classification: Idle/Left/Right/Up/Down).
//! - [`Capability::WorkloadAdvisory`] — cognitive workload level (3-class:
//!   Low/Medium/High).
//! - [`Capability::SessionQuality`] — session integrity indicator
//!   (3-class: Good/Degraded/Lost).
//! - [`Capability::ArtifactEvents`] — artifact detection (4-class:
//!   Eye/Muscle/Motion/Electrode).
//!
//! Each emits **discrete enum payloads** at a bounded rate. The maximum
//! information leak through this channel is bounded by the
//! [Information-Theoretic Privacy Bound](#privacy-bound) below.
//!
//! ## Privacy bound
//!
//! For the catalogue of all four capabilities at their maximum rates, the
//! mutual information between raw EEG `X` and the application-observable
//! event stream `Y` is bounded by:
//!
//! ```text
//! I(X; Y) ≤ H(Y) ≤ Σ_κ r_κ · log₂(|payload_κ|) bits/s
//!         ≤ 50·log₂(5) + 1·log₂(3) + 2·log₂(3) + 10·log₂(4)
//!         ≤ 140.85 bits/s
//! ```
//!
//! This is an analytic upper bound, derived from the cardinality of each
//! capability's payload space and the kernel-enforced rate limit. It is
//! not measurement; it is a structural property of this crate's
//! definition.
//!
//! ## Threat model
//!
//! Three adversaries are considered:
//!
//! - **A1** (malicious installed application): may have any internal
//!   logic, may attempt to extract neural data.
//! - **A2** (compromised application with memory-safety bug): a
//!   memory-safe app whose runtime state has been corrupted.
//! - **A3** (network adversary): observes the application's external
//!   outputs (BLE radio, GPIO toggles, power consumption).
//!
//! Against all three, the kernel guarantees that no event of a type
//! outside the application's manifest is ever delivered, and no raw EEG
//! tensor is ever serialised across the kernel-application boundary.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

// ───────────────────────────────────────────────────────────────────────────
// Capability enum
// ───────────────────────────────────────────────────────────────────────────

/// The set of capabilities an application may request.
///
/// **Note on structural data minimisation.** The variants of this enum are
/// the *complete* set of capabilities an application may ever hold. There
/// is no `RawEEG`, no `ContinuousEmotion`, no `CognitiveProfile`, no
/// `Reidentification`. These are not denied by runtime check — they do
/// not exist at the type level. An application that wishes to access raw
/// neural data has no way to express the request; the kernel has no way to
/// deliver the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Capability {
    /// Direction-of-attention event, 5-class discrete output.
    ///
    /// Payload alphabet: {Idle, Left, Right, Up, Down} — cardinality 5.
    Navigation = 0,

    /// Cognitive workload level, 3-class discrete output.
    ///
    /// Payload alphabet: {Low, Medium, High} — cardinality 3.
    WorkloadAdvisory = 1,

    /// Session integrity indicator, 3-class discrete output.
    ///
    /// Payload alphabet: {Good, Degraded, Lost} — cardinality 3.
    SessionQuality = 2,

    /// Artifact detection, 4-class discrete output.
    ///
    /// Payload alphabet: {Eye, Muscle, Motion, Electrode} — cardinality 4.
    ArtifactEvents = 3,
}

impl Capability {
    /// The full ordered list of capabilities.
    ///
    /// This is the kernel's [`Catalogue`] of admissible capabilities.
    /// The length and ordering of this list is part of the stable ABI
    /// declared in RFC-0006.
    pub const ALL: &'static [Capability] = &[
        Capability::Navigation,
        Capability::WorkloadAdvisory,
        Capability::SessionQuality,
        Capability::ArtifactEvents,
    ];

    /// The bit position of this capability in a [`CapabilitySet`] bitfield.
    #[must_use]
    pub const fn bit(self) -> u8 {
        self as u8
    }

    /// The bitmask of this capability in a [`CapabilitySet`] bitfield.
    #[must_use]
    pub const fn mask(self) -> u32 {
        1u32 << self.bit()
    }

    /// The maximum event rate (in events per second) the kernel will emit
    /// for this capability. Rates are bounded by RFC-0006.
    #[must_use]
    pub const fn max_rate_hz(self) -> u32 {
        match self {
            Capability::Navigation => 50,
            Capability::WorkloadAdvisory => 1,
            Capability::SessionQuality => 2,
            Capability::ArtifactEvents => 10,
        }
    }

    /// The cardinality of this capability's discrete payload space.
    ///
    /// Used in the privacy-bound calculation (see crate-level docs).
    #[must_use]
    pub const fn payload_cardinality(self) -> u32 {
        match self {
            Capability::Navigation => 5,
            Capability::WorkloadAdvisory => 3,
            Capability::SessionQuality => 3,
            Capability::ArtifactEvents => 4,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// CapabilitySet
// ───────────────────────────────────────────────────────────────────────────

/// A set of capabilities, represented as a 32-bit bitfield.
///
/// The wire format of this bitfield is normative and is specified in
/// RFC-0006 §5 (Capability Bitfield). Bits 0–3 correspond to the four
/// admissible capabilities; bits 4–31 are reserved and must be zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    bits: u32,
}

impl CapabilitySet {
    /// The empty set.
    pub const EMPTY: Self = Self { bits: 0 };

    /// The full set (all four admissible capabilities).
    pub const ALL: Self = Self {
        bits: 0x0000_000F, // bits 0-3
    };

    /// Mask covering all admissible capability bits.
    pub const ADMISSIBLE_MASK: u32 = 0x0000_000F;

    /// Construct an empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self::EMPTY
    }

    /// Construct a set containing a single capability.
    #[must_use]
    pub const fn singleton(cap: Capability) -> Self {
        Self { bits: cap.mask() }
    }

    /// Construct a set from a raw 32-bit value.
    ///
    /// # Errors
    ///
    /// Returns `Err(BitfieldError::ReservedBitSet)` if any reserved bit
    /// (bits 4–31) is set. This is a hard error per RFC-0006: receivers
    /// must reject handshakes with reserved bits set.
    pub const fn from_bits(bits: u32) -> Result<Self, BitfieldError> {
        if bits & !Self::ADMISSIBLE_MASK != 0 {
            Err(BitfieldError::ReservedBitSet { bits })
        } else {
            Ok(Self { bits })
        }
    }

    /// The underlying bitfield representation (RFC-0006 wire format).
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.bits
    }

    /// Returns `true` if this set contains the given capability.
    #[must_use]
    pub const fn contains(self, cap: Capability) -> bool {
        self.bits & cap.mask() != 0
    }

    /// Returns the number of capabilities in this set.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.bits.count_ones()
    }

    /// Returns `true` if this set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Returns a new set with the given capability added.
    #[must_use]
    pub const fn with(self, cap: Capability) -> Self {
        Self {
            bits: self.bits | cap.mask(),
        }
    }

    /// Returns a new set with the given capability removed.
    #[must_use]
    pub const fn without(self, cap: Capability) -> Self {
        Self {
            bits: self.bits & !cap.mask(),
        }
    }

    /// Returns the union of this set with `other`.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Returns the intersection of this set with `other`.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    /// Returns the set difference `self \ other`.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self {
            bits: self.bits & !other.bits,
        }
    }

    /// Returns `true` if `self` is a subset of `other`.
    ///
    /// This is the central operation of install-time manifest verification:
    /// the application's manifest must be a subset of the kernel catalogue.
    #[must_use]
    pub const fn is_subset_of(self, other: Self) -> bool {
        self.bits & !other.bits == 0
    }

    /// Iterate over the capabilities in this set, in canonical order
    /// (ascending bit position).
    #[must_use]
    pub const fn iter(self) -> CapabilityIter {
        CapabilityIter {
            remaining: self.bits & Self::ADMISSIBLE_MASK,
        }
    }

    /// Compute the analytic mutual-information upper bound for this
    /// capability set, in bits per second.
    ///
    /// The returned value is `Σ_κ r_κ · log₂(|payload_κ|) · SCALE` where
    /// `SCALE = 1_000_000` (fixed-point representation; no floating-point
    /// dependency on the hot path). Divide by `SCALE` to recover the
    /// rational value.
    ///
    /// For the full set [`Capability::ALL`], this yields approximately
    /// `140_853_754` (i.e., 140.85 bits/s).
    #[must_use]
    pub fn information_bound_scaled(self) -> u64 {
        // log2 table scaled by 1_000_000.
        // log2(3) ≈ 1.58496250072  → 1_584_962
        // log2(4) = 2              → 2_000_000
        // log2(5) ≈ 2.32192809489  → 2_321_928
        const LOG2_3: u64 = 1_584_962;
        const LOG2_4: u64 = 2_000_000;
        const LOG2_5: u64 = 2_321_928;

        let mut total: u64 = 0;
        for cap in self.iter() {
            let rate = u64::from(cap.max_rate_hz());
            let log2_card = match cap.payload_cardinality() {
                3 => LOG2_3,
                4 => LOG2_4,
                5 => LOG2_5,
                // Unreachable for current Capability enum. Defensive: return
                // a conservative upper bound rather than panic.
                _ => LOG2_5,
            };
            total = total.saturating_add(rate.saturating_mul(log2_card));
        }
        total
    }
}

/// Iterator over the capabilities in a [`CapabilitySet`].
#[derive(Debug, Clone)]
pub struct CapabilityIter {
    remaining: u32,
}

impl Iterator for CapabilityIter {
    type Item = Capability;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let bit = self.remaining.trailing_zeros();
        self.remaining &= !(1u32 << bit);
        // bit is guaranteed to be in 0..=3 because the iterator was
        // constructed from a value masked with ADMISSIBLE_MASK.
        match bit {
            0 => Some(Capability::Navigation),
            1 => Some(Capability::WorkloadAdvisory),
            2 => Some(Capability::SessionQuality),
            3 => Some(Capability::ArtifactEvents),
            _ => None, // unreachable in practice
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.remaining.count_ones() as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for CapabilityIter {}

impl IntoIterator for CapabilitySet {
    type Item = Capability;
    type IntoIter = CapabilityIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Errors that may arise from constructing a [`CapabilitySet`] from raw bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitfieldError {
    /// A reserved bit (4–31) was set. Per RFC-0006, receivers must reject
    /// handshakes with reserved bits set.
    ReservedBitSet {
        /// The offending bitfield value.
        bits: u32,
    },
}

// ───────────────────────────────────────────────────────────────────────────
// Manifest and verification
// ───────────────────────────────────────────────────────────────────────────

/// An application's declared capability request.
///
/// A `Manifest` is signed by the application developer and presented to
/// the kernel at install time. The kernel verifies the signature (out of
/// scope for this crate) and then verifies that the manifest is
/// well-formed and admissible against the [`Catalogue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Manifest {
    /// The set of capabilities the application requests.
    pub requested: CapabilitySet,
}

impl Manifest {
    /// Construct a manifest requesting the given capability set.
    #[must_use]
    pub const fn new(requested: CapabilitySet) -> Self {
        Self { requested }
    }
}

/// The kernel's catalogue of admissible capabilities.
///
/// For the current AxonOS reference implementation, the catalogue is
/// always [`CapabilitySet::ALL`]. A specialised deployment (for example,
/// a clinical setting that does not need `WorkloadAdvisory`) may restrict
/// the catalogue at boot time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Catalogue {
    /// The set of capabilities the kernel will admit.
    pub admissible: CapabilitySet,
}

impl Catalogue {
    /// The default catalogue: all four capabilities are admissible.
    pub const DEFAULT: Self = Self {
        admissible: CapabilitySet::ALL,
    };

    /// Construct a catalogue with the given admissible set.
    #[must_use]
    pub const fn new(admissible: CapabilitySet) -> Self {
        Self { admissible }
    }
}

impl Default for Catalogue {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Verify a manifest against the kernel catalogue.
///
/// This is the central function of install-time admission. A manifest is
/// admissible if and only if its requested set is a subset of the
/// catalogue's admissible set.
///
/// # Returns
///
/// `Ok(())` if the manifest is admissible.
///
/// `Err(VerificationFailure::ExcessCapabilities)` if the manifest
/// requests capabilities outside the catalogue.
///
/// # Theorem (capability containment)
///
/// If `verify_manifest(manifest, catalogue) == Ok(())`, then for every
/// capability `c` in `manifest.requested`, `catalogue.admissible.contains(c)`
/// is true. This is verified by the Kani harness `cap_c1_subset_implies_each`
/// in `kani-proofs/`.
pub fn verify_manifest(
    manifest: &Manifest,
    catalogue: &Catalogue,
) -> Result<(), VerificationFailure> {
    if manifest.requested.is_subset_of(catalogue.admissible) {
        Ok(())
    } else {
        let excess = manifest.requested.difference(catalogue.admissible);
        Err(VerificationFailure::ExcessCapabilities { excess })
    }
}

/// Reasons a manifest may fail verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationFailure {
    /// The manifest requests capabilities not in the catalogue.
    ExcessCapabilities {
        /// The capabilities in the manifest but not in the catalogue.
        excess: CapabilitySet,
    },
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_is_empty() {
        let s = CapabilitySet::EMPTY;
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.iter().count(), 0);
    }

    #[test]
    fn singleton_contains_only_one() {
        let s = CapabilitySet::singleton(Capability::Navigation);
        assert!(s.contains(Capability::Navigation));
        assert!(!s.contains(Capability::WorkloadAdvisory));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn all_contains_all_four() {
        let s = CapabilitySet::ALL;
        for cap in Capability::ALL {
            assert!(s.contains(*cap), "ALL should contain {cap:?}");
        }
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn from_bits_rejects_reserved() {
        // Bit 4 is reserved.
        assert!(matches!(
            CapabilitySet::from_bits(0x0000_0010),
            Err(BitfieldError::ReservedBitSet { .. })
        ));
        // Bit 31 is reserved.
        assert!(matches!(
            CapabilitySet::from_bits(0x8000_0000),
            Err(BitfieldError::ReservedBitSet { .. })
        ));
    }

    #[test]
    fn from_bits_accepts_admissible() {
        for bits in 0..=0x0Fu32 {
            assert!(CapabilitySet::from_bits(bits).is_ok());
        }
    }

    #[test]
    fn union_intersection_difference() {
        let a = CapabilitySet::singleton(Capability::Navigation).with(Capability::SessionQuality);
        let b =
            CapabilitySet::singleton(Capability::SessionQuality).with(Capability::ArtifactEvents);

        let u = a.union(b);
        assert_eq!(u.len(), 3);
        assert!(u.contains(Capability::Navigation));
        assert!(u.contains(Capability::SessionQuality));
        assert!(u.contains(Capability::ArtifactEvents));

        let i = a.intersection(b);
        assert_eq!(i, CapabilitySet::singleton(Capability::SessionQuality));

        let d = a.difference(b);
        assert_eq!(d, CapabilitySet::singleton(Capability::Navigation));
    }

    #[test]
    fn subset_relation() {
        let small = CapabilitySet::singleton(Capability::Navigation);
        let big = CapabilitySet::ALL;
        assert!(small.is_subset_of(big));
        assert!(!big.is_subset_of(small));
        assert!(CapabilitySet::EMPTY.is_subset_of(CapabilitySet::EMPTY));
        assert!(big.is_subset_of(big));
    }

    #[test]
    fn iter_yields_in_canonical_order() {
        let s = CapabilitySet::ALL;
        let collected: heapless::Vec<_, 4> = s.iter().collect();
        assert_eq!(
            collected.as_slice(),
            &[
                Capability::Navigation,
                Capability::WorkloadAdvisory,
                Capability::SessionQuality,
                Capability::ArtifactEvents,
            ]
        );
    }

    #[test]
    fn verify_manifest_accepts_subset() {
        let manifest = Manifest::new(CapabilitySet::singleton(Capability::Navigation));
        let catalogue = Catalogue::DEFAULT;
        assert_eq!(verify_manifest(&manifest, &catalogue), Ok(()));
    }

    #[test]
    fn verify_manifest_accepts_full_match() {
        let manifest = Manifest::new(CapabilitySet::ALL);
        let catalogue = Catalogue::DEFAULT;
        assert_eq!(verify_manifest(&manifest, &catalogue), Ok(()));
    }

    #[test]
    fn verify_manifest_rejects_excess() {
        // Restricted catalogue: only Navigation is admissible.
        let restricted = Catalogue::new(CapabilitySet::singleton(Capability::Navigation));
        let manifest = Manifest::new(CapabilitySet::ALL);
        match verify_manifest(&manifest, &restricted) {
            Err(VerificationFailure::ExcessCapabilities { excess }) => {
                // Excess is everything except Navigation.
                assert!(!excess.contains(Capability::Navigation));
                assert!(excess.contains(Capability::WorkloadAdvisory));
                assert!(excess.contains(Capability::SessionQuality));
                assert!(excess.contains(Capability::ArtifactEvents));
            }
            other => panic!("expected ExcessCapabilities, got {other:?}"),
        }
    }

    #[test]
    fn information_bound_matches_preprint() {
        // For the full capability set:
        //   Navigation:       50 Hz × log2(5) ≈ 50 × 2.32192 = 116.0964
        //   WorkloadAdvisory:  1 Hz × log2(3) ≈  1 × 1.58496 =   1.5850
        //   SessionQuality:    2 Hz × log2(3) ≈  2 × 1.58496 =   3.1699
        //   ArtifactEvents:   10 Hz × log2(4) =  10 × 2.0    =  20.0000
        //   Total                                            ≈ 140.85 bits/s
        let s = CapabilitySet::ALL;
        let scaled = s.information_bound_scaled();
        let bits_per_sec = scaled as f64 / 1_000_000.0;
        assert!(
            (bits_per_sec - 140.85).abs() < 0.01,
            "expected ≈140.85 bits/s, got {bits_per_sec:.4}"
        );
    }

    #[test]
    fn information_bound_monotone() {
        // Adding a capability never decreases the bound.
        let empty = CapabilitySet::EMPTY.information_bound_scaled();
        let nav = CapabilitySet::singleton(Capability::Navigation).information_bound_scaled();
        let nav_and_sq = CapabilitySet::singleton(Capability::Navigation)
            .with(Capability::SessionQuality)
            .information_bound_scaled();
        let all = CapabilitySet::ALL.information_bound_scaled();

        assert!(empty < nav);
        assert!(nav < nav_and_sq);
        assert!(nav_and_sq < all);
    }

    #[test]
    fn capability_bits_are_distinct() {
        let bits: heapless::Vec<u8, 4> = Capability::ALL.iter().map(|c| c.bit()).collect();
        // No duplicates.
        for i in 0..bits.len() {
            for j in (i + 1)..bits.len() {
                assert_ne!(bits[i], bits[j]);
            }
        }
    }
}
