<div align="center">

<img src="https://rustacean.net/assets/rustacean-flat-happy.svg" width="120" alt="Ferris, the Rust mascot" />

# axonos-kernels

### the verifiable substrate underneath a brain–computer interface

```
seven crates · 3 603 lines · 28 formal proofs · 66 tests · zero unsafe outside two operations
```

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=for-the-badge)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-success?style=for-the-badge)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![Kani verified](https://img.shields.io/badge/Kani-28%20BMC%20proofs-blueviolet?style=for-the-badge)](https://github.com/model-checking/kani)

[![MSRV](https://img.shields.io/badge/MSRV-1.75-orange?style=flat-square)](https://blog.rust-lang.org/2023/12/28/Rust-1.75.0.html)
[![Cortex-M4F](https://img.shields.io/badge/target-Cortex--M4F-purple?style=flat-square)](https://doc.rust-lang.org/rustc/platform-support/thumbv7em-none-eabi.html)
[![Cortex-M33](https://img.shields.io/badge/target-Cortex--M33-purple?style=flat-square)](https://doc.rust-lang.org/rustc/platform-support/thumbv8m.main-none-eabi.html)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid_outside_2_ops-brightgreen?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![Workspace](https://img.shields.io/badge/cargo-workspace-yellow?style=flat-square&logo=rust)](https://doc.rust-lang.org/cargo/reference/workspaces.html)

[**About**](./ABOUT.md) · [**Architecture**](#architecture) · [**Crates**](#crates) · [**Build**](#build) · [**Verification**](#verification) · [**Contributing**](./CONTRIBUTING.md) · [**Licence**](#license)

</div>

---

## In one paragraph

A closed-loop brain–computer interface is a hard-real-time medical
device measured in microseconds. Most BCI software in 2026 is not built
on a foundation that can carry that claim. `axonos-kernels` is the
start of one — seven Rust crates that compose into a verifiable kernel
substrate for BCI signal pipelines on Cortex-M class microcontrollers,
with the scheduling decision, the inter-process communication
primitive, the capability gate, the time source, and the application
binary interface each factored into its own auditable, formally checked
component.

## What you are looking at

This repository is the **kernel side** of the [AxonOS](https://axonos.org)
project. It contains:

- Five **foundational crates** — `spsc`, `scheduler`, `capability`,
  `time`, `intent` — each a single concern, each independently
  reviewable, each shipping with Kani bounded-model-checking harnesses
  for its safety-critical invariants.
- An **integration layer** — `axonos-kernel-core` — that composes the
  foundational crates into a coherent BCI signal pipeline.
- A **bare-metal binary** — `axonos-firmware-stm32f407` — that boots
  the kernel on the reference Cortex-M4F target with a DWT-backed
  monotonic clock and a 4-millisecond tick.

The full purpose, audience, and market context are described in
[ABOUT.md](./ABOUT.md). The legal terms and attribution requirements
are in [LICENSE-APACHE](./LICENSE-APACHE), [LICENSE-MIT](./LICENSE-MIT),
and [NOTICE](./NOTICE). Forking is welcome and the procedure takes
three clicks — see [CONTRIBUTING.md](./CONTRIBUTING.md).

---

## Architecture

We separate concerns along the orthogonal axes of a real-time scheduler.
Each crate owns one axis. The integration layer composes them. The
firmware binary binds the composition to silicon.

```
        ┌──────────────────────────────────────────────────────────┐
        │            axonos-firmware-stm32f407 (binary)            │
        │                                                          │
        │   #[entry] fn main() -> ! {                              │
        │       DwtClock::enable();                                │
        │       let kernel = build_kernel()?;                      │
        │       loop { kernel.tick(); }                            │
        │   }                                                      │
        └────────────────────────────┬─────────────────────────────┘
                                     ▼
        ┌──────────────────────────────────────────────────────────┐
        │           axonos-kernel-core  (integration)              │
        │                                                          │
        │   BciKernel<Clock, TaskCap, IpcCap>                      │
        │     · Liu–Layland admission at construction              │
        │     · Manifest verification against catalogue            │
        │     · EDF scheduling tick                                │
        │     · Capability-gated observation production            │
        └─┬───────────┬──────────────┬────────────┬───────────┬────┘
          │           │              │            │           │
          ▼           ▼              ▼            ▼           ▼
       ╭─────╮  ╭─────────╮  ╭──────────────╮ ╭───────╮ ╭──────────╮
       │spsc │  │scheduler│  │  capability  │ │ time  │ │  intent  │
       ╰─────╯  ╰─────────╯  ╰──────────────╯ ╰───────╯ ╰──────────╯
          ↑           ↑             ↑             ↑          ↑
        K1–K5      S1–S5         C1–C7         T1–T6      I1–I5
        Kani       Kani          Kani          Kani       Kani

       data path  time path   policy path  clock path  wire path
```

This factoring follows the [seL4](https://sel4.systems/) tradition:
the formally verifiable core is a small set of pure-Rust primitives;
hardware-bound parts are clearly demarcated.

---

## Crates

| Crate | Purpose | LOC | Tests | Kani |
|:---|:---|---:|---:|---:|
| [`axonos-spsc`](./axonos-spsc) | Single-producer/single-consumer ring buffer | 445 | 8 | 5 |
| [`axonos-scheduler`](./axonos-scheduler) | EDF admission, response-time, deadline selection | 557 | 10 | 5 |
| [`axonos-capability`](./axonos-capability) | Manifest verification, privacy bounds | 661 | 14 | 7 |
| [`axonos-time`](./axonos-time) | Monotonic clock trait, saturating arithmetic | 518 | 13 | 6 |
| [`axonos-intent`](./axonos-intent) | RFC-0006 wire format, conformance vectors | 621 | 11 | 5 |
| [`axonos-kernel-core`](./axonos-kernel-core) | Integration layer composing the foundation | 586 | 10 | — |
| [`axonos-firmware-stm32f407`](./axonos-firmware-stm32f407) | Bare-metal Cortex-M4F binary, DWT clock | 215 | — | — |
| **Total** | | **3 603** | **66** | **28** |

Each crate's `README.md` documents its API, its verification status,
and its specific dependencies.

---

## Quick start

### 30-second tour

```bash
# Clone and test
git clone https://github.com/AxonOS-org/axonos-kernels
cd axonos-kernels
cargo test --workspace
# → 66 tests passed.
```

### One-minute integration

```rust
use axonos_kernel_core::{BciKernel, KernelConfig, new_ipc_channel};
use axonos_scheduler::{Task, TaskId, Micros};
use axonos_capability::{Capability, CapabilitySet, Manifest};
use axonos_intent::{Confidence, NavigationDirection};
use axonos_time::MockClock;

// 1. Declare the BCI task set with WCETs from analysis.
let mut config: KernelConfig<8, 64> = KernelConfig::new();
config.add_task(Task::periodic(TaskId(1), Micros(642), Micros(4000)))?;
config.add_task(Task::periodic(TaskId(2), Micros(12),  Micros(4000)))?;

// 2. Declare the application's capability manifest.
let manifest = Manifest::new(
    CapabilitySet::singleton(Capability::Navigation)
        .with(Capability::SessionQuality),
);

// 3. Construct. Liu–Layland admission runs here. Failure aborts.
let mut kernel: BciKernel<MockClock, 8, 64> =
    BciKernel::new(config, manifest, MockClock::new())?;

// 4. Produce one observation through the capability gate.
let ipc = new_ipc_channel::<64>();
let (mut producer, mut consumer) = ipc.split().unwrap();
kernel.produce_observation(
    &mut producer,
    NavigationDirection::Right,
    Confidence::from_q0_16(0x8000),
)?;
```

---

## Build

### Host

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
```

### Embedded

```bash
rustup target add thumbv7em-none-eabihf       # Cortex-M4F — STM32F407
rustup target add thumbv8m.main-none-eabihf   # Cortex-M33 — STM32H573
cargo build --workspace --release --target thumbv7em-none-eabihf
cargo build --workspace --release --target thumbv8m.main-none-eabihf
```

### Firmware (bare-metal)

```bash
cd axonos-firmware-stm32f407
cargo build --release
# → target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407
```

Flash with [probe-rs](https://probe.rs/):

```bash
cargo install probe-rs --features cli
probe-rs run --chip STM32F407VGTx \
    target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407
```

---

## Verification

Every crate ships with a `kani-proofs/` sub-package. To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup

cd axonos-spsc/kani-proofs       && cargo kani
cd axonos-scheduler/kani-proofs  && cargo kani
cd axonos-capability/kani-proofs && cargo kani
cd axonos-time/kani-proofs       && cargo kani
cd axonos-intent/kani-proofs     && cargo kani
```

| ID range | Crate | What is proved |
|:---|:---|:---|
| K1–K5 | `axonos-spsc` | Round-trip identity; wait-freedom; FIFO; full/empty signals |
| S1–S5 | `axonos-scheduler` | Admission soundness; EDF correctness; deterministic tie-break |
| C1–C7 | `axonos-capability` | Subset relation; manifest soundness/completeness; monotone bound |
| T1–T6 | `axonos-time` | Monotonic arithmetic; saturation; clock observability |
| I1–I5 | `axonos-intent` | Round-trip identity; strict decoder rejection of malformed input |

**Total: 28 BMC harnesses.** Runnable against the published source.

---

## Engineering principles

These principles govern every technical decision and are visible in
every artifact:

1. **No claim above its evidence level.** Measurements are reported as
   measurements, derivations as derivations, predictions as predictions.
2. **No `unsafe` in reviewable modules.** Where unsafe is unavoidable
   (`axonos-spsc`'s payload path), it is exactly two operations, each
   guarded by a Kani-verified invariant. Everywhere else,
   `#![forbid(unsafe_code)]`.
3. **No heap allocation on the hot path.** Static buffers, const-generic
   sizing, fixed-point arithmetic.
4. **No silent recovery from inconsistent state.** Errors surface as
   exhaustive `Result` enums, never as defaults.
5. **No proprietary lock-in via the kernel.** All crates are dual-licensed
   under Apache-2.0 OR MIT. The wire formats are published as engineering
   RFCs.

---

## Continuous integration

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs **13 jobs** on every push:

| # | Job | Purpose |
|:---|:---|:---|
| 1–3 | Test (ubuntu, macos, windows) | Cross-platform host correctness |
| 4 | rustfmt | Formatting consistency |
| 5 | clippy `-D warnings` | Lint cleanliness |
| 6 | MSRV 1.75 | Pinned minimum supported Rust |
| 7–8 | no_std (thumbv7em, thumbv8m) | Embedded target verification |
| 9 | docs `-D rustdoc warnings` | API docs build clean |
| 10 | miri (`axonos-spsc`) | Undefined-behaviour detection on the unsafe surface |
| 11 | kani × 5 crates | Bounded model checking |
| 12 | firmware (STM32F407) | Bare-metal firmware build |
| 13 | cargo-deny | Licence and advisory database check |

Local CI mirror — replicate every check before you push:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --all-features
cargo build --workspace --release --target thumbv7em-none-eabihf
```

---

## Status of quantitative claims

This repository contains two kinds of quantitative claims:

- **Derived** — computed from algorithms in the crates themselves.
- **Specification** — cited from primary hardware documentation.

It contains **no** runtime-measured claims from the AxonOS reference
hardware. Hardware measurement is Phase-1 work, scheduled for Q2 2026
under the falsification protocol described in the preprint.

Specifically:

- `axonos-scheduler` tests compute that the AxonOS BCI pipeline task set
  admits at `U = 0.174` and has a response-time bound of `R = 796 µs`.
  These numbers are derived in code; the test asserts them.
- `axonos-capability` tests compute that the full-catalogue information
  bound is `≤ 140.85 bits/s`. Derived in code; the test asserts it.
- No claim in this repository says "we measured X µs on real hardware."

Change any of these algorithms and the asserted numbers change. The
numbers in the preprint are tied to verifiable computation in this
repository.

---

## Roadmap

| Phase | Window | Deliverable |
|:---|:---|:---|
| **Now** | May 2026 | Seven crates published. 66 tests, 28 Kani harnesses, CI green. |
| **Phase 1** | Q2 2026 | GPIO-instrumented WCRT measurement on STM32H573 reference fixture. Falsification protocol P1–P5 executed and published regardless of outcome. |
| **Phase 2** | Q3–Q4 2026 | First 8-channel clinical kit deployment with the partner ALS rehabilitation centre. |
| **Phase 3** | 2027 | FDA Pre-Submission. Ferrocene-qualified toolchain integration. ISO 14971 risk management file. |

---

## Intellectual property

This codebase is dual-licensed under **Apache-2.0 OR MIT** at the user's
option. The dual licensing is permissive: you may use, modify,
redistribute, and commercialise this code, including in closed-source
proprietary products.

### What you must preserve

- The SPDX licence header on every source file.
- The copyright attribution to Denis Yermakou.
- The [NOTICE](./NOTICE) file in any derivative redistribution
  (Apache-2.0 § 4(d)).
- The licence texts themselves ([LICENSE-APACHE](./LICENSE-APACHE),
  [LICENSE-MIT](./LICENSE-MIT)).

### What is protected separately

The name **"AxonOS"** is an unregistered word mark of Denis Yermakou.
You may state your project is "based on AxonOS" as a factual description.
You may not name your fork "AxonOS Pro" or similar in a way that implies
endorsement. Full trademark policy in [NOTICE](./NOTICE).

### Patent grant

Under Apache-2.0 § 3, contributors grant a perpetual, irrevocable
patent licence to users for any patent claims their contributions
necessarily infringe. The grant terminates for any party that initiates
patent litigation against the project. Details in
[LICENSE-APACHE](./LICENSE-APACHE).

### Forks are welcome

We actively encourage forks for research, education, and downstream
products. The fork procedure takes three clicks; see
[CONTRIBUTING.md](./CONTRIBUTING.md). If you build something with this,
we would like to hear about it: `info@axonos.org`.

---

## Workspace structure

```
axonos-kernels/
├── README.md                              ← this file
├── ABOUT.md                               ← purpose, audience, market
├── CONTRIBUTING.md                        ← fork in 3 clicks, attribution
├── NOTICE                                 ← Apache-2.0 attribution
├── LICENSE-APACHE                         ← full Apache 2.0 text
├── LICENSE-MIT                            ← full MIT text
├── Cargo.toml                             ← workspace manifest
├── deny.toml                              ← cargo-deny configuration
├── rustfmt.toml                           ← formatting style
├── .github/workflows/ci.yml               ← 13 CI jobs
│
├── axonos-spsc/                           ← data path · K1–K5 BMC
├── axonos-scheduler/                      ← time path · S1–S5 BMC
├── axonos-capability/                     ← policy path · C1–C7 BMC
├── axonos-time/                           ← clock path · T1–T6 BMC
├── axonos-intent/                         ← wire path · I1–I5 BMC
├── axonos-kernel-core/                    ← integration layer
└── axonos-firmware-stm32f407/             ← bare-metal Cortex-M4F binary
```

---

## License

Dual-licensed at your choice under either:

- **[Apache License, Version 2.0](./LICENSE-APACHE)** ([upstream](http://www.apache.org/licenses/LICENSE-2.0))
- **[MIT License](./LICENSE-MIT)** ([upstream](https://opensource.org/licenses/MIT))

Unless explicitly stated otherwise, any contribution intentionally
submitted for inclusion in this work shall be dual-licensed as above,
without any additional terms or conditions. This is the standard
"inbound = outbound" model used by the Rust project itself.

See [NOTICE](./NOTICE) for required Apache-2.0 attribution. See
[CONTRIBUTING.md](./CONTRIBUTING.md) for the fork procedure and the
post-fork compliance burden (which is small).

---

## Related repositories

- **[`axonos-rfcs`](https://github.com/AxonOS-org/axonos-rfcs)** —
  Engineering specifications (RFC-0001 through RFC-0006).
- **[`axonos-sdk`](https://github.com/AxonOS-org/axonos-sdk)** —
  Application-side SDK for consuming intent observations.
- **Project website:** [axonos.org](https://axonos.org).
- **Long-form essays:** [medium.com/@AxonOS](https://medium.com/@AxonOS).

---

<div align="center">

**Author:** Denis Yermakou · [denis@axonos.org](mailto:denis@axonos.org)

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org)

Zurich · Berlin · Milano · San Mateo · Singapore

<sub>Made with 🦀 and a long real-time tick.</sub>

</div>
