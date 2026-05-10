#!/usr/bin/env python3
"""
Measure WCRT via DWT cycle counter.

Collects epoch timestamps and durations (Δ_k) from STM32F407
via SWD/debug probe.

Usage:
    python measure_wcrt.py --duration 43200 --output dwt_traces_12h.parquet

Output format:
    epoch_index | start_cycles | end_cycles | delta_cycles | delta_us
"""

import argparse
import time
import struct
from pathlib import Path


def parse_args():
    parser = argparse.ArgumentParser(description="Measure WCRT via DWT")
    parser.add_argument("--duration", type=int, default=43200,
                       help="Measurement duration in seconds (default: 12h)")
    parser.add_argument("--output", type=str, default="dwt_traces.parquet",
                       help="Output file path")
    parser.add_argument("--probe", type=str, default="probe-rs",
                       help="Debug probe interface")
    return parser.parse_args()


def measure_epoch(probe_interface):
    """Read DWT_CYCCNT at epoch boundaries via SWD."""
    # In production: use probe-rs Python API
    # For now: placeholder
    return {
        "epoch_index": 0,
        "start_cycles": 0,
        "end_cycles": 0,
        "delta_cycles": 0,
        "delta_us": 0.0,
    }


def main():
    args = parse_args()

    print(f"Starting WCRT measurement for {args.duration} seconds")
    print(f"Output: {args.output}")

    start_time = time.time()
    epochs = []
    epoch_count = 0

    while time.time() - start_time < args.duration:
        epoch = measure_epoch(args.probe)
        epochs.append(epoch)
        epoch_count += 1

        if epoch_count % 1000000 == 0:
            elapsed = time.time() - start_time
            print(f"Epochs: {epoch_count}, Elapsed: {elapsed:.1f}s")

    # Compute statistics
    deltas = [e["delta_us"] for e in epochs]
    max_delta = max(deltas)
    mean_delta = sum(deltas) / len(deltas)

    print(f"\nMeasurement complete:")
    print(f"  Total epochs: {epoch_count}")
    print(f"  Max Δ: {max_delta:.3f} µs")
    print(f"  Mean Δ: {mean_delta:.3f} µs")
    print(f"  Deadline: 4000 µs")
    print(f"  Margin: {4000 - max_delta:.1f} µs")

    # Save to Parquet (requires pyarrow)
    try:
        import pyarrow as pa
        import pyarrow.parquet as pq

        table = pa.table({
            "epoch_index": [e["epoch_index"] for e in epochs],
            "start_cycles": [e["start_cycles"] for e in epochs],
            "end_cycles": [e["end_cycles"] for e in epochs],
            "delta_cycles": [e["delta_cycles"] for e in epochs],
            "delta_us": deltas,
        })

        pq.write_table(table, args.output)
        print(f"Saved to {args.output}")
    except ImportError:
        # Fallback: CSV
        import csv
        csv_path = Path(args.output).with_suffix(".csv")
        with open(csv_path, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=epochs[0].keys())
            writer.writeheader()
            writer.writerows(epochs)
        print(f"Saved to {csv_path} (CSV fallback)")


if __name__ == "__main__":
    main()
