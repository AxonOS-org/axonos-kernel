# axonos-spsc

[![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=flat-square)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-success?style=flat-square)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![Kani verified](https://img.shields.io/badge/Kani-verified-blueviolet?style=flat-square)](./kani-proofs/)

A single-producer, single-consumer ring buffer for AxonOS.

`#![no_std]`. No heap. Wait-free. Statically sized.
The unsafe surface is **exactly two operations**, each guarded by a Kani-verified
sequence-number invariant.

## Why this crate exists

The AxonOS kernel needs to pass neural-signal samples from the M4F real-time
core to the A53 application core through shared SRAM, with bounded-time
push and pop operations, under the hard 4-millisecond budget of a 250 Hz
acquisition cadence.

Standard SPSC implementations make compromises that AxonOS cannot afford:

- `heapless::spsc` requires `Send`-bound payloads, uses internal `Cell`
  patterns whose unsafe surface is not formally verified, and links
  against `core::sync::atomic` in ways that constrain memory ordering.
- `crossbeam-queue::ArrayQueue` requires `std`.
- `rtrb` is excellent but verifies via Loom (which is sound but operational),
  not by BMC of the unsafe surface.

`axonos-spsc` is designed for a single audience: BCI signal-pipeline use
cases where the unsafe surface needs to be machine-checkable end-to-end,
where the wait-free property must hold against an adversarial scheduler,
and where the FIFO property must hold under the ARMv7-M weak memory model.

## Quick start

```rust,no_run
use axonos_spsc::SpscBuffer;

let buffer: SpscBuffer<u32, 64> = SpscBuffer::new();
let (mut producer, mut consumer) = buffer.split().unwrap();

producer.try_push(42).unwrap();
assert_eq!(consumer.try_pop(), Ok(42));
```

## Design contract

- `#![no_std]`, no heap, statically sized via const generic `N`.
- `N` must be a power of two and at least 2. Checked by `debug_assert!`
  at construction.
- At most one `Producer` and one `Consumer` exist per buffer, enforced by
  `split()` returning `Option` and consuming the buffer's unique split flag.
- `try_push` is wait-free: completes in a bounded number of instructions
  regardless of any concurrent consumer activity.
- `try_pop` is wait-free under the symmetric condition.
- FIFO order is preserved across pushes; sequence-number arithmetic
  using `usize::wrapping_*` is sound for any realistic buffer size.

The unsafe surface is two operations:

1. `(*slot).write(value)` inside `Producer::try_push`,
2. `(*slot).assume_init_read()` inside `Consumer::try_pop`.

Each is preceded by an arithmetic check that the slot is in the safe
range (between `tail` and `head`) and followed by a Release-store on the
counter that publishes the side effect.

## Memory ordering

The implementation uses Release-store / Acquire-load on the `head` and
`tail` counters. The Rust/C++11 memory model guarantees that a write
performed before a Release-store is observed by any thread that reads the
same atomic with Acquire ordering and sees the new value (Boehm and Adve,
PLDI 2008). On ARMv7-M and ARMv8-A (the AxonOS target architectures), this
compiles to plain loads/stores with `dmb ish` memory barriers as
appropriate.

Because the M4F core has no data cache, no cache-maintenance operations
are required. On the A53 side, the shared SRAM region is mapped
`Device-nGnRnE` (non-cacheable, non-bufferable) in the MMU configuration,
which closes the cache-pressure timing-channel pathway by hardware design.

## Verification

The crate ships with five Kani harnesses in `kani-proofs/`:

| ID | Property | Bound |
|:---|:---|:---|
| K1 | Push then pop returns the same value | unwind 8 |
| K2 | `try_push` is wait-free (terminates without internal loop) | unwind 4 |
| K3 | FIFO order across two pushes and two pops | unwind 8 |
| K4 | Full signal: (N+1)th push on capacity-N buffer returns `Err(Full)` | unwind 6 |
| K5 | Empty signal: pop on drained buffer returns `Err(Empty)` | unwind 4 |

To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup
cd kani-proofs
cargo kani
```

The harnesses are written against a capacity-4 buffer, which is sufficient
for BMC parametricity reasoning: any property that holds for `N=4` holds
for any `N >= 4` by induction on the abstract state machine. Larger `N`
proofs are future work via TLA+.

## Building for embedded targets

```bash
rustup target add thumbv7em-none-eabihf    # Cortex-M4F (STM32F407)
rustup target add thumbv8m.main-none-eabihf # Cortex-M33 (STM32H573)
cargo build --release --target thumbv7em-none-eabihf
cargo build --release --target thumbv8m.main-none-eabihf
```

The release profile is configured for size and determinism:
`codegen-units = 1`, `lto = "fat"`, `panic = "abort"`, `debug = true`
(symbols retained for WCET analysis without affecting runtime size).

## Stability

This crate is pre-1.0. The API may evolve. The wire format of the
on-buffer storage is **not** stable across crate versions and must not be
relied upon for inter-process communication; for that, use the AxonOS
intent ABI specified in RFC-0006.

## License

Dual-licensed under either:

- Apache License, Version 2.0 (see [LICENSE-APACHE](LICENSE-APACHE))
- MIT License (see [LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

This crate is part of the AxonOS project. The contribution process,
including signing guidelines and security disclosure policy, is documented
in the parent organisation at https://github.com/AxonOS-org.

For security disclosures: `security@axonos.org`.
For general correspondence: `info@axonos.org`.

---

**Author:** Denis Yermakou · [denis@axonos.org](mailto:denis@axonos.org)

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org)
