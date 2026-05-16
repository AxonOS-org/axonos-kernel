# axonos-intent

[![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-0E2A47?style=flat-square)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-0E2A47?style=flat-square)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid-0E2A47?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![Kani verified](https://img.shields.io/badge/Kani-5%20proofs-0E2A47?style=flat-square)](./kani-proofs/)

Typed intent observations matching the AxonOS wire format
([RFC-0006](https://github.com/AxonOS-org/axonos-rfcs/blob/main/rfcs/0006-intent-wire-format-abi.md)).

`#![no_std]`. `#![forbid(unsafe_code)]`. Strict round-trip-stable
encoder/decoder for the 32-byte observation record.

## Wire format (RFC-0006 §4.1)

```text
offset  size  field
─────   ───   ─────────────────────────────────────
   0    8     timestamp_us         u64 LE, ≤ 2^48
   8    1     kind_tag             u8, 0x00–0x03
   9    1     direction            u8, kind-specific
  10    2     confidence_q0_16     u16 LE, Q0.16
  12    4     sequence             u32 LE
  16    8     attestation_tag      u8[8], HMAC-SHA256 truncated
  24    8     reserved             u8[8], MUST be zero
                                   ────
                                   32 bytes total, 8-byte aligned
```

Verified at compile time via `const _: () = assert!(...)`. If the
constants `OBSERVATION_SIZE` or `OBSERVATION_ALIGN` ever drift from the
normative values, the build breaks.

## Quick start

```rust,no_run
use axonos_intent::{
    IntentObservation, Kind, Direction, NavigationDirection,
    Confidence, AttestationTag,
};
use axonos_time::Instant;

let obs = IntentObservation {
    timestamp: Instant(4_000),
    kind: Kind::Navigation,
    direction: Direction::Navigation(NavigationDirection::Right),
    confidence: Confidence::from_q0_16(0xA800), // ≈ 0.66
    sequence: 42,
    attestation: AttestationTag([0; 8]),
};

let bytes: [u8; 32] = obs.encode();
let decoded = IntentObservation::decode(&bytes).unwrap();
assert_eq!(decoded, obs);
```

## Strict decoding

The decoder rejects every deviation from the RFC-0006 normative
requirements with a specific `DecodeError`:

| Failure | Variant |
|:---|:---|
| `kind_tag` outside [0x00, 0x03] | `InvalidKindTag { tag }` |
| `direction` outside the per-kind range | `InvalidDirection { kind, byte }` |
| `timestamp_us` > 2^48 (session envelope) | `TimestampOutOfRange { timestamp }` |
| any byte in offsets 24..32 is non-zero | `NonZeroReservedByte { offset, value }` |

This is structural protection against future protocol drift. A producer
cannot smuggle extra data through reserved bytes, cannot signal
capabilities outside the catalogue through future-reserved tags, and
cannot claim timestamps beyond the documented session envelope.

## Cross-references

- **`axonos-capability`** — the `Capability` enum that each `Kind` maps to.
- **`axonos-time`** — the `Instant` type used in the timestamp field.
- **RFC-0006 §4** — wire format specification.
- **RFC-0006 §6** — attestation algorithm (this crate carries the tag;
  HMAC verification is downstream).

## Verification

Five Kani harnesses verify correctness properties:

| ID | Property |
|:---|:---|
| I1 | `decode(encode(o)) == o` for every well-formed observation |
| I2 | Encoded size is always exactly 32 bytes |
| I3 | Every `kind_tag` outside [0..=3] is rejected as `InvalidKindTag` |
| I4 | Every timestamp > 2^48 is rejected as `TimestampOutOfRange` |
| I5 | Every non-zero reserved byte is rejected as `NonZeroReservedByte` |

To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup
cd kani-proofs
cargo kani
```

I1 is the round-trip identity, the structural soundness property of the
encoder. I3–I5 are the structural rejection properties — the
verification that the decoder enforces RFC-0006 strictly.

## Conformance vectors

Test `vector_1_decodes_to_expected` in `src/lib.rs` exercises Appendix A.1
of RFC-0006: a Navigation/Right observation at timestamp 1000 µs,
confidence 0x8000, sequence 1. Any implementation claiming RFC-0006
compliance must decode this byte sequence identically.

## Stability

This crate is pre-1.0. The wire format is **frozen** under RFC-0006 §7.
Adding a new kind or direction requires a minor RFC bump. The Rust API
may evolve before 1.0.

## License

Dual-licensed: Apache-2.0 OR MIT.

---

axonos.org · medium.com/@AxonOS · info@axonos.org
