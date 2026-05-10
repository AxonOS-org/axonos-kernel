#!/usr/bin/env python3
"""
Extreme Value Theory fitting for pWCET estimation.

Peaks-over-Threshold (POT) analysis with Gumbel/GEV fitting.

Usage:
    python evt_gumbel_fit.py --input dwt_traces.parquet --threshold-p99

Output:
    - pWCET estimates at regulatory exceedance levels
    - QQ-plot for goodness-of-fit
    - KS statistic
"""

import argparse
import numpy as np
from scipy import stats
from scipy.optimize import minimize_scalar
import matplotlib.pyplot as plt


def parse_args():
    parser = argparse.ArgumentParser(description="EVT pWCET fitting")
    parser.add_argument("--input", type=str, required=True,
                       help="Input Parquet/CSV with DWT traces")
    parser.add_argument("--threshold-p99", action="store_true",
                       help="Use P99 as POT threshold")
    parser.add_argument("--threshold", type=float, default=4.8,
                       help="POT threshold in µs (default: 4.8)")
    parser.add_argument("--output", type=str, default="evt_results.png",
                       help="Output plot path")
    return parser.parse_args()


def load_data(path):
    """Load DWT traces from Parquet or CSV."""
    if path.endswith(".parquet"):
        import pyarrow.parquet as pq
        table = pq.read_table(path)
        return table.column("delta_us").to_pylist()
    else:
        import csv
        with open(path) as f:
            reader = csv.DictReader(f)
            return [float(row["delta_us"]) for row in reader]


def gumbel_fit(excesses):
    """Fit Gumbel distribution to POT excesses."""
    # Method of moments for initial estimate
    mean = np.mean(excesses)
    std = np.std(excesses)

    # Gumbel: μ = mean - γ·β, β = std·√6/π
    gamma = 0.5772156649  # Euler-Mascheroni
    beta_init = std * np.sqrt(6) / np.pi
    mu_init = mean - gamma * beta_init

    # MLE refinement
    def neg_log_lik(params):
        mu, beta = params
        if beta <= 0:
            return 1e10
        n = len(excesses)
        ll = -n * np.log(beta) - np.sum((excesses - mu) / beta) \
             - np.sum(np.exp(-(excesses - mu) / beta))
        return -ll

    from scipy.optimize import minimize
    result = minimize(neg_log_lik, [mu_init, beta_init],
                     method="L-BFGS-B",
                     bounds=[(None, None), (1e-6, None)])

    return result.x[0], result.x[1]


def pwcet_estimate(mu, beta, p):
    """pWCET at exceedance probability p."""
    # Gumbel quantile: x_p = μ - β·ln(-ln(p))
    return mu - beta * np.log(-np.log(p))


def ks_test(excesses, mu, beta):
    """Kolmogorov-Smirnov goodness-of-fit test."""
    # Transform to uniform via CDF
    uniform = np.exp(-np.exp(-(excesses - mu) / beta))
    d, p_value = stats.kstest(uniform, "uniform")
    return d, p_value


def main():
    args = parse_args()

    print("Loading DWT traces...")
    data = load_data(args.input)
    print(f"  Total samples: {len(data):,}")

    # POT threshold
    if args.threshold_p99:
        threshold = np.percentile(data, 99)
    else:
        threshold = args.threshold

    print(f"POT threshold: {threshold:.3f} µs")

    # Extract excesses
    excesses = np.array([x - threshold for x in data if x > threshold])
    print(f"Excesses: {len(excesses):,}")

    # Fit Gumbel
    print("\nFitting Gumbel distribution...")
    mu, beta = gumbel_fit(excesses)
    print(f"  μ = {mu:.3f} µs")
    print(f"  β = {beta:.3f} µs")

    # KS test
    d, p_value = ks_test(excesses, mu, beta)
    print(f"\nK-S test: D = {d:.4f}, p-value = {p_value:.4f}")

    if p_value < 0.05:
        print("WARNING: K-S test rejects Gumbel fit at α = 0.05")
    else:
        print("K-S test: Gumbel fit accepted")

    # pWCET estimates
    print("\npWCET estimates:")
    levels = [
        ("1 - 10^-4", 1 - 1e-4),
        ("1 - 10^-5", 1 - 1e-5),
        ("1 - 10^-6 (SIL-2)", 1 - 1e-6),
        ("1 - 10^-7 (SIL-3)", 1 - 1e-7),
    ]

    for name, p in levels:
        pwcet = threshold + pwcet_estimate(mu, beta, p)
        margin = 4000 / pwcet
        print(f"  {name:20s}: {pwcet:6.2f} µs  (margin: {margin:.0f}×)")

    # Consistency check
    n = len(data)
    expected_max = threshold + pwcet_estimate(mu, beta, 1 - 1/n)
    observed_max = max(data)
    print(f"\nConsistency check:")
    print(f"  Expected max (N={n:,}): {expected_max:.2f} µs")
    print(f"  Observed max:           {observed_max:.2f} µs")

    if observed_max < expected_max:
        print("  Tail is lighter than Gumbel (conservative)")

    # Plot
    fig, axes = plt.subplots(2, 2, figsize=(12, 10))

    # Histogram with fit
    ax = axes[0, 0]
    ax.hist(excesses, bins=50, density=True, alpha=0.7, label="Excesses")
    x = np.linspace(excesses.min(), excesses.max(), 200)
    from scipy.stats import gumbel_r
    ax.plot(x, gumbel_r.pdf(x, mu, beta), "r-", label="Gumbel fit")
    ax.set_xlabel("Excess [µs]")
    ax.set_ylabel("Density")
    ax.legend()
    ax.set_title("POT Excess Distribution")

    # QQ-plot
    ax = axes[0, 1]
    theoretical = gumbel_r.ppf(np.linspace(0.01, 0.99, len(excesses)), mu, beta)
    ax.scatter(np.sort(theoretical), np.sort(excesses), alpha=0.5)
    ax.plot([theoretical.min(), theoretical.max()],
            [theoretical.min(), theoretical.max()], "r--")
    ax.set_xlabel("Theoretical quantile [µs]")
    ax.set_ylabel("Observed quantile [µs]")
    ax.set_title("Q-Q Plot")

    # Full distribution
    ax = axes[1, 0]
    ax.hist(data, bins=100, density=True, alpha=0.7)
    ax.axvline(threshold, color="r", linestyle="--", label=f"POT threshold ({threshold:.1f} µs)")
    ax.set_xlabel("Δ [µs]")
    ax.set_ylabel("Density")
    ax.set_xlim(0, 20)
    ax.legend()
    ax.set_title("Full Jitter Distribution")

    # Tail zoom
    ax = axes[1, 1]
    tail = [x for x in data if x > threshold]
    ax.hist(tail, bins=50, density=True, alpha=0.7)
    ax.set_xlabel("Δ [µs]")
    ax.set_ylabel("Density")
    ax.set_title("Tail Distribution (POT)")

    plt.tight_layout()
    plt.savefig(args.output, dpi=150)
    print(f"\nPlot saved to {args.output}")


if __name__ == "__main__":
    main()
