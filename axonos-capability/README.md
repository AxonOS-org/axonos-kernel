# axonos-capability

[![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-0E2A47?style=flat-square)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-0E2A47?style=flat-square)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid-0E2A47?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![Kani verified](https://img.shields.io/badge/Kani-7%20proofs-0E2A47?style=flat-square)](./kani-proofs/)

Capability-based application isolation primitives for AxonOS.

`#![no_std]`. `#![forbid(unsafe_code)]`. Structural data minimisation by
absence — prohibited types do not exist in the type catalogue.

## What this crate does

This crate is the **policy enforcement path** of the AxonOS kernel. It
provides:

- The [`Capability`] enum: the complete vocabulary of admissible capabilities.
- The [`CapabilitySet`] bitfield: typed, normative wire format per RFC-0006.
- The [`Manifest`] / [`Catalogue`] / `verify_manifest` machinery: install-time
  admission of applications against the kernel's catalogue.
- The information-theoretic privacy bound: an analytic upper bound on the
  mutual information between raw EEG and the application-observable event
  stream, computed in fixed-point arithmetic with no floating-point
  dependency.

## Structural data minimisation

The central principle: **prohibited capabilities do not exist as enum
variants**. Raw EEG, continuous emotion inference, cognitive profile
reads, and re-identification are not denied by runtime check — they are
absent from the type system entirely.

This matters for two reasons:

1. **Cannot be requested.** An application that wishes to access raw
   neural data has no way to express the request: the type does not exist
   in the SDK.
2. **Cannot be delivered.** The kernel has no event-delivery function
   that accepts a forbidden type. The Kani harness `cap_c3_verify_sound`
   verifies that any manifest accepted by `verify_manifest` contains only
   capabilities admissible by the catalogue; the type system enforces
   that no other capability can appear in either.

The contrast is with conventional "deny by policy" approaches, where
prohibited operations exist at the type level but are denied at runtime
by access-control lists. Policy-based denial is breakable by a determined
application; structural absence is not.

## Quick start

```rust,no_run
use axonos_capability::{
    verify_manifest, Capability, CapabilitySet, Catalogue, Manifest,
};

// An application requests Navigation and SessionQuality.
let manifest = Manifest::new(
    CapabilitySet::singleton(Capability::Navigation)
        .with(Capability::SessionQuality),
);

// The kernel catalogue admits all four capabilities by default.
let catalogue = Catalogue::DEFAULT;

// Install-time verification.
verify_manifest(&manifest, &catalogue)
    .expect("manifest must be a subset of catalogue");

// Privacy bound: how many bits/s of EEG can leak through this manifest?
let bound_scaled = manifest.requested.information_bound_scaled();
let bound_bits_per_sec = bound_scaled as f64 / 1_000_000.0;
println!("Privacy bound: ≤ {:.2} bits/s", bound_bits_per_sec);
// → 119.27 bits/s (Navigation 116.10 + SessionQuality 3.17)
```

## The four permitted capabilities

| Capability | Max rate | Payload cardinality | Bound contribution |
|:---|:---:|:---:|:---:|
| `Navigation` | 50 Hz | 5 (Idle, L, R, U, D) | ≤ 116.10 bits/s |
| `WorkloadAdvisory` | 1 Hz | 3 (Low, Med, High) | ≤ 1.58 bits/s |
| `SessionQuality` | 2 Hz | 3 (Good, Degr, Lost) | ≤ 3.17 bits/s |
| `ArtifactEvents` | 10 Hz | 4 (Eye, Mu, Mo, El) | ≤ 20.00 bits/s |
| **Total (full catalogue)** | | | **≤ 140.85 bits/s** |

The total bound matches the analytic upper bound stated in the AxonOS
preprint, §VII (Theorem 8, Corollary 9). The match is exact in
fixed-point arithmetic at scale `1_000_000`.

## Verification

Seven Kani harnesses verify the crate's correctness properties:

| ID | Property |
|:---|:---|
| C1 | Subset implies contains for each element |
| C2 | `from_bits` rejects every reserved-bit pattern |
| C3 | `verify_manifest` is sound: accepted manifest has all caps in catalogue |
| C4 | `verify_manifest` is complete: rejection identifies excess capability |
| C5 | Subset relation is reflexive |
| C6 | Subset relation is transitive |
| C7 | Information bound is monotone under set inclusion |

To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup
cd kani-proofs
cargo kani
```

C3 is the most important: it establishes that the install-time admission
function correctly enforces the manifest-subset-of-catalogue invariant.
C7 is the most subtle: it establishes that any verified manifest inherits
a privacy bound no greater than the catalogue's bound, which is the
foundation of the kernel's compositional privacy guarantee.

## Threat model

Three adversaries are considered, in line with RFC-0005 and §VI of the
preprint:

- **A1** — malicious installed application. May have any internal logic;
  attempts to extract neural data through any channel.
- **A2** — compromised application with a memory-safety bug. Memory-safe
  app whose runtime state has been corrupted by external input.
- **A3** — network adversary. Observes only the application's external
  outputs (BLE radio, GPIO toggles, power consumption traces).

Against all three, the kernel's structural guarantee is:

1. **Type-level absence.** No event of a type outside the manifest is
   ever delivered to the application.
2. **Raw-data isolation.** No raw EEG tensor is ever serialised across
   the kernel-application boundary.
3. **Bounded information.** The total information transmitted to the
   application across all admitted capabilities is bounded by the
   analytic mutual-information ceiling computed by
   `information_bound_scaled`.

## Wire format

The `CapabilitySet` bitfield is the normative wire format per RFC-0006 §5:

```text
Bit    Mask          Capability
─────  ────────────  ─────────────────────
  0    0x00000001    Navigation
  1    0x00000002    WorkloadAdvisory
  2    0x00000004    SessionQuality
  3    0x00000008    ArtifactEvents
 4–31  0xFFFFFFF0    reserved (MUST be zero)
```

`CapabilitySet::from_bits` rejects any value with reserved bits set, per
RFC-0006's normative requirement that receivers reject non-conforming
bitfields. This is the implementation of the structural protection
against future protocol drift.

## Building for embedded targets

```bash
rustup target add thumbv7em-none-eabihf    # Cortex-M4F (STM32F407)
rustup target add thumbv8m.main-none-eabihf # Cortex-M33 (STM32H573)
cargo build --release --target thumbv7em-none-eabihf
```

## Stability

This crate is pre-1.0. The API may evolve. The wire-format bitfield is
already stable per RFC-0006 and will not change without a major version
bump of the RFC.

## License

Dual-licensed under either:

- Apache License, Version 2.0
- MIT License

at your option.

## Contributing

For security disclosures: `security@axonos.org`.
For general correspondence: `info@axonos.org`.

---

axonos.org · medium.com/@AxonOS · info@axonos.org
