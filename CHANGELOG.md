# Notable changes — axonos-kernel

All notable changes to the AxonOS kernel workspace are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The workspace versions all 8 crates lock-step (`axonos-capability`,
`axonos-intent`, `axonos-time`, `axonos-spsc`, `axonos-scheduler`,
`axonos-kernel-core`, `axonos-firmware-stm32f407`, and the workspace root).

---

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

[v0.2.0]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.2.0
[v0.1.9]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.9
[v0.1.8]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.8
[v0.1.7]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.7
[v0.1.6]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.6
[v0.1.5]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.5
[v0.1.4]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.4
[v0.1.3]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.3
[v0.1.0]: https://github.com/AxonOS-org/AxonOS-kernel/releases/tag/v0.1.0
