# axonos-scheduler

[![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-0E2A47?style=flat-square)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-0E2A47?style=flat-square)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid-0E2A47?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![Kani verified](https://img.shields.io/badge/Kani-verified-0E2A47?style=flat-square)](./kani-proofs/)

Earliest-deadline-first (EDF) scheduling decision logic for AxonOS.

`#![no_std]`. `#![forbid(unsafe_code)]`. Pure-Rust algorithmic core.
The scheduling *decisions* of an EDF kernel — admission, deadline selection,
response-time analysis — separated from any hardware-specific concerns.

## Scope

This crate contains:

- Static task representation (`Task`, `TaskSet`, `TaskInstance`)
- Liu–Layland admission test (`TaskSet::admit`)
- Synchronous busy-period response-time analysis (`response_time_bound`)
- Earliest-deadline-first selection function (`select_next`)

This crate does NOT contain:

- Context switching (architecture-specific; requires `cortex-m` integration)
- Interrupt handlers
- Timer drivers
- Boot code
- Memory protection unit (MPU) setup

Those concerns are the responsibility of the full AxonOS kernel which
wraps this crate. This separation exists so that scheduling decisions can
be reasoned about, tested, and formally verified independently of any
specific hardware platform.

## Quick start

```rust,no_run
use axonos_scheduler::{Task, TaskId, TaskSet, Micros, response_time_bound};

// Construct the AxonOS BCI signal pipeline.
let mut set: TaskSet<8> = TaskSet::new();
set.push(Task::periodic(TaskId(1), Micros(642), Micros(4000))).unwrap();   // signal
set.push(Task::periodic(TaskId(2), Micros(12),  Micros(4000))).unwrap();   // consent FSM
set.push(Task::periodic(TaskId(3), Micros(18),  Micros(4000))).unwrap();   // HMAC
set.push(Task::periodic(TaskId(4), Micros(24),  Micros(4000))).unwrap();   // BLE egress

// Liu-Layland admission test at U_max = 0.25 (scaled by 1_000_000).
set.admit(250_000).expect("BCI pipeline must admit at U_max = 0.25");

// Compute the response-time bound via the synchronous busy-period equation.
let r = response_time_bound(&set);
assert_eq!(r, Micros(696));
```

## Mathematical model

A periodic task `tau_i = (C_i, T_i, D_i)` has worst-case execution time
`C_i`, period `T_i`, and relative deadline `D_i`. The current
implementation requires `D_i = T_i` (implicit-deadline task system),
which is the standard case for BCI signal pipelines.

**Liu–Layland EDF feasibility (1973).** For an implicit-deadline task set
on a uniprocessor under EDF, the task set is schedulable if and only if
the total utilisation `U = sum(C_i / T_i) <= 1`. `axonos-scheduler`
additionally enforces a user-supplied operational ceiling `U_max` (BCI
default: 0.25) for margin against unmodelled effects.

**Synchronous busy-period RTA (Baruah 2003).** The response-time bound is
the fixed point of:

```text
L_{k+1} = sum_j ceil(L_k / T_j) * C_j
```

starting from `L_0 = sum_j C_j`. For BCI workloads where all critical
tasks share a common period and `sum(C_i) < min(T_j)`, this fixed point
converges in one iteration.

## Why a separate scheduler crate?

The history of safety-critical kernels suggests that scheduling is the
single most subtle component. seL4 spent years on formally verifying its
priority scheduler. Hubris ships with a static priority scheme and
documents its limitations carefully. Tock has rewritten its scheduler
twice.

By factoring the scheduling decision into a pure-Rust library with no
unsafe code, no hardware dependence, and a formal verification surface,
we achieve three things:

1. **Independent reviewability.** Anyone can audit the scheduling logic
   without auditing the entire kernel.
2. **Independent testability.** Property-based and BMC verification can
   target the scheduling decision alone, in isolation from interrupt
   timing and hardware state.
3. **Independent evolution.** The scheduler can be improved (e.g., from
   EDF to mixed-criticality EDF-VD) without touching the hardware-bound
   layers of the kernel.

This is also the structure that NICTA's seL4 adopted for its proof
infrastructure: separate the kernel's logical state machine from its
hardware-specific implementation.

## Fixed-point arithmetic

The crate uses fixed-point arithmetic throughout (scaling factor
`1_000_000`) for utilisation calculations. No floating-point dependency,
no FPU requirement, no soft-float emulation overhead. This is required
for AxonOS's hot path where any non-deterministic execution (such as
software floating-point) would violate the WCET bound.

Verify the scale at runtime via `TaskSet::utilisation_scale()`.

## Verification

The crate ships with five Kani harnesses in `kani-proofs/`:

| ID | Property |
|:---|:---|
| S1 | Admission test is sound: passes iff `U <= U_max` |
| S2 | `select_next` returns the instance with the smallest absolute deadline |
| S3 | Tie-breaking is deterministic: lower `TaskId` wins |
| S4 | Single-task RTA equals the task's WCET |
| S5 | Empty task set is trivially schedulable with `R = 0` |

To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup
cd kani-proofs
cargo kani
```

## Building for embedded targets

```bash
rustup target add thumbv7em-none-eabihf    # Cortex-M4F (STM32F407)
rustup target add thumbv8m.main-none-eabihf # Cortex-M33 (STM32H573)
cargo build --release --target thumbv7em-none-eabihf
```

## Limitations and future work

- **Implicit deadlines only.** Constrained-deadline (`D_i < T_i`) is not
  yet supported. Required for some sensor-fusion workloads beyond BCI.
- **No mixed-criticality.** Mixed-criticality EDF-VD (Vestal 2007) is a
  natural extension for the safety-critical regulatory pathway.
- **No blocking.** Tasks with shared resources (mutexes, semaphores) are
  not yet modelled. The current contract is that all tasks are
  independent. Blocking will require the priority-inheritance or
  priority-ceiling protocol, which is a separate body of work.

These limitations are explicit. They will be addressed in subsequent
versions of this crate or in a successor crate (e.g.,
`axonos-scheduler-mcs`).

## Stability

This crate is pre-1.0. The API may evolve.

## License

Dual-licensed under either:

- Apache License, Version 2.0
- MIT License

at your option.

## Contributing

This crate is part of the AxonOS project. See
https://github.com/AxonOS-org for the contribution process.

For security disclosures: `security@axonos.org`.
For general correspondence: `info@axonos.org`.

---

axonos.org · medium.com/@AxonOS · info@axonos.org
