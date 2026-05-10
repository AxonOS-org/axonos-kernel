# AxonOS Kernel

[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)
[![Rust: 1.85.0+](https://img.shields.io/badge/rust-1.85.0%2B-orange.svg)](https://www.rust-lang.org)
[![No Std](https://img.shields.io/badge/no__std-supported-success.svg)]()
[![Unsafe: Forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)]()

> **Safety-critical `#![no_std]` Rust microkernel for brain-computer interface systems**

AxonOS is a bare-metal real-time operating system for Cortex-M4F and Cortex-M33 targets, designed for closed-loop neurostimulation and motor-imagery BCI applications.

## Key Features

- **EDF Scheduling**: Earliest-Deadline-First with Liu-Layland schedulability proof
- **Zero-Copy Signal Path**: SPSC ring buffer from ADC DMA to classifier
- **Capability Isolation**: Structural data minimisation at type-system level
- **Dual-Core Contract**: Formal timing contract between M4F DSP and A53 app core
- **Forbidden Unsafe**: `#![forbid(unsafe_code)]` except two targeted blocks in SPSC

## Evidence Levels

Every quantitative claim carries a mandatory evidence label:

| Level | Method | Hardware |
|-------|--------|----------|
| [L1] | Instruction-count from assembly | None |
| [L2] | DWT cycle counter | STM32F407 |
| [L3] | Oscilloscope (Saleae Logic Pro 16) | STM32H573 |

## Quick Start

```bash
# Clone repository
git clone https://github.com/AxonOS-org/axonos-kernel.git
cd axonos-kernel

# Build for STM32F407 (Cortex-M4F)
cargo build --target thumbv7em-none-eabihf --features cortex-m4f

# Build for STM32H573 (Cortex-M33 with TrustZone)
cargo build --target thumbv8m.main-none-eabihf --features cortex-m33,trustzone

# Run tests on host
cargo test --lib

# Run Kani proofs
cargo kani --features kani

# Build examples
cargo build --example basic_pipeline --target thumbv7em-none-eabihf
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Application Layer                     │
│  (Cortex-A53: Session, BLE/Wi-Fi, WASM sandbox)           │
├─────────────────────────────────────────────────────────────┤
│                      IPC Contract (DC1-DC6)               │
│         SPSC Ring Buffer │ Heartbeat │ Attestation        │
├─────────────────────────────────────────────────────────────┤
│                      AxonOS Kernel (Cortex-M4F)             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ EDF Scheduler│  │ Signal Pipe │  │ Capability Model │  │
│  │  (U≤0.25)   │  │ (640µs WCET)│  │  (Theorem 8.3)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Consent FSM │  │  Interlock  │  │  Attestation     │  │
│  │  (DC5)      │  │  (Safe-idle)│  │  (ATECC608B)    │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                      Hardware Abstraction                    │
│  ADS1299 (ADC) │ nRF52840 (BLE) │ ISO7741 (Isolation)     │
└─────────────────────────────────────────────────────────────┘
```

## Schedulability Guarantees

| Metric | Value | Evidence |
|--------|-------|----------|
| Admission ceiling | U_max = 0.25 | [L1] |
| Binding utilisation | U^L2 = 0.2181 | [L2] |
| WCET (signal pipeline) | C_1^L2 = 818 µs | [L2] |
| Busy period | L = 972 µs | [L2] |
| Deadline slack | S_1 = 3028 µs | [L1] |
| EDF jitter σ | 2.1 µs | [L2] |
| Zero deadline misses | 0 / 10.8×10^6 epochs | [L2] |

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — System overview and module hierarchy
- [Scheduler](docs/SCHEDULER.md) — EDF schedulability analysis
- [Signal Path](docs/SIGNAL_PATH.md) — Zero-copy pipeline specification
- [IPC Contract](docs/IPC_CONTRACT.md) — DC1-DC6 dual-core contract
- [Capability Model](docs/CAPABILITY_MODEL.md) — Structural isolation and privacy bounds
- [Validation](docs/VALIDATION.md) — L1/L2/L3 evidence taxonomy

## Safety Properties

- **Theorem 6.3**: SPSC sequence-number correctness (Release-Acquire)
- **Theorem 8.3**: Structural data minimisation (no prohibited types reach apps)
- **Theorem 9.1**: Mutual information bound ≤ 140.85 bits/s
- **Theorem 9.3**: Min-entropy residual ≥ H_∞(X) - 7.49 bits

## Kani Verification

Three bounded proofs for SPSC protocol:
- **K1**: No data race (unwind: 8, time: 4.2s)
- **K2**: Wait-freedom (unwind: 4, time: 1.1s)
- **K3**: Memory ordering / payload integrity (unwind: 2, time: 0.8s)

Three bounded proofs for heartbeat FSM (DC5):
- **K1**: Safety (unwind: 12, time: 2.3s)
- **K2**: Liveness (unwind: 12, time: 1.8s)
- **K3**: Monotonicity (unwind: 8, time: 0.9s)

## Regulatory Alignment

Preliminary IEC 62304 Class C alignment (pre-clinical engineering phase):

| Requirement | AxonOS Artifact | Status |
|-------------|----------------|--------|
| §5.1 Development plan | RFC-0001 to RFC-0005 | Planned |
| §5.3 Architectural design | RFC-0004 | Draft complete |
| §5.5 Unit implementation | Rust + clippy + CI | CI passing |
| §5.6 Verification | Unit-test coverage >90% | Partial |
| §5.7 Integration testing | 15/15 interop vectors | Complete |
| §5.8 System testing | Phase 1 GPIO test | [pending] Q2 2026 |

## Related Projects

- [axonos-sdk](https://github.com/AxonOS-org/axonos-sdk) — Application-facing SDK
- [axonos-rfcs](https://github.com/AxonOS-org/axonos-rfcs) — Engineering RFCs
- [axonos-consent](https://github.com/AxonOS-org/axonos-consent) — MMP Consent Extension

## License

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.

## Citation

```bibtex
@article{yermakou2026axonos,
  title={AxonOS: Analytical Real-Time Schedulability, Structural Capability Isolation, 
         and Empirical Validation of a Safety-Critical Brain Computer Interface Microkernel},
  author={Yermakou, Denis},
  journal={arXiv preprint},
  year={2026}
}
```

## Contact

Denis Yermakou — denis@axonos.org

AxonOS, Singapore
