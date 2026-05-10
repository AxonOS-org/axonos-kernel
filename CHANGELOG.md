# Changelog

## [0.1.0] — 2026-05-05

### Added
- EDF scheduler with Liu-Layland schedulability test
- Five-stage signal processing pipeline (Kalman, FIR, Notch, Artifact, CSP, LDA)
- Zero-copy SPSC ring buffer with sequence-number protocol
- Dual-core real-time contract (DC1-DC6)
- Capability-based application isolation
- Consent FSM and stimulation interlock
- HMAC-SHA256 attestation interface
- Platform support for STM32F407 and STM32H573
- Kani bounded model checking proofs
- Comprehensive documentation and test suite

### Evidence
- L2 utilisation: U^L2 = 0.2181 < U_max = 0.25
- L2 WCRT: 972 µs (4.1× below 4 ms deadline)
- Zero deadline misses over 10.8×10^6 epochs
- EDF jitter: σ = 2.1 µs, P99.9 = 6.5 µs

### Pending
- L3 GPIO oscilloscope validation (Q2 2026)
- Direct power measurement
- Ferrocene toolchain qualification
