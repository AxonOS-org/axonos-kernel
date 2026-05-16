# axonos-time

[![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-0E2A47?style=flat-square)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-0E2A47?style=flat-square)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid-0E2A47?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![zero dependencies](https://img.shields.io/badge/dependencies-0-0E2A47?style=flat-square)](./Cargo.toml)
[![Kani verified](https://img.shields.io/badge/Kani-6%20proofs-0E2A47?style=flat-square)](./kani-proofs/)

Monotonic time abstraction for AxonOS.

`#![no_std]`. `#![forbid(unsafe_code)]`. **Zero dependencies.** Saturating
arithmetic everywhere. No panics on the hot path.

## Why this crate exists

The AxonOS kernel needs a single time abstraction that works across:

- **Cortex-M** targets via the DWT cycle counter
- **RISC-V** targets via the `mcycle` CSR
- **Host targets** for integration testing
- **Kani harnesses** for bounded model checking

Each hardware backend differs in resolution, wrap-around behaviour, and
configuration. The substantive work of designing a kernel scheduler must
not be entangled with those differences. `axonos-time` is the trait
boundary: it provides the canonical `Instant`/`Micros` types and the
[`MonotonicClock`] trait that decouples scheduler logic from hardware
clock state.

## Quick start

```rust
use axonos_time::{Instant, Micros, MockClock, MonotonicClock};

let clock = MockClock::new();
let start = clock.now();
clock.advance(Micros::from_millis(4)); // simulate a 4-ms epoch
let elapsed = clock.elapsed_since(start);
assert_eq!(elapsed, Micros(4_000));
```

## Design discipline

- **All arithmetic saturates.** No silent wrap-around, no runtime panic
  on overflow. `Instant::add_micros(u64::MAX)` is well-defined; it
  returns `Instant(u64::MAX)`.
- **All construction is `const fn`** where possible. Static clocks live
  in `.bss`.
- **No allocator, no dependencies.** Including `axonos-time` adds zero
  transitive dependencies to your build.
- **No `unsafe`.** The crate is `#![forbid(unsafe_code)]`.

## The `MonotonicClock` trait

```rust,no_run
pub trait MonotonicClock {
    fn now(&self) -> Instant;
    fn elapsed_since(&self, earlier: Instant) -> Micros { /* default */ }
}
```

### Contract

An implementation SHALL satisfy:

1. **Monotonicity.** For any two calls to `now()` on the same clock in
   program order, the second call returns an `Instant` `>=` the first.
2. **Resolution.** ≤ 1 microsecond on the AxonOS reference hardware. The
   STM32F407 DWT cycle counter gives ≈ 5.95 ns at 168 MHz, well within
   the requirement.
3. **Wait-freedom on the hot path.** `now()` SHOULD complete in bounded
   steps. The reference DWT implementation completes in ≤ 6 CPU cycles.
4. **No allocation.** Implementations SHALL NOT allocate.

### Built-in implementations

| Type | Backend | Availability |
|:---|:---|:---|
| [`MockClock`] | `AtomicU64`, controllable from tests | Always |
| `StdClock` | `std::time::Instant` | `--features std` |

### DWT integration

We deliberately do not ship a Cortex-M DWT implementation in this crate
because adding the `cortex-m` dependency would be meaningless on host
targets. The expected pattern is given in the crate docs and reproduced
here:

```rust,ignore
use axonos_time::{Instant, MonotonicClock};
use cortex_m::peripheral::DWT;

pub struct DwtClock { cpu_mhz: u32 }

impl MonotonicClock for DwtClock {
    fn now(&self) -> Instant {
        let cycles = DWT::cycle_count();
        Instant(u64::from(cycles) / u64::from(self.cpu_mhz))
    }
}
```

In production, `DWT::cycle_count()` is a 32-bit counter that wraps
after ≈ 25 seconds at 168 MHz; a wrap-tracking extension to 64 bits is
handled by a low-priority interrupt running at least every half-period.
See the forthcoming `axonos-time-dwt` integration crate for the
production pattern.

## Saturating semantics

The choice to saturate (rather than panic or wrap) is deliberate and
documented:

| Operation | Behaviour at limit |
|:---|:---|
| `Micros::saturating_add(MAX, ONE)` | returns `Micros::MAX`; no overflow |
| `Micros::saturating_sub(ZERO, ONE)` | returns `Micros::ZERO`; no underflow |
| `Instant::add_micros(u64::MAX, anything)` | returns `Instant(u64::MAX)` |
| `b.saturating_since(a)` when `b < a` | returns `Micros::ZERO` (not panic) |
| `b.saturating_since(a)` when `b − a > u32::MAX` | returns `Micros::MAX` |

For a real-time scheduler, this is the correct discipline: any input
that would cause an arithmetic anomaly is interpreted as a clamping
event, not as a violation. The scheduler can detect and respond to
clamping through additional checks (e.g., compare elapsed against
expected period), but it never observes undefined or panicking behaviour
from the clock primitives themselves.

## Verification

Six Kani harnesses verify correctness properties:

| ID | Property |
|:---|:---|
| T1 | `Instant::add_micros` is monotone (never decreases) |
| T2 | `saturating_since` is non-negative; backward direction saturates to zero |
| T3 | `saturating_since` is exact when the difference fits in `u32` |
| T4 | `saturating_since` saturates to `Micros::MAX` for large differences |
| T5 | `Micros::saturating_add` is monotone (`a + b ≥ a, b`) |
| T6 | `MockClock` advances are externally observable |

To reproduce:

```bash
cargo install --locked kani-verifier
cargo kani setup
cd kani-proofs
cargo kani
```

T1, T3, T4 are the substantive correctness theorems. T6 is the
behavioural specification of the mock clock.

## Building for embedded targets

```bash
rustup target add thumbv7em-none-eabihf    # Cortex-M4F (STM32F407)
rustup target add thumbv8m.main-none-eabihf # Cortex-M33 (STM32H573)
cargo build --release --target thumbv7em-none-eabihf
```

The default build is fully `no_std`. The `std` feature is opt-in and
enables `StdClock` for host-side integration testing only.

## Stability

This crate is pre-1.0. The `MonotonicClock` trait is the stable contract;
the data types `Instant` and `Micros` are pre-1.0 and may add methods.
No method will be removed without a major version bump.

## License

Dual-licensed: Apache-2.0 OR MIT.

---

axonos.org · medium.com/@AxonOS · info@axonos.org
