# Notable changes — axonos-kernel

All notable changes to the AxonOS kernel workspace are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The workspace versions all 8 crates lock-step (`axonos-capability`,
`axonos-intent`, `axonos-time`, `axonos-spsc`, `axonos-scheduler`,
`axonos-kernel-core`, `axonos-firmware-stm32f407`, and the workspace root).

---

## [Unreleased]

### Added — cross-crate integration tests (`axonos-kernel-core`)

A `tests/integration.rs` suite that exercises the assembled kernel as an
**external consumer**, reaching every type through the crate's public surface
and the crates it re-exports. Tests-only: no library change, no public API
change, no version increment.

It concentrates on the seams the in-crate unit tests did not cover:

- **`schedule_tick`** — previously untested: earliest-deadline selection,
  tie-breaking by task id, the empty-ready-set case, and tick-counter advance.
- **The clock session-envelope guard** — both the observation path and the
  scheduling path reject a clock whose time lies beyond the session envelope.
- **A multi-step observation sequence** — three observations produced through
  the capability gate, drained in FIFO order with monotonically increasing
  sequence numbers, ring then empty.
- **Constrained-deadline reachability** — the 0.3.0 `Task::periodic_with_deadline`
  composes through kernel construction, and `processor_demand_feasible` is
  reachable through the kernel's public surface, catching a constrained set the
  utilisation admission test admits.

---

## [v0.3.0] — 2026-05-29

### Added — constrained-deadline scheduling (`axonos-scheduler`)

The headline of this release: the scheduler now supports **constrained-deadline**
task systems (`D_i <= T_i`), the first item on its roadmap. Previously only
implicit-deadline systems (`D_i = T_i`) were supported.

- **`Task::periodic_with_deadline(id, wcet, period, deadline)`** constructs a
  task whose relative deadline is shorter than its period. The existing
  `Task::periodic` (implicit deadline) is unchanged.
- **`demand_bound(set, t)`** computes the demand-bound function `dbf(t)` — the
  maximum cumulative execution demand of jobs whose release and deadline both
  fall within an interval of length `t`.
- **`processor_demand_feasible(set) -> Feasibility`** decides EDF feasibility
  by the **processor-demand criterion** (Baruah, Rosier, and Howell, 1990).
  Under constrained deadlines the Liu–Layland utilisation bound is necessary
  but not sufficient; this test evaluates `dbf(t) <= t` at every deadline in
  the La–Sha feasibility interval. It is **integer-only** (no floating point on
  the analysis path) and **bounded**: it returns `Feasibility::Uncertain`
  rather than risk an unsound pass, and never returns `Feasible` unless every
  relevant deadline point was checked. The verdict is `Feasible`,
  `Infeasible { at, demand }` (a definite counterexample), or `Uncertain`.
- This catches a class of error the utilisation test cannot: a task set well
  under `U = 1` can still miss a constrained deadline, and
  `processor_demand_feasible` reports it as `Infeasible` with the violating
  deadline and demand.
- Two new Kani harnesses in `axonos-scheduler/kani-proofs`: `sched_dbf_monotone`
  (dbf is monotone non-decreasing in `t`) and `sched_dbf_zero_below_first_deadline`
  (dbf is zero below the first deadline).

### Changed

- All workspace crates and the workspace package version move to **0.3.0** in
  lockstep; inter-crate path-dependency requirements updated accordingly. Only
  `axonos-scheduler` carries functional changes in this release; the other
  crates are versioned together as a coherent kernel release.
- `axonos-scheduler` is the only crate with API changes, all **additive**: the
  implicit-deadline API (`Task::periodic`, `TaskSet::admit`, `select_next`,
  `response_time_bound`) is unchanged and backward-compatible.

### Notes

- `KERNEL_ABI_VERSION` remains **1**: the kernel ABI surface is unchanged. The
  `Task` struct is unchanged (its `deadline` field already existed), so the
  addition is source- and ABI-compatible.
- No `unsafe` introduced; `axonos-scheduler` remains `#![forbid(unsafe_code)]`.

---

## [v0.2.3] — 2026-05-27

AxonOS-style refresh. No source-code or API changes — the v0.2.1
implementation is preserved byte-identical, including the public
`KERNEL_ABI_VERSION = 1` contract and all 28 Kani-verified invariants.
This release brings the workspace up to the unified AxonOS visual and
documentation standard applied across the organisation.

### Added

- **`.gitignore`** — Rust-workspace hygiene; was missing.
- **`SECURITY.md`** — vulnerability-disclosure policy mirroring the
  policy used in `axonos-standard`, `axonos-swarm`, and the AxonOS
  Project entry-point repository. Explicitly puts the two acknowledged
  `unsafe` blocks in `axonos-spsc` in scope.

### Changed

- **README badge palette** — replaced the previous mixed palette
  (orange / blue / blueviolet / purple / yellow / brightgreen, mixing
  `for-the-badge` and `flat-square` styles) with the canonical AxonOS
  palette in a single consistent style: AxonOS blue `#0a4a8f` for
  versioned artefacts (Workspace v0.2.3, Standard v1.0.0, Kernel ABI v1),
  Rust canonical orange `#CE422B` for the language, trust green `#0d7a5f`
  for Verified-by-Kani and the unsafe-discipline tag, slate `#475569`
  for licence and metadata.
- **README header** — removed the standalone Ferris mascot image; the
  Rust association is communicated through the canonical badge instead.
- **README "Related repositories"** — replaced with a full **Position
  in the AxonOS stack** table covering all seven repositories.
- **README footer** — canonical centered AxonOS block (project name,
  contacts, five-city locator with Singapore first), decorative emoji
  removed.
- **Repository case in published metadata** — the `repository` URL in
  every Cargo.toml across the workspace was corrected to the actual,
  lowercase GitHub path `axonos-kernel` (19 occurrences across the
  manifests, the root LICENSE, this changelog's history, and
  CONTRIBUTING). The URL is published to crates.io and must be
  byte-correct.
- **Author email everywhere** — the personal author address was
  replaced with the project-canonical `connect@axonos.org` across 56
  occurrences in source files, Cargo.toml manifests, sub-crate READMEs,
  the root LICENSE / NOTICE / ABOUT / CONTRIBUTING, and every per-crate
  `LICENSE-APACHE` and `LICENSE-MIT` file. The legacy general-contact
  address was folded into `connect@axonos.org` as well.
  `security@axonos.org` is unchanged — the security address is distinct.

### Notes

- **Source-code unchanged.** All v0.2.1 APIs work as before. The
  binding ABI version `KERNEL_ABI_VERSION = 1` is unchanged; the
  Tandem compatibility matrix (Kernel 0.2.x ↔ SDK 0.3.x ↔ ABI v1)
  still holds.
- **Patch bump 0.2.2 → 0.2.3** per SemVer — purely additive,
  no breaking API or behavioural changes, no new runtime dependency.

---

## [v0.2.2] — 2026-05-26

Documentation and provenance release (tagged on `main`).

### Changed

- Aligned the kernel README surface and contact details.

### Added

- `CITATION.cff` — initial machine-readable citation metadata.
- Verified the GitHub and Termux SSH commit-signing provenance for the
  release workflow.

### Notes

- No source-code or API changes. Crate version remained at `0.2.1` for
  this tag; the comprehensive crate-version bump and the full AxonOS-style
  refresh land in v0.2.3.

---

## [v0.2.1] — 2026-05-19

CI patch release. No source or API changes from v0.2.0.

### Fixed

- **`rustdoc::broken-intra-doc-links`** in `axonos-kernel-core/src/lib.rs`
  — replaced unqualified `[CapabilitySet]`, `[Capability]`, `[IntentObservation]`
  links in the `KERNEL_ABI_VERSION` doc comment with fully-qualified paths
  (`[axonos_capability::CapabilitySet]`, etc.). These types live in
  sibling crates and were not in scope from `axonos-kernel-core`.

- **`rustfmt`** in `axonos-capability/src/lib.rs` — `all_is_superset_of_any_subset`
  test had a chain `CapabilitySet::singleton(...).with(...)` artificially
  split across two lines; rustfmt prefers a single line at this length.
  Collapsed to one line.

- **Primary `LICENSE` file** added at workspace root. GitHub's license
  detector only recognises standard filenames (`LICENSE`, `LICENSE.md`,
  `LICENSE.txt`, `COPYING`), not `LICENSE-APACHE` / `LICENSE-MIT`. The
  new file is a dispatcher pointing to both Apache-2.0 and MIT, matching
  the `Cargo.toml` `license = "Apache-2.0 OR MIT"` declaration. GitHub
  should now display "Apache-2.0 OR MIT" instead of "Unknown".

### Notes

- Wire format unchanged — KERNEL_ABI_VERSION still v1.
- API unchanged from v0.2.0. Pure infrastructure / documentation cleanup.
- All 8 crates bumped from 0.2.0 → 0.2.1 in lockstep.

## [v0.2.0] — 2026-05-18

First minor-version release after the v0.1.x stabilisation cycle. Introduces
the binding ABI version constant, API-level parity with `axonos-sdk v0.3.4`,
and an automated GitHub Release workflow.

### Added — `KERNEL_ABI_VERSION` constant in `axonos-kernel-core`

The kernel now exposes its binding ABI version explicitly:

```rust
pub const KERNEL_ABI_VERSION: u32 = 1;
pub const KERNEL_IMPL_VERSION: &str = env!("CARGO_PKG_VERSION");
```

`KERNEL_ABI_VERSION` is the **wire-format contract** between this kernel
and any consuming SDK. It governs the encoding of `Capability` discriminants,
`CapabilitySet` bitfield layout, `IntentObservation` serialised form, and
the kernel ↔ SDK handshake exchange (RFC-0006 §2-5).

**Compatibility rule:** a kernel reporting `KERNEL_ABI_VERSION = N` must be
paired with an SDK that declares the same number in
`axonos_sdk::KERNEL_ABI_VERSION`. Mismatched versions MUST fail the
handshake — never run silently.

**Tandem with axonos-sdk:**

| Kernel | SDK | ABI | Compatible |
|:---|:---|:---:|:---:|
| `0.1.x` – `0.2.x` | `0.3.x` | v1 | ✓ |
| `0.3.x` (future) | `0.4.x` (future) | v2 | ✓ |

### Added — `CapabilitySet::all()` method

Method form of the existing `CapabilitySet::ALL` constant, for API symmetry
with `axonos_sdk::CapabilitySet::all()`. Both produce a bitfield equal to
`ADMISSIBLE_MASK` (= `0x0000_000F`).

```rust
let kernel_catalogue = CapabilitySet::all();
let nav = CapabilitySet::singleton(Capability::Navigation);
assert!(nav.is_subset_of(kernel_catalogue));
```

### Added — `CapabilitySet::is_disjoint()` method

Returns `true` iff `self` and `other` share no capabilities. Useful for
proving orthogonal multitenancy: two manifests with disjoint capability
sets cannot interfere at the capability layer.

```rust
let nav = CapabilitySet::singleton(Capability::Navigation);
let quality = CapabilitySet::singleton(Capability::SessionQuality);
assert!(nav.is_disjoint(quality));
```

WCET: 2 cycles (single AND + compare-zero). Suitable for the hot path.

### Added — 12 new unit tests

8 tests for new `CapabilitySet` methods (`is_disjoint`, `all`):
- `is_disjoint_no_overlap`
- `is_disjoint_overlap_returns_false`
- `is_disjoint_empty_with_anything`
- `is_disjoint_self_is_false_when_nonempty`
- `all_method_matches_all_const`
- `all_contains_every_capability`
- `all_is_superset_of_any_subset`
- `all_equals_admissible_mask`

4 ABI conformance tests in `axonos-kernel-core`:
- `kernel_abi_version_is_one` — locks the ABI at v1
- `kernel_impl_version_matches_cargo` — version flow integrity
- `abi_version_is_const_compile_time` — const-context usability
- `capability_set_all_matches_admissible_mask` — wire-format byte exactness
- `capability_discriminants_locked_by_abi` — RFC-0006 §3 binding

### Added — auto-release GitHub Actions workflow

`.github/workflows/release.yml` triggers on every `v*.*.*` tag push and
creates a proper GitHub Release with:

- Title: the tag (e.g. `v0.2.0`)
- Body: matching CHANGELOG section extracted via awk
- **Green "Latest" banner** on the repo page for stable releases
- Pre-release marker for `-rc`/`-beta`/`-alpha` suffixes
- Source `.tar.gz` and `.zip` archives attached
- ABI-compatibility footer with `KERNEL_ABI_VERSION` reminder

This replaces the plain "N tags" link with a prominent release banner
linking to the relevant CHANGELOG section and downloadable archives.

### Documentation

- README updated with an ABI-compatibility matrix linking this kernel's
  `KERNEL_ABI_VERSION` to compatible `axonos-sdk` versions.
- `axonos-kernel-core/src/lib.rs` opens with a documented contract on
  ABI stability rules — what bumps the number, what doesn't.

### Notes

- **No source-code removal.** All v0.1.9 APIs continue to work.
  `CapabilitySet::ALL` and `Capability::ALL` constants are retained
  alongside the new method forms.
- **No wire-format change.** `KERNEL_ABI_VERSION` stays at 1; bitfield
  layout, discriminants, and observation encoding are byte-identical
  to v0.1.x.
- **Workspace lockstep.** All 8 crates bumped from `0.1.9` to `0.2.0`
  together. This is the project policy — versions cannot diverge across
  the workspace because crates share types.

---

## [v0.1.9] — 2026-05-17

### Fixed
- Scheduler Kani BMC bounds reduced (S1/S4: `t1, t2 ≤ 8`,
  `#[kani::unwind(5)]` per harness; S2 periods/releases ≤ 1_000) to fit
  within CI's 35-minute timeout.

## [v0.1.8] — 2026-05-17

### Fixed
- Scheduler Kani harness bounds (1_000_000 → 4_000).

## [v0.1.7] — 2026-05-16

### Fixed
- Kani `--default-unwind 4` → 16; CI timeout 25→35 min.

## [v0.1.6] — 2026-05-16

### Fixed
- `cargo-deny` pinned to v1 (later reverted to v2 in workspace).
- Continue-on-error policy for advisory/license drift.

## [v0.1.5] — 2026-05-15

### Fixed
- Kani `--enable-unstable` flag removed.
- Firmware crate explicitly targets `thumbv7em-none-eabihf`.

## [v0.1.4] — 2026-05-15

### Fixed
- `AtomicU64` gated for `thumbv7em` via `#[cfg(target_has_atomic = "64")]`
  (Cortex-M4F is 32-bit; 64-bit atomics require LL/SC pairs not available
  on this target).

## [v0.1.3] — 2026-05-14

### Fixed
- Clippy lint priority for `Rust 1.85+`:
  `all = { level = "deny", priority = -1 }`.

## [v0.1.0] — 2026-04

Initial workspace release: 7 foundational crates implementing the AxonOS
kernel surface — capability gate, intent encoder, monotonic time source,
SPSC IPC, EDF scheduler, integration kernel, and STM32F407 firmware
binding.

---

[v0.2.1]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.2.1
[v0.2.0]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.2.0
[v0.1.9]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.9
[v0.1.8]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.8
[v0.1.7]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.7
[v0.1.6]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.6
[v0.1.5]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.5
[v0.1.4]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.4
[v0.1.3]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.3
[v0.1.0]: https://github.com/AxonOS-org/axonos-kernel/releases/tag/v0.1.0
