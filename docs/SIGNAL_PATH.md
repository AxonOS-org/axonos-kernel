# Zero-Copy Signal Path

## Design

The path from ADS1299 ADC to classifier is a statically allocated SPSC ring buffer.
- No heap allocation
- No copy on hot path
- Capacity: 64 slots (power of 2)

## Pipeline Stages (Table 3)

| Stage | C_i (µs) | Derivation |
|-------|----------|------------|
| Kalman state estimator (8-ch) | 80.0 | 13,440 cycles / 168 MHz |
| FIR bandpass (order 64, 8-ch) | 320.0 | ≈40 µs/ch |
| Notch filter (50 Hz + 60 Hz) | 60.0 | 10,080 cycles / 168 MHz |
| Artifact rejection (±120 µV) | 40.0 | 6,720 cycles / 168 MHz |
| CSP spatial filter (8 × 8) | 100.0 | 16,800 cycles / 168 MHz |
| LDA classifier | 40.2 | 6,754 cycles / 168 MHz |
| **Pipeline subtotal** | **640.2** | incl. SPSC push overhead |

## FIR Filter Detail (Remark 5.5)

The FIR filter accounts for ≈50% of pipeline WCET.

- Order-64 FIR on 8 channels: 64 × 8 × 2 = 1024 MAC operations
- M4F SMLAD instruction (dual 16-bit MAC): halves to 512 instructions
- 1-cycle throughput: 512 / 168MHz ≈ 3.0 µs compute
- Plus coefficient-load overhead: ≈40 µs/channel
- Total: 8 × 40 = 320 µs

## SPSC Ring Buffer (Theorem 6.3)

### Slot States (Definition 6.1)

Slot i of capacity-N ring (N = 2^k):
- Free: seq_i = i
- Published: seq_i = i + 1
- Consumed: seq_i = i + N

### Memory Ordering Proof

Under Rust/C++11 memory model with Release-Acquire pairs:

1. W --sb--> S (program order, same thread)
2. S --sw--> L (Release-Acquire synchronizes-with)
3. L --sb--> R (program order, same thread)

By transitivity: W --hb--> R

### Unsafe Scope

Two targeted `unsafe` blocks in `ringbuf/spsc.rs`:
1. `core::ptr::write(slot, value)` — producer payload write
2. `core::ptr::read(slot)` — consumer payload read

All other kernel code is safe Rust.

### Kani Verification

Three bounded proofs in `kani_proofs/spsc_proof.rs`:
- K1: No data race (unwind: 8)
- K2: Wait-freedom (unwind: 4)
- K3: Memory ordering / payload integrity (unwind: 2)
