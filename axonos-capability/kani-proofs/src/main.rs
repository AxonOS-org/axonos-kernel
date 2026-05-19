// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # Kani BMC harnesses for axonos-capability
//!
//! Verifies the structural data-minimisation invariant under bounded model
//! checking. The properties verified here are the core safety guarantees
//! of the AxonOS capability system; they are referenced from RFC-0005
//! (Capability-Based Application Manifest) and §VI of the preprint.

#![cfg_attr(kani, no_std)]

#[cfg(kani)]
use axonos_capability::{
    verify_manifest, Capability, CapabilitySet, Catalogue, Manifest, VerificationFailure,
};

// ───────────────────────────────────────────────────────────────────────────
// C1: subset implies contains for each element
// ───────────────────────────────────────────────────────────────────────────

/// **C1.** For any two capability sets `a, b`, if `a.is_subset_of(b)` then
/// for every capability `c` in `a`, `b.contains(c)`.
///
/// This is the central correctness statement of the subset relation: it
/// matches the mathematical definition. The Kani solver exhaustively
/// checks all `2⁴ × 2⁴ = 256` admissible bit configurations.
#[cfg(kani)]
#[kani::proof]
fn cap_c1_subset_implies_contains() {
    let a_bits: u32 = kani::any();
    let b_bits: u32 = kani::any();

    kani::assume(a_bits <= 0x0F);
    kani::assume(b_bits <= 0x0F);

    let a = CapabilitySet::from_bits(a_bits).unwrap();
    let b = CapabilitySet::from_bits(b_bits).unwrap();

    if a.is_subset_of(b) {
        for cap in Capability::ALL {
            if a.contains(*cap) {
                assert!(b.contains(*cap), "C1: if a ⊆ b and c ∈ a, then c ∈ b");
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// C2: from_bits rejects every reserved-bit pattern
// ───────────────────────────────────────────────────────────────────────────

/// **C2.** For any 32-bit value with at least one bit ≥ 4 set,
/// `from_bits` returns an error. (No reserved-bit pattern is admissible.)
#[cfg(kani)]
#[kani::proof]
fn cap_c2_reserved_bits_always_rejected() {
    let bits: u32 = kani::any();
    kani::assume(bits & !0x0Fu32 != 0); // at least one reserved bit set

    let result = CapabilitySet::from_bits(bits);
    assert!(result.is_err(), "C2: reserved bits must be rejected");
}

// ───────────────────────────────────────────────────────────────────────────
// C3: verify_manifest is sound — accepted manifest has all caps in catalogue
// ───────────────────────────────────────────────────────────────────────────

/// **C3.** If `verify_manifest(manifest, catalogue)` returns `Ok(())`, then
/// for every capability `c` in `manifest.requested`,
/// `catalogue.admissible.contains(c)`.
///
/// This is the install-time admission soundness theorem. If verification
/// passes, no capability outside the catalogue is in the manifest.
/// Equivalently: prohibited types are absent from any accepted manifest.
#[cfg(kani)]
#[kani::proof]
fn cap_c3_verify_sound() {
    let req_bits: u32 = kani::any();
    let cat_bits: u32 = kani::any();

    kani::assume(req_bits <= 0x0F);
    kani::assume(cat_bits <= 0x0F);

    let manifest = Manifest::new(CapabilitySet::from_bits(req_bits).unwrap());
    let catalogue = Catalogue::new(CapabilitySet::from_bits(cat_bits).unwrap());

    if verify_manifest(&manifest, &catalogue).is_ok() {
        for cap in Capability::ALL {
            if manifest.requested.contains(*cap) {
                assert!(
                    catalogue.admissible.contains(*cap),
                    "C3: accepted manifest has caps in catalogue"
                );
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// C4: verify_manifest is complete — rejected manifest has at least one excess
// ───────────────────────────────────────────────────────────────────────────

/// **C4.** If `verify_manifest(manifest, catalogue)` returns
/// `Err(ExcessCapabilities)`, then at least one capability in the
/// `excess` field is in `manifest.requested` but not in
/// `catalogue.admissible`.
///
/// Completeness: every rejection correctly identifies at least one
/// excess capability.
#[cfg(kani)]
#[kani::proof]
fn cap_c4_verify_complete() {
    let req_bits: u32 = kani::any();
    let cat_bits: u32 = kani::any();

    kani::assume(req_bits <= 0x0F);
    kani::assume(cat_bits <= 0x0F);

    let manifest = Manifest::new(CapabilitySet::from_bits(req_bits).unwrap());
    let catalogue = Catalogue::new(CapabilitySet::from_bits(cat_bits).unwrap());

    if let Err(VerificationFailure::ExcessCapabilities { excess }) =
        verify_manifest(&manifest, &catalogue)
    {
        // At least one excess capability exists.
        assert!(!excess.is_empty(), "C4: rejection must identify excess");

        // Every cap in `excess` is in manifest but not in catalogue.
        for cap in Capability::ALL {
            if excess.contains(*cap) {
                assert!(manifest.requested.contains(*cap), "C4: excess ⊆ requested");
                assert!(
                    !catalogue.admissible.contains(*cap),
                    "C4: excess ∩ admissible = ∅"
                );
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// C5: subset is reflexive
// ───────────────────────────────────────────────────────────────────────────

/// **C5.** For every admissible `CapabilitySet a`, `a.is_subset_of(a)`.
#[cfg(kani)]
#[kani::proof]
fn cap_c5_subset_reflexive() {
    let bits: u32 = kani::any();
    kani::assume(bits <= 0x0F);

    let a = CapabilitySet::from_bits(bits).unwrap();
    assert!(a.is_subset_of(a), "C5: subset reflexivity");
}

// ───────────────────────────────────────────────────────────────────────────
// C6: subset is transitive
// ───────────────────────────────────────────────────────────────────────────

/// **C6.** For every triple `a, b, c` of admissible sets, if
/// `a.is_subset_of(b)` and `b.is_subset_of(c)`, then `a.is_subset_of(c)`.
#[cfg(kani)]
#[kani::proof]
fn cap_c6_subset_transitive() {
    let a_bits: u32 = kani::any();
    let b_bits: u32 = kani::any();
    let c_bits: u32 = kani::any();

    kani::assume(a_bits <= 0x0F);
    kani::assume(b_bits <= 0x0F);
    kani::assume(c_bits <= 0x0F);

    let a = CapabilitySet::from_bits(a_bits).unwrap();
    let b = CapabilitySet::from_bits(b_bits).unwrap();
    let c = CapabilitySet::from_bits(c_bits).unwrap();

    if a.is_subset_of(b) && b.is_subset_of(c) {
        assert!(a.is_subset_of(c), "C6: subset transitivity");
    }
}

// ───────────────────────────────────────────────────────────────────────────
// C7: information bound is monotone
// ───────────────────────────────────────────────────────────────────────────

/// **C7.** Adding a capability to a set never decreases the information
/// bound (monotonicity of the privacy bound under set inclusion).
///
/// This is critical: it means that any manifest verified against a
/// catalogue inherits an information bound no greater than the
/// catalogue's bound.
#[cfg(kani)]
#[kani::proof]
fn cap_c7_information_bound_monotone() {
    let a_bits: u32 = kani::any();
    let b_bits: u32 = kani::any();

    kani::assume(a_bits <= 0x0F);
    kani::assume(b_bits <= 0x0F);

    let a = CapabilitySet::from_bits(a_bits).unwrap();
    let b = CapabilitySet::from_bits(b_bits).unwrap();

    if a.is_subset_of(b) {
        assert!(
            a.information_bound_scaled() <= b.information_bound_scaled(),
            "C7: subset implies information bound ≤"
        );
    }
}

fn main() {
    // This binary exists to host Kani harnesses. Under non-Kani builds,
    // it is inert; under cargo kani, the harness functions below are run.
    #[cfg(not(kani))]
    eprintln!("axonos-capability: Kani harness collection. Run with: cargo kani");
}
