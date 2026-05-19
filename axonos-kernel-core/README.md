# axonos-kernel-core

[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=flat-square)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-success?style=flat-square)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid-brightgreen?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![MSRV 1.75](https://img.shields.io/badge/MSRV-1.75-orange?style=flat-square)](https://blog.rust-lang.org/2023/12/28/Rust-1.75.0.html)

Integration layer for the AxonOS kernel. Composes the five foundational
crates ([`axonos-spsc`](../axonos-spsc), [`axonos-scheduler`](../axonos-scheduler),
[`axonos-capability`](../axonos-capability), [`axonos-time`](../axonos-time),
[`axonos-intent`](../axonos-intent)) into a coherent BCI signal pipeline running
on a 4-millisecond tick.

`#![no_std]`. `#![forbid(unsafe_code)]`. Hardware-independent — bring your own
[`MonotonicClock`](https://docs.rs/axonos-time).

## What this crate provides

- `BciKernel<C, T_CAP, IPC_CAP>` — a generic assembly of the scheduler,
  capability gate, SPSC IPC channel, time source, and intent encoder.
- `KernelConfig` — builder for the task set, manifest, IPC capacity,
  utilisation ceiling.
- `produce_observation` — capability-gated encode-and-push path.
- `schedule_tick` — pure EDF scheduling decision.

## What this crate does NOT provide

- Hardware initialisation (clock tree, GPIO, ADC, DMA, interrupt controller).
  That is the concern of [`axonos-firmware-stm32f407`](../axonos-firmware-stm32f407).
- Signal-processing pipeline kernels (FIR, CSP, LDA, Riemannian classifier).
  This crate is the scheduling and capability enforcement skeleton onto which
  signal processing is dropped in.

## Quick start

```rust,no_run
use axonos_kernel_core::{BciKernel, KernelConfig, new_ipc_channel};
use axonos_scheduler::{Task, TaskId, Micros};
use axonos_capability::{Capability, CapabilitySet, Manifest};
use axonos_intent::{Confidence, NavigationDirection};
use axonos_time::MockClock;

// 1. Declare the BCI task set with WCETs from the WCET analysis.
let mut config: KernelConfig<8, 64> = KernelConfig::new();
config.add_task(Task::periodic(TaskId(1), Micros(642), Micros(4000))).unwrap();
config.add_task(Task::periodic(TaskId(2), Micros(12),  Micros(4000))).unwrap();

// 2. Declare the application's capability manifest.
let manifest = Manifest::new(
    CapabilitySet::singleton(Capability::Navigation)
        .with(Capability::SessionQuality),
);

// 3. Construct the kernel. Admission test runs here; failure aborts.
let mut kernel: BciKernel<MockClock, 8, 64> =
    BciKernel::new(config, manifest, MockClock::new())
        .expect("Liu-Layland admission and manifest check must pass");

// 4. Wire up the IPC channel and produce one observation.
let ipc = new_ipc_channel::<64>();
let (mut producer, mut consumer) = ipc.split().unwrap();
let bytes = kernel.produce_observation(
    &mut producer,
    NavigationDirection::Right,
    Confidence::from_q0_16(0x8000),
).unwrap();

// 5. Drain through the consumer side.
let received = consumer.try_pop().unwrap();
assert_eq!(received, bytes);
```

## Integration tests

The crate ships with **10 integration tests** that exercise all five
foundational crates end-to-end:

| Test | What it verifies |
|:---|:---|
| `kernel_constructs_and_admits_pipeline` | Liu-Layland admission test at `U_max=0.25` |
| `response_time_bound_matches_preprint` | Computed `R = 696µs` matches preprint analysis |
| `admission_rejects_overloaded_pipeline` | High-utilisation set rejected with `AdmissionFailure` |
| `produce_observation_round_trips_through_ipc` | Encode → IPC → decode round-trip |
| `produce_rejects_capability_not_in_manifest` | Capability gate rejects forbidden kinds |
| `ipc_full_returns_specific_error` | SPSC full signal surfaces as `TickError::IpcFull` |
| `schedule_tick_picks_earliest_deadline` | EDF selection picks smallest absolute deadline |
| `schedule_tick_tie_breaks_by_id` | Deterministic tie-break by `TaskId` |
| `clock_advance_visible_through_kernel` | `MonotonicClock` plumbing functional |
| `manifest_rejecting_excess_capability_fails` | Manifest verification at construction |

Run:

```bash
cargo test -p axonos-kernel-core
```

## Numerical claims

Tests compute the following values **from code**, not as fixed strings:

| Quantity | Value | Source |
|:---|---:|:---|
| Total utilisation `U` | `0.174` | `kernel.utilisation_scaled() / 1_000_000` |
| Response time bound `R` | `696 µs` | `kernel.response_time_bound()` |
| Information bound (full catalogue) | `≤ 140.85 bits/s` | `axonos-capability` |

These numbers match the AxonOS preprint. Any change to the task set in code
changes the numbers in tests; the preprint claims are tied to verifiable
computation.

## Building

```bash
cargo build -p axonos-kernel-core --release
cargo test  -p axonos-kernel-core
```

For embedded targets:

```bash
rustup target add thumbv7em-none-eabihf      # Cortex-M4F
rustup target add thumbv8m.main-none-eabihf  # Cortex-M33
cargo build -p axonos-kernel-core --release --target thumbv7em-none-eabihf
```

## Stability

Pre-1.0. Const-generic capacities (`T_CAP`, `IPC_CAP`) are part of the public
API; future versions may add builder methods but will not remove existing
ones without a major bump.

## License

Dual-licensed under either Apache-2.0 or MIT, at your option.
See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).

---

**Author:** Denis Yermakou · denis@axonos.org

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org)
