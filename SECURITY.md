# Security Policy — axonos-kernel

This document is the vulnerability-disclosure policy for the
`axonos-kernel` workspace.

## Scope

This policy covers defects in the published source of any crate in
this workspace — `axonos-spsc`, `axonos-scheduler`, `axonos-capability`,
`axonos-time`, `axonos-intent`, `axonos-kernel-core`, and
`axonos-firmware-stm32f407` — that may have a security impact:
panic-on-malformed-input paths, out-of-bounds reads, unsound timing
assumptions an adversary could exploit, or capability-gate bypasses.

For specification-level concerns about the kernel design — the
scheduling discipline, the wire format, the capability catalogue —
open an issue or pull request against
[`axonos-rfcs`](https://github.com/AxonOS-org/axonos-rfcs) (RFC-0001
through RFC-0006). Concerns about the broader project should be raised
against [`axonos-standard`](https://github.com/AxonOS-org/axonos-standard).

## How to report

Report a suspected security problem by writing to
**security@axonos.org**. Describe the problem concretely: which crate,
which file or function, which Kani harness if applicable, what an
attacker could do, and where possible how to reproduce or demonstrate
it.

A reporter who prefers not to use email may instead open a private
security advisory through the GitHub security-advisory mechanism on
this repository. A reporter should **not** open an ordinary public
issue for a suspected security problem.

The two acknowledged `unsafe` blocks in `axonos-spsc` are explicitly
in scope. Any sound counter-example to their Kani-verified invariants
is a security finding.

## What to expect

The project acknowledges a security report within five business days.
The default coordinated-disclosure window is ninety days from
acknowledgement to public disclosure, shortened if a fix is ready
sooner and extended only by mutual agreement where remediation is
genuinely complex. The reporter is credited in the public disclosure
unless they ask to remain anonymous.

## Supported versions

Security remediations are issued for the current minor version of the
workspace, recorded in the root [`Cargo.toml`](./Cargo.toml). Older
minor versions, once superseded, do not receive remediations; a
deployment on a superseded version should plan its migration.

## What this policy does not cover

This policy does not cover security problems in third-party
implementations of the AxonOS architecture that are not this reference
kernel. If such an issue is caused by a defect in the underlying
specification, that specification defect is in scope and should be
reported here or against `axonos-rfcs` / `axonos-standard`.

This policy is not a warranty. The workspace is provided under the
dual Apache-2.0 / MIT licence (see [`NOTICE`](./NOTICE) and the
per-crate `LICENSE-APACHE` / `LICENSE-MIT` files) with the customary
disclaimers.

---

The AxonOS Project · https://axonos.org · security@axonos.org
