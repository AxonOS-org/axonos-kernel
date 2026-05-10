# EDF Scheduler Specification

## Theorem 5.2 (Liu-Layland EDF)

A set of n periodic tasks with D_i = T_i is schedulable on a uniprocessor under EDF if and only if U ≤ 1.

## Task Model

Task τ_i = (C_i, T_i, D_i, φ_i):
- C_i: worst-case execution time [µs]
- T_i: period [µs]
- D_i: relative deadline [µs] (= T_i in AxonOS)
- φ_i: phase offset [µs]

## AxonOS Task Set (Table 4)

| Task | C_i (µs) | T_i (µs) | U_i = C_i/T_i |
|------|----------|----------|---------------|
| Signal pipeline (τ1) | 818 [L2] | 4000 | 0.2045 |
| Consent state machine (τ2) | 12 | 4000 | 0.0030 |
| HMAC attestation (τ3) | 18 | 4000 | 0.0045 |
| BLE intent egress (τ4) | 24 | 4000 | 0060 |
| Background diagnostics (τ5) | 100 | 1,000,000 | 0.0001 |
| **Nominal utilisation U** | | | **0.1737** |
| **L1-inflated U'** | | | **0.1748** |
| **L1-inflated U^cold** | | | **0.1794** |
| **L2-inflated U^L2** | | | **0.2181** |
| **Admission ceiling U_max** | | | **0.2500** |
| **Headroom U_max - U^L2** | | | **0.0319** |

## Timing Pessimism Sources

### Source 1: Flash Read Penalty
- STM32F407 has no data cache; ART accelerator covers instruction cache only
- Flash data reads at 168 MHz require 5 wait states
- t_flash ≤ 64 × 8 × 6 / 168MHz = 18.3 µs [L1]
- Applies only to first epoch after boot

### Source 2: Interrupt Entry Overhead
- Exception entry: 12 cycles (push 8 registers)
- Exception return: 12 cycles (pop 8 registers)
- Total: 24 cycles per interrupt round-trip
- t_IRQ ≤ 4 × 24 / 168MHz = 0.571 µs [L1]

### Source 3: DMA Bus-Matrix Arbitration
- ADS1299 SPI DMA transfers 6 32-bit words
- Worst-case 40-cycle AHB arbitration per word
- t_DMA ≤ 6 × 40 / 168MHz = 1.43 µs [L1]

### Source 4: AHB Pipeline Stall
- DMA2 holds AHB bus during SPI burst
- CPU's outstanding VLDR requests stalled
- t_AHB ≤ 8 × 48 / 168MHz = 2.29 µs [L1]

### Source 5: FPU Lazy-Stacking Overhead
- First FPU instruction after interrupt: 16-cycle penalty
- t_FPU ≤ 4 × 16 / 168MHz = 0.381 µs [L1]

### Source 6: L2-Inferred Unmodelled Overhead
- SRAM contention (M4F/A53 shared bank)
- Pipeline data hazards in matrix kernels
- AHB bus-matrix re-arbitration
- Gap: C_1^L2 - C_1^cold = 818 - 663.2 = 154.8 µs (23.3%) [L2]

## Synchronous Busy Period

L = Σ_j ceil(L / T_j) × C_j

Starting from L^(0) = Σ_j C_j^L2 = 972 µs.

Since L^(0) = 972 µs < min_j T_j = 4000 µs, all ceiling terms equal 1:
L^(1) = 1 × 818 + 1 × 12 + 1 × 18 + 1 × 24 + 1 × 100 = 972 µs = L^(0)

Response-time bound: R_1 ≤ L = 972 µs [L2]

## Deadline Slack

S_1 ≜ D_1 - R_1^L2 = 4000 - 972 = 3028 µs

Deadline utilisation ratio: ρ_1 = 972 / 4000 = 0.243

## EDF vs RMS Policy Justification

**Lemma 5.11**: EDF is appropriate because:
1. Theoretical optimality: any feasible task set schedulable under any policy is schedulable under EDF
2. Scaling margin: if future Stage 7 raises utilisation toward U_max = 0.25, only EDF guarantees schedulability
