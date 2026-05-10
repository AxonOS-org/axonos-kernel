# Validation Taxonomy (RFC-0003)

## Evidence Levels

### L1: Instruction-Count Derived

c_L1 = Σ_i n_i · t_i^worst

where n_i is instruction count and t_i^worst is worst-case cycle count from ARM Cortex-M4 TRM.

Conservative; no hardware required.

### L2: Runtime Measured

c_L2 = max_k=1^N Δ_k

where Δ_k is DWT-cycle-counter-measured epoch duration and N = 10.8 × 10^6.

### L3: Independent Oscilloscope-Validated

c_L3 = max_k=1^M τ_k^GPIO

where τ_k^GPIO is duration measured by logic analyser at GPIO toggle points, with analyser's clock independent of DUT HCLK.

### Pending

A [pending] claim has no supporting evidence at publication time and must state:
- Target date
- Falsification criterion

## Measured Performance (Table 11)

| Metric | Value | Level |
|--------|-------|-------|
| Pipeline WCET C_1 (L1-nominal) | 640.2 µs | [L1] |
| L1-inflated WCET C^cold | 663.2 µs | [L1] |
| L2-inferred WCET C_1^L2 (binding) | 818 µs | [L2] |
| L1 busy-period (five-source) | 817.2 µs | [L1] |
| L2 WCRT (binding) | 972 µs | [L2] |
| Deadline misses | 0 observed | [L2] |
| EDF jitter mean Δ̄ | (0.300 ± 0.006) µs | [L2] |
| EDF jitter σ_Δ | 2.1 µs | [L2] |
| EDF jitter P99 | 4.8 µs | [L2] |
| EDF jitter P99.9 | 6.5 µs | [L2] |
| EDF jitter max | 9.2 µs | [L2] |
| SPSC IPC latency | 0.2 µs | [L2] |
| DC5 safe-idle (M4F halt) | (11.3 ± 0.4) ms | [L2] |
| GPIO-validated WCRT (H573) | [pending] Q2 2026 | |

## Poisson Deadline-Miss Upper Bound

Zero deadline misses over N = 10.8 × 10^6 epochs.

Garwood 95% upper confidence bound:
Pr(deadline miss per epoch) ≤ χ²_0.95,2 / (2N) = 2.996 / (2 × 10.8 × 10^6) = 1.39 × 10^-7

Consistent with IEC 61508 SIL-2 at epoch level.

## pWCET Indicative Analysis (Remark 4.2)

Peaks-over-Threshold Gumbel fit to jitter tail:

| Level p | pWCET (µs) | Deadline margin |
|---------|-----------|-----------------|
| 1 - 10^-4 | 8.20 | 488× |
| 1 - 10^-5 | 9.89 | 404× |
| 1 - 10^-6 (SIL-2) | 11.59 | 345× |
| 1 - 10^-7 (SIL-3) | 13.29 | 301× |

**Caveat**: Indicative only; formal pWCET requires K-S goodness-of-fit on raw DWT trace data.

## Phase 1 Validation Protocol (Q2 2026)

1. Instrument GPIO PA0 (epoch entry) and PA1 (pipeline complete)
2. Capture with Saleae Logic Pro 16 at 100 MS/s (10 ns resolution)
3. Run ≥ 1 hour (≥ 900,000 epochs)
4. Report maximum GPIO interval, P99.9, and σ
5. Retain raw waveform files with SHA-256 checksums

Result will be published regardless of outcome.
