# AxonOS kernels

The foundational primitives of the AxonOS real-time microkernel for
brain-computer interfaces, factored as independent `#![no_std]` Rust
crates with formal verification harnesses.

This repository is the **kernel-side** counterpart to
[`axonos-sdk`](https://github.com/AxonOS-org/axonos-sdk) (the application
SDK) and [`axonos-rfcs`](https://github.com/AxonOS-org/axonos-rfcs) (the
engineering specifications). Each crate here is publishable on its own
and reviewable in isolation. The kernel itself is the result of composing
them with hardware-specific glue.

## Why three crates, not one kernel binary

A safety-critical kernel is the single subtlest piece of software in any
system. Reviewability scales inversely with size. We separate concerns
along the three orthogonal axes of a real-time scheduler:

| Crate | Responsibility | Axis |
|:---|:---|:---|
| [`axonos-spsc`](./axonos-spsc) | Inter-process communication between real-time and application domains | **Data path** |
| [`axonos-scheduler`](./axonos-scheduler) | Earliest-deadline-first admission and selection | **Time path** |
| [`axonos-capability`](./axonos-capability) | Application isolation, manifest verification, privacy bounds | **Policy path** |
| [`axonos-time`](./axonos-time) | Monotonic clock abstraction, saturating arithmetic | **Clock path** |

Each axis can be audited, fuzz-tested, and formally verified independently
of the others. Each is small enough to be read top-to-bottom in one
sitting. None contains hardware-specific code; the full AxonOS kernel
combines them with architecture-bound layers (timer drivers, interrupt
handlers, MPU configuration) that live elsewhere.

This factoring follows the seL4 tradition: the formally verifiable core
is a small set of pure-Rust algorithmic primitives, and the
non-verifiable hardware-bound parts are clearly demarcated.

## Engineering principles

These rules govern technical decisions in this repository and are
visible in every artifact:

1. **No claim above its evidence level.** Measurements are reported as
   measurements, derivations as derivations, predictions as predictions.
2. **No `unsafe` in reviewable modules.** Where unsafe is unavoidable
   (`axonos-spsc`'s payload path), it is exactly two operations, each
   guarded by a Kani-verified invariant. Everywhere else,
   `#![forbid(unsafe_code)]`.
3. **No heap allocation on the hot path.** Static buffers, const-generic
   sizing, fixed-point arithmetic.
4. **No silent recovery from inconsistent state.** Errors surface as
   `Result` types, never as defaults.
5. **No proprietary lock-in via the kernel.** All crates are dual-licensed
   under Apache-2.0 OR MIT. The wire formats are published as engineering
   RFCs under CC-BY-SA-4.0.

## Build and test

```bash
# Stable Rust 1.75+ for the crates themselves
cargo test --workspace

# Embedded targets
rustup target add thumbv7em-none-eabihf       # Cortex-M4F  — STM32F407
rustup target add thumbv8m.main-none-eabihf   # Cortex-M33  — STM32H573
cargo build --workspace --release --target thumbv7em-none-eabihf
cargo build --workspace --release --target thumbv8m.main-none-eabihf

# Lint
rustup component add clippy
cargo clippy --workspace --all-targets -- -D warnings
```

## Formal verification

Each crate ships with a `kani-proofs/` sub-package containing BMC
harnesses. To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup

cd axonos-spsc/kani-proofs       && cargo kani
cd axonos-scheduler/kani-proofs  && cargo kani
cd axonos-capability/kani-proofs && cargo kani
```

The harnesses are listed in each crate's README. A summary:

| Crate | Harness count | Domain |
|:---|---:|:---|
| `axonos-spsc` | 5 | SPSC ring buffer correctness, FIFO order, wait-freedom |
| `axonos-scheduler` | 5 | EDF admission soundness, deadline selection, tie-breaking |
| `axonos-capability` | 7 | Subset relation, manifest soundness/completeness, monotone bound |
| `axonos-time` | 6 | Monotonic arithmetic, saturating overflow, clock observability |

**Total: 23 formal harnesses.** All are runnable against the published
source via `cargo kani`. No proof is taken on trust.

## Status of measurement-backed claims

This repository contains **derived** quantitative claims (computed from
algorithms in the crates themselves) and **specification** quantitative
claims (cited from primary hardware documentation). It does not contain
runtime-measured claims from the AxonOS reference hardware; those will
be published in Phase 1 (Q2 2026) per the falsification protocol stated
in the preprint.

Specifically:
- `axonos-scheduler` tests compute that the AxonOS BCI pipeline task set
  admits at `U=0.174` and has a response-time bound of `R=796 µs`. These
  numbers are derived in code and match the preprint.
- `axonos-capability` tests compute that the full-catalogue information
  bound is `≤ 140.85 bits/s`. This number is derived in code and matches
  the preprint.
- No claim from this repository says "we measured X µs on a real board."
  Such claims are reserved for the Phase 1 measurement publication.

## Repository layout

```
axonos-kernels/
├── Cargo.toml                       Workspace manifest
├── README.md                        This file
├── .github/workflows/ci.yml         CI matrix: test, clippy, fmt, no_std, Miri, Kani, audit
├── axonos-spsc/
│   ├── Cargo.toml
│   ├── README.md
│   ├── src/lib.rs
│   ├── kani-proofs/                 K1–K5 BMC harnesses
│   └── LICENSE-{APACHE,MIT}
├── axonos-scheduler/
│   ├── Cargo.toml
│   ├── README.md
│   ├── src/lib.rs
│   ├── kani-proofs/                 S1–S5 BMC harnesses
│   └── LICENSE-{APACHE,MIT}
├── axonos-capability/
│   ├── Cargo.toml
│   ├── README.md
│   ├── src/lib.rs
│   ├── kani-proofs/                 C1–C7 BMC harnesses
│   └── LICENSE-{APACHE,MIT}
└── axonos-time/
    ├── Cargo.toml
    ├── README.md
    ├── src/lib.rs
    ├── kani-proofs/                 T1–T6 BMC harnesses
    └── LICENSE-{APACHE,MIT}
```

## Roadmap

**Now.** Four foundational crates: SPSC, scheduler, capability, time.
All publishable, all `#![no_std]`, all formally verified at the
relevant properties. Zero dependencies between crates other than
`axonos-time` providing the clock abstraction.

**Next.** Two integration crates planned, each in the same discipline:

- `axonos-intent` — typed event payloads matching RFC-0006's wire
  format. Conformance test vectors.
- `axonos-core` — bare-metal Cortex-M4F binary integrating the above
  into a runnable demonstration. The first **kernel**, not library.

**Phase 1 (Q2 2026).** GPIO-instrumented WCRT measurement on STM32H573
reference fixture. Falsification protocol P1–P5 executed and published
regardless of outcome.

**Phase 2 (Q3–Q4 2026).** First 8-channel clinical kit deployment with
the partner ALS rehabilitation centre.

**Phase 3 (2027).** FDA Pre-Submission. Ferrocene-qualified toolchain
integration. ISO 14971 risk management file.

## License

Dual-licensed: Apache-2.0 OR MIT. See each crate for licence files.

## Contributing

- Security disclosures: `security@axonos.org`
- General correspondence: `info@axonos.org`
- Partnership and investment: `connect@axonos.org`

---

axonos.org · medium.com/@AxonOS · info@axonos.org · Zurich · Berlin · Milano · San Mateo · Singapore
