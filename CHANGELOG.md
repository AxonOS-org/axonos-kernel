# Changelog

All notable changes to the AxonOS kernel foundational crates will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.1] — 2026-05-16

### Regulatory Status
**NOT FOR CLINICAL USE.** This release contains pre-submission design controls and
verification evidence only. It has not been cleared or approved by any regulatory
body (FDA, Notified Body, etc.). Integration into a medical device requires a full
quality management system per ISO 13485 and compliance with local regulations.

### Added
- `axonos-scheduler` — Earliest-deadline-first (EDF) scheduling decision logic.
  Liu–Layland admission test with fixed-point utilisation arithmetic,
  synchronous busy-period response-time analysis, and deterministic deadline
  tie-breaking.
- `axonos-spsc` — Single-producer, single-consumer ring buffer. Wait-free
  `try_push` / `try_pop`. Memory ordering via Release/Acquire sequence counters.
  Unsafe surface is exactly two operations (`ptr::write`, `assume_init_read`);
  both guarded by Kani-verified invariants.
- `axonos-capability` — Capability-based application isolation. Structural data
  minimisation by absence: prohibited neural-data types do not exist as enum
  variants. Analytic mutual-information upper bound computed in fixed-point
  arithmetic.
- `axonos-time` — Monotonic clock abstraction (`Instant`, `Micros`). Saturating
  arithmetic, no panics on the hot path. `MonotonicClock` trait for hardware
  integration; `MockClock` for testing and bounded model checking.
- `axonos-intent` — Strict RFC-0006 wire-format encoder/decoder for typed intent
  observations. 32-byte record layout verified at compile time. All decoding is
  rejecting: invalid kind tags, out-of-range direction bytes, non-zero reserved
  fields, and timestamps beyond the session envelope are refused with specific
  errors.
- 28 Kani bounded model checking (BMC) harnesses across all five crates.
- Continuous integration: test (Linux/macOS/Windows), rustfmt, Clippy, MSRV
  (Rust 1.75), `no_std` cross-build (Cortex-M4F / Cortex-M33), Miri
  (undefined-behaviour detection), Kani reproduction, and `cargo-deny` audit.

### Evidence (Derived Claims)
The following values are computed algorithmically from the source code; they are
**not** runtime measurements on physical hardware.

| Claim | Value | Source |
|---|---|---|
| BCI pipeline utilisation bound | `U = 0.174` | `axonos-scheduler` unit tests, fixed-point derivation |
| Synchronous busy-period response-time bound | `R = 796 µs` | `axonos-scheduler` unit tests, implicit-deadline RTA |
| Full-catalogue information leak bound | `≤ 140.85 bits/s` | `axonos-capability` unit tests, analytic `Σ r·log₂\|payload\|` |

### Security
- `#![forbid(unsafe_code)]` in `axonos-scheduler`, `axonos-capability`,
  `axonos-time`, and `axonos-intent`.
- `axonos-spsc` contains two `unsafe` operations on the payload path, each with
  documented safety invariants and Kani BMC coverage.
- No heap allocation on any hot path. Static buffers and const-generic sizing
  only.

### Known Limitations
- **No hardware-in-the-loop validation.** Response-time and utilisation claims are
  algorithmically derived; they have not been validated by oscilloscope or GPIO
  instrumentation on the reference STM32H573 fixture. Phase 1 WCRT measurement is
  scheduled for Q2 2026.
- **No runnable kernel binary.** This release ships library crates only. The
  `axonos-core` integration crate (bare-metal scheduler loop, context switch,
  timer driver, MPU setup) is planned for the next release.
- **No MPU, stack, or context-switch code.** These are architecture-bound
  concerns deliberately excluded from the verifiable algorithmic core; they will
  reside in the integration layer.
- **No end-to-end HMAC verification.** `axonos-intent` carries an opaque truncated
  attestation tag; tag computation and verification are the responsibility of the
  downstream consumer.

### Notes
- The dependency graph between crates is acyclic: `axonos-intent` depends on
  `axonos-time` and `axonos-capability`; the remaining three crates are
  standalone.
- All crates are `#![no_std]`, dual-licensed under Apache-2.0 OR MIT, and
  publishable independently.

---

## Template (Unreleased)

### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
