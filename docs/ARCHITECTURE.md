# AxonOS Kernel Architecture

## Overview

AxonOS is a `#![no_std]` Rust microkernel for safety-critical brain-computer interface (BCI) systems on Cortex-M4F and Cortex-M33 bare-metal targets.

## Core Properties

| Property | Value | Evidence |
|----------|-------|----------|
| Scheduling | EDF (Earliest-Deadline-First) | [L1] Liu-Layland theorem |
| Admission ceiling | U_max = 0.25 | [L1] Proposition 5.4 |
| Binding utilisation | U^L2 = 0.2181 | [L2] Empirical WCRT |
| WCET (signal pipeline) | C_1^L2 = 818 µs | [L2] |
| Busy period | L = 972 µs | [L2] |
| Deadline slack | S_1 = 3028 µs | [L1] Theorem 5.9 |
| EDF jitter σ | 2.1 µs | [L2] |
| IPC latency | ≤ 0.2 µs | [L2] |
| Safe-idle timeout | ≤ 12 ms | [L2] DC5 |

## Module Hierarchy

```
axonos-kernel/
├── src/
│   ├── lib.rs              # Core types, evidence levels, config
│   ├── scheduler/          # EDF scheduler, admission control
│   │   ├── edf.rs          # EdfScheduler, busy period analysis
│   │   ├── task.rs         # Task, Job, Priority definitions
│   │   └── admission.rs    # AdmissionController, task set
│   ├── signal/             # Signal processing pipeline
│   │   ├── pipeline.rs     # SignalPipeline orchestrator
│   │   ├── fir.rs          # FIR bandpass filter
│   │   ├── kalman.rs       # Kalman state estimator
│   │   ├── notch.rs        # Powerline notch filter
│   │   ├── artifact.rs     # Artifact rejection
│   │   ├── csp.rs          # CSP spatial filter
│   │   └── lda.rs          # LDA classifier
│   ├── ringbuf/            # Zero-copy SPSC ring buffer
│   │   ├── spsc.rs         # SpscRingBuffer (Theorem 6.3)
│   │   └── sequence.rs     # Sequence number protocol
│   ├── ipc/                # Dual-core real-time contract
│   │   ├── dualcore.rs     # DualCoreContract implementation
│   │   └── contract.rs     # DC1-DC6 definitions
│   ├── capability/         # Capability-based isolation
│   │   ├── manifest.rs     # Application manifest
│   │   ├── catalogue.rs    # Permitted/prohibited types
│   │   └── dispatch.rs     # Event dispatch (Theorem 8.3)
│   ├── consent/            # Consent FSM and interlock
│   │   ├── fsm.rs          # ConsentFsm
│   │   └── interlock.rs    # StimulationInterlock
│   ├── attestation/        # HMAC-SHA256 attestation
│   │   └── hmac.rs         # ATECC608B interface
│   ├── platform/           # Hardware abstraction
│   │   ├── cortex_m4f.rs   # STM32F407 support
│   │   ├── cortex_m33.rs   # STM32H573 support
│   │   ├── dwt.rs          # DWT cycle counter
│   │   ├── gpio.rs         # GPIO for L3 validation
│   │   ├── dma.rs          # DMA controller
│   │   ├── spi.rs          # SPI interface
│   │   └── adc.rs          # ADS1299 driver
│   ├── hal/                # Hardware abstraction layer
│   │   ├── critical_section.rs
│   │   ├── memory.rs       # Memory barriers
│   │   └── irq.rs          # IRQ controller
│   └── zerocalib/          # ZeroCalib Riemannian classifier
       ├── riemannian.rs    # SPD manifold operations
       ├── mdm.rs           # MDM classifier
       └── alignment.rs     # Euclidean alignment
```

## Memory Safety

- `#![forbid(unsafe_code)]` across all modules
- Two targeted `unsafe` blocks in `ringbuf/spsc.rs` (formally justified by Theorem 6.3)
- Kani bounded model checking for SPSC protocol and heartbeat FSM

## Validation Taxonomy

| Level | Method | Hardware |
|-------|--------|----------|
| L1 | Instruction-count from assembly | None |
| L2 | DWT cycle counter | STM32F407 |
| L3 | Oscilloscope (Saleae Logic Pro 16) | STM32H573 |

## Reference Hardware

| Component | Part | Role |
|-----------|------|------|
| DSP core | STM32F407 Cortex-M4F @ 168 MHz | Hard real-time pipeline |
| App core | Cortex-A53 @ 1.2 GHz | Session, I/O, WASM sandbox |
| ADC | ADS1299 8-ch 24-bit 250 SPS | EEG acquisition |
| Secure element | ATECC608B HMAC-SHA256 | Attestation |
| BLE radio | nRF52840 BLE 5.3 | Intent egress |
| Isolation | ISO7741 5 kV | Galvanic isolation |
