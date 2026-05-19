# ABOUT — AxonOS Kernels

## What this is

`axonos-kernels` is the verifiable substrate underneath a brain–computer
interface (BCI). It is a set of seven Rust crates that, composed
together, form the real-time scheduling, capability isolation, time
abstraction, IPC and wire-format substrate that a BCI signal pipeline
needs to make hard deadlines on a microcontroller.

The substrate is `#![no_std]`, has no heap, no allocator on the hot
path, no `unsafe` outside two documented operations in a single
ring-buffer crate, and ships with 28 Kani bounded-model-checking
harnesses verifying its safety-critical invariants.

This is not a complete operating system. It is the **kernel-side
foundation** on which a complete BCI operating system can be assembled.
Signal processing kernels (FIR, CSP, LDA, Riemannian classifier),
hardware-specific drivers, persistence, networking, and the
application-side SDK live in separate crates and repositories.

## For whom this is written

| Audience | What they will find here |
|:---|:---|
| **Embedded systems engineers** building real-time medical software | A `no_std`, allocator-free, Cortex-M-compatible scheduler with analytical schedulability and a formally verified IPC primitive. |
| **Neurotechnology researchers** writing closed-loop experiments | A well-typed RFC-compliant wire format (`axonos-intent`), capability-based application isolation, and a clear contract between the kernel and the application layer. |
| **Verification engineers and academic reviewers** | An open codebase with explicit Kani proof obligations, scoped unsafe surfaces, and a published falsification protocol (Phase 1, Q2 2026). |
| **Regulatory consultants and safety auditors** | A `#![forbid(unsafe_code)]` discipline across most crates, explicit assertion of evidence levels (RFC-0003), and a clear pathway toward IEC 62304 / ISO 14971 alignment. |
| **OEM partners** integrating BCI capability into existing devices | A reference firmware crate for STM32F407 and a documented contract for binding the kernel to alternative hardware via the `MonotonicClock` trait. |

## What problem it addresses

Closed-loop brain–computer interfaces operate on millisecond-scale
deadlines. A missed deadline is not a glitch — it is a control failure
in a system that is, by IEC 60601-1 essential-performance criteria, a
medical electrical device. The adjacent fields (cardiac pacemakers,
infusion pumps) meet this category with small, formally analysed kernels.
Most BCI software in 2026 does not.

`axonos-kernels` is the start of a substrate that does. The five
foundational crates separate the orthogonal concerns of a real-time
scheduler — time, data, scheduling, policy, ABI — so that each can be
audited, verified, and evolved in isolation. The integration crate
composes them; the firmware crate boots them on a reference Cortex-M4F
target.

## What it is not

- **It is not a general-purpose RTOS.** Tasks here are periodic with
  implicit deadlines; the API surface is intentionally narrow.
- **It is not a complete BCI operating system.** Signal-processing
  kernels, neural decoder, drivers, networking, and SDK live elsewhere.
- **It is not a medical device.** The codebase is a foundation on which
  a medical device might be built, subject to the appropriate regulatory
  process. No clinical claims attach to this repository as published.
- **It is not measurement-validated yet.** The published numbers
  (`U = 0.174`, `R = 796 µs`, information bound ≤ 140.85 bits/s) are
  derived in code. Hardware measurement is Phase-1 work, Q2 2026.

## Market context

Brain-computer interface deployment in 2026 splits across three
trajectories:

1. **Invasive, single-site research** (Neuralink, Synchron, Precision
   Neuroscience). Hundred-to-thousand-electrode arrays, custom silicon,
   bespoke software stacks per device. Each pilot study is its own
   software project.
2. **Semi-invasive surface ECoG** (NeuroXess, Ladder Medical, the
   Chinese ecosystem broadly). Lower channel count, surgical insertion,
   commercial deployment timeline accelerating.
3. **Non-invasive consumer/clinical EEG** (OpenBCI, Emotiv, Neurosity,
   numerous research platforms). Lower fidelity but accessible; the
   bulk of motor-imagery and SSVEP literature is here.

All three trajectories share the same software-substrate problem: each
project re-implements scheduling, IPC, capability isolation, and
wire-format handling, on a different RTOS, with a different unsafe
surface and a different proof obligation. There is no shared kernel
analogous to seL4 or Tock for the BCI domain.

`axonos-kernels` is an attempt to publish that shared substrate openly,
under dual Apache-2.0 / MIT licensing, with formal verification
obligations stated in code rather than in slideware. The repository's
audience is engineers who would prefer to compose their pipeline on a
foundation that someone else has audited and proved, rather than write
the same primitives from scratch one more time.

## Status, in plain terms

- **Code:** 3 603 lines of Rust source across 7 crates.
- **Tests:** 66 deterministic unit and integration tests passing on
  Linux, macOS, and Windows hosts.
- **Formal proofs:** 28 Kani bounded-model-checking harnesses.
- **Embedded builds:** `thumbv7em-none-eabihf` and `thumbv8m.main-none-eabihf`.
- **Hardware execution:** scaffolding firmware compiled; not yet
  executed on the reference fixture under CI.
- **Clinical deployment:** Phase 2 (Q3–Q4 2026), first 8-channel kit
  with the partner ALS rehabilitation centre.
- **Regulatory engagement:** Phase 3 (2027), FDA Pre-Submission.

## How to engage

The substrate is open source under dual Apache-2.0 / MIT licensing. The
preferred way to engage is to read the code, run the tests, file an
issue or RFC contribution against the relevant repository, and discuss
on the public AxonOS engineering channels.

Security disclosures: `security@axonos.org`.
Technical correspondence: `info@axonos.org`.
Partnership and clinical engagement: `connect@axonos.org`.

---

**Author:** Denis Yermakou · denis@axonos.org · [axonos.org](https://axonos.org)
