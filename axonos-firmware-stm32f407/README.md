# axonos-firmware-stm32f407

[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=flat-square)](#license)
[![no_std no_main](https://img.shields.io/badge/no__std%20%2B%20no__main-yes-success?style=flat-square)](https://docs.rust-embedded.org/embedonomicon/)
[![forbid unsafe](https://img.shields.io/badge/unsafe-forbid-brightgreen?style=flat-square)](https://doc.rust-lang.org/reference/attributes/codegen.html)
[![target Cortex-M4F](https://img.shields.io/badge/target-thumbv7em--none--eabihf-purple?style=flat-square)](https://doc.rust-lang.org/rustc/platform-support/thumbv7em-none-eabi.html)

Bare-metal Cortex-M4F firmware for the AxonOS reference platform
(STM32F407 Discovery / Nucleo). Boots the
[`axonos-kernel-core`](../axonos-kernel-core) integration, wires the DWT
cycle counter as the [`MonotonicClock`](../axonos-time), and runs a
4-millisecond BCI signal tick.

## What this firmware does

1. **Boot.** Enables the DWT trace unit and cycle counter at the entry point.
2. **Construct the kernel.** Builds the reference BCI task set
   (signal pipeline, consent FSM, HMAC attestation, BLE egress, diagnostics)
   and admits it under Liu–Layland at `U_max = 0.25`.
3. **Validate the manifest.** Checks the application's capability set is
   a subset of the kernel catalogue.
4. **Tick loop.** Every 4 ms: picks the next task by EDF, generates a
   synthetic Navigation observation, validates against the capability
   gate, pushes the encoded record into the SPSC IPC ring.

This is a **scaffolding firmware** — it demonstrates that all the
foundational crates and the kernel-core integration compile and run on
real Cortex-M4F hardware with a real time source. It does **not** yet
include the signal-processing pipeline kernels (FIR, CSP, LDA,
Riemannian classifier).

## Hardware

- **MCU.** STM32F407VG (Cortex-M4F, 168 MHz, FPU, DSP extension)
- **Flash.** 1 MB
- **RAM.** 192 KB (128 KB SRAM + 64 KB CCMRAM)
- **Reference board.** STM32F407 Discovery (st.com part: STM32F4DISCOVERY)
  or Nucleo-F407ZG.

Production AxonOS clinical deployment targets STM32H573 (Cortex-M33 with
TrustZone, ARMv8-M); the F407 binary exists for development convenience
and ecosystem compatibility.

## Build

Requires the `thumbv7em-none-eabihf` target:

```bash
rustup target add thumbv7em-none-eabihf
cd axonos-firmware-stm32f407
cargo build --release
```

The release ELF binary is at:

```
target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407
```

## Flash and run

### On a real board (STM32F407 Discovery, ST-Link)

```bash
cargo install probe-rs --features cli
probe-rs run --chip STM32F407VGTx \
    target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407
```

### In QEMU (development only)

```bash
qemu-system-arm -cpu cortex-m4 -machine netduino2 \
    -kernel target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407 \
    -semihosting-config enable=on,target=native -nographic
```

## Architecture

```text
                 ┌──────────────────────────────────────┐
                 │      axonos-firmware-stm32f407       │
                 │                                      │
                 │  #[entry] fn main() -> ! {           │
                 │    DwtClock::enable();               │
                 │    let kernel = build_kernel()?;     │
                 │    let ipc = new_ipc_channel();      │
                 │    loop {                            │
                 │      wait_for_4ms_tick();            │
                 │      kernel.produce_observation();   │
                 │    }                                 │
                 │  }                                   │
                 └────────┬─────────────────────────────┘
                          │ depends on
                          ▼
   ┌────────────────────────────────────────────────────────┐
   │                  axonos-kernel-core                    │
   │   BciKernel<DwtClock, T_CAP=8, IPC_CAP=64>             │
   └──┬──────────┬──────────┬──────────┬──────────┬─────────┘
      │          │          │          │          │
      ▼          ▼          ▼          ▼          ▼
   ┌─────┐  ┌─────────┐ ┌─────────┐ ┌──────┐ ┌─────────┐
   │spsc │  │scheduler│ │capability│ │ time │ │ intent  │
   └─────┘  └─────────┘ └─────────┘ └──────┘ └─────────┘
   data       time        policy    clock    wire path
   path       path        path      path
```

The five foundational crates compose at the type level via
`axonos-kernel-core`. The firmware is the thinnest possible binding from
real hardware (DWT, GPIO, NVIC) to that composition.

## Memory layout

The `memory.x` linker script declares the STM32F407VG memory map per
RM0090:

| Region | Origin | Length |
|:---|:---|---:|
| FLASH | `0x08000000` | 1024 KB |
| RAM (SRAM1+SRAM2) | `0x20000000` | 128 KB |
| CCMRAM | `0x10000000` | 64 KB |

`cortex-m-rt` places `.text`, `.rodata` in FLASH; `.data`, `.bss`,
stack in RAM. CCMRAM is declared but unused by the scaffolding firmware;
production code uses it for scheduler state that does not require DMA
access.

## Verification status

This binary has not been executed on hardware in the repository CI.
It is verified only at the build level: `cargo build --release` must
succeed against `thumbv7em-none-eabihf`. The CI matrix runs this build
on every push.

**Phase-1 measurement (Q2 2026)** will exercise this firmware on a
GPIO-instrumented reference fixture with falsification thresholds set
in advance per the published preprint:

- P1: Sound WCRT `R ≤ 796 µs` measured over `≥ 10^4` epochs
- P2: Utilisation `U ≤ 0.25` under worst-case arrival
- P3: SPSC IPC round-trip `≤ 0.2 µs` over `10^6` trials
- P4: Halt-to-safe-idle `≤ 12 ms`
- P5: Capability audit — no prohibited type delivered in `10^9` fuzz calls

## Dependencies

| Crate | Purpose |
|:---|:---|
| [`cortex-m`](https://crates.io/crates/cortex-m) `0.7` | Cortex-M peripherals API |
| [`cortex-m-rt`](https://crates.io/crates/cortex-m-rt) `0.7` | Runtime, `#[entry]` macro, linker glue |
| [`panic-halt`](https://crates.io/crates/panic-halt) `0.2` | Panic handler — busy loop on fault |
| `axonos-kernel-core` | The integration layer |
| `axonos-spsc`, `axonos-scheduler`, `axonos-capability`, `axonos-time`, `axonos-intent` | Foundational crates |

No transitive heap-allocating dependency. The full call graph is
`#![no_std]`.

## License

Dual-licensed under either Apache-2.0 or MIT, at your option.
See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).

---

**Author:** Denis Yermakou · denis@axonos.org

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org)
