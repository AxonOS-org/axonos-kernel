# Dual-Core Real-Time Contract (DC1-DC6)

## Partition Model

- **M4F** (hard real-time): signal pipeline under EDF, consent state machine, stimulation interlock
- **A53** (soft real-time): session management, BLE/Wi-Fi egress, WebAssembly sandbox
- **Shared SRAM**: 64-slot SPSC ring buffer (64 bytes/slot, 4096 bytes total)

## Contract Clauses (Table 6)

| ID | Guarantee | Bound | Level |
|----|-----------|-------|-------|
| DC1 | Pipeline meets deadline every cycle | — | [L2] |
| DC2 | SPSC IPC latency bounded | ≤ 0.2 µs | [L2] |
| DC3 | A53 wake-up deterministic | ≤ 50 µs | [L2] |
| DC4 | A53 state memory isolation | N/A | [L1] |
| DC5 | Safe-idle on M4F heartbeat loss | ≤ 12 ms | [L2] |
| DC6 | Intent attestation (HMAC-SHA256) | N/A | [L1] |

## IPC Latency Analysis (Theorem 7.1)

Minimum achievable SPSC IPC round-trip:

t_IPC,min = 1/f_store + 12/f_AHB+SRAM + 1/f_load = 14/168MHz = 83.3 ns = 0.083 µs [L1]

Measured: t_IPC = 0.200 µs [L2]

Excess: Δ_bus = 0.117 µs (+140%), attributable to A53-side bus-matrix re-arbitration.

## FMEA (Table 7)

| ID | Failure Mode | Effect | S | O | D | RPN |
|----|-------------|--------|---|---|---|-----|
| DC1 | Task overrun | Stale intent | 9 | 3 | 3 | 81 |
| DC2 | IPC latency violation | Stale data | 3 | 3 | 3 | 27 |
| DC3 | A53 wake-up failure | Intent lost | 3 | 3 | 9 | 81 |
| DC4 | Linker overlap | Stack corrupt | 9 | 3 | 9 | 243 |
| DC5 | M4F silent → stale stim | Continued neurostimulation | 9 | 3 | 9 | 243 |
| DC6 | HMAC failure | Unverified intent | 9 | 1 | 9 | 81 |

**Highest RPN: DC5 = 243** (revised from optimistic prior version)

Target: Kani verification to reduce O from 3 to 1, lowering RPN to 81.

## DC5 Safe-Idle

Measured A53 safe-idle transition on M4F halt injection: (11.3 ± 0.4) ms [L2]

Bound: ≤ 12 ms corresponds to 2.5 missed heartbeats plus A53 scheduler preemption latency.
