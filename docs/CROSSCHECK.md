<!--
SPDX-License-Identifier: Apache-2.0 OR MIT
SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
-->

# Cross-check: AxonOS against dy-wcet

Two implementations, written months apart for different reasons, analysing the
same task set. They agree — and the interesting part is what it took to make
the agreement mean something.

**Everything below can be recomputed.** The task set is in
[`crosscheck.json`](crosscheck.json) with every iteration of the recurrence
recorded, so a reader who doubts any number can follow the derivation without
running anything.

---

## The first attempt proved almost nothing

An earlier cross-check used the AxonOS pipeline as it stands: five tasks
totalling 796 µs against a shortest period of 4 000 µs. Both tools returned
796 µs and the utilisation matched at 17.41%.

That agreement was real and nearly worthless.

When the sum of all execution times is below the shortest period, **no task
activates twice inside any response window**. Every ceiling in the recurrence
is 1, the fixed point is reached on the first iteration, and what has been
verified is addition:

```text
642 + 12 + 18 + 24 + 100 = 796
```

None of the machinery that makes response-time analysis difficult — the
ceilings, the fixed point, the refusal on non-convergence — was reached. Two
calculators agreeing that a sum is a sum is not evidence about two analyses.

## The set that does exercise it

Same shape of system, periods spread the way they actually are on a device that
polls a sensor faster than it runs its pipeline.

| Priority | Task | C | T | D | B |
|--:|:--|--:|--:|--:|--:|
| 0 | `sensor_poll` | 120 µs | 1 000 µs | 1 000 µs | — |
| 1 | `signal_pipeline` | 642 µs | 4 000 µs | 4 000 µs | — |
| 2 | `consent_fsm` | 30 µs | 4 000 µs | 4 000 µs | 40 µs |
| 3 | `hmac_attestation` | 18 µs | 8 000 µs | 8 000 µs | 40 µs |
| 4 | `telemetry_tx` | 900 µs | 20 000 µs | 20 000 µs | — |

Execution times of 1 710 µs against a shortest period of 1 000 µs. The sum
exceeds the period, so tasks reactivate inside response windows and the
recurrence has work to do.

**Utilisation 335 250 ppm (33.53%).** Necessary, not sufficient.

## Where the arithmetic stops being addition

`telemetry_tx` takes three iterations, and the activation count changes between
them:

```text
R = 900    ⌈900/1000⌉=1 · ⌈900/4000⌉=1 · ⌈900/4000⌉=1 · ⌈900/8000⌉=1
           900 + 120 + 642 + 30 + 18                          →  1710

R = 1710   ⌈1710/1000⌉=2   ← sensor_poll activates a second time
           900 + 2×120 + 642 + 30 + 18                        →  1830

R = 1830   ⌈1830/1000⌉=2, the rest unchanged                   →  1830   ← fixed point
```

The second iteration is the whole point. At R = 1 710 the sensor poll has run
twice, not once, and an implementation that counted activations across the
task's **period** instead of its **response window** would have computed:

```text
900 + ⌈20000/1000⌉ × 120 = 900 + 2400 = 3300 µs
```

That is **3 300 µs against a true 1 830** — 1 470 µs of error. Here it is conservative and harmless. Set the
telemetry deadline to 2 000 µs and the same mistake reports a failure that does
not happen; invert the priorities and it reports a pass on a set that misses.

## Results

| Task | Response | Deadline | Iterations | |
|:--|--:|--:|--:|:--|
| `sensor_poll` | 120 µs | 1 000 µs | 1 | pass |
| `signal_pipeline` | 762 µs | 4 000 µs | 2 | pass |
| `consent_fsm` | 832 µs | 4 000 µs | 2 | pass |
| `hmac_attestation` | 850 µs | 8 000 µs | 2 | pass |
| `telemetry_tx` | **1 830 µs** | 20 000 µs | **3** | pass |

Schedulable under the stated execution times.

## What this does and does not establish

**Establishes.** Two independent implementations reach the same fixed points on
a set where the fixed point is not the first value tried. The ceilings, the
iteration and the convergence test all execute and agree.

**Does not establish** that either implementation is correct. Two
implementations sharing a misreading of the recurrence agree with each other
perfectly. That is why every number here carries its derivation: agreement is
checked against arithmetic, not against the other tool.

**Does not establish anything about hardware.** The execution times are inputs.
Where they came from is a separate question and a harder one, and no amount of
arithmetic on top of a wrong C makes the answer right.

**Does not compare like with like on scheduling.** AxonOS schedules EDF and
applies a processor-demand criterion for constrained deadlines; `dy-wcet` does
fixed-priority response-time analysis. On this set both reduce to the same
recurrence, which is why they can be compared at all — and it is a limit of the
comparison, not a property of the tools.

## Reproduce it

```sh
git clone https://github.com/DYResearch/dy-wcet && cd dy-wcet
cargo test --test on_paper
```

The set above is a fixture rather than a test, because it belongs to AxonOS
rather than to the crate. To run it:

```rust
use dy_wcet::{Task, TaskSet};

let mut s = TaskSet::new();
s.push(Task { wcet_us: 120, period_us:  1_000, deadline_us:  1_000, blocking_us:  0 })?;
s.push(Task { wcet_us: 642, period_us:  4_000, deadline_us:  4_000, blocking_us:  0 })?;
s.push(Task { wcet_us:  30, period_us:  4_000, deadline_us:  4_000, blocking_us: 40 })?;
s.push(Task { wcet_us:  18, period_us:  8_000, deadline_us:  8_000, blocking_us: 40 })?;
s.push(Task { wcet_us: 900, period_us: 20_000, deadline_us: 20_000, blocking_us:  0 })?;

assert_eq!(s.response_of(4), dy_wcet::Response::Bounded(1830));
```

---

<sub>Denis Yermakou · [connect@axonos.org](mailto:connect@axonos.org) ·
[DY Research](https://github.com/DYResearch) ·
[AxonOS](https://github.com/AxonOS-org) · Apache-2.0 OR MIT</sub>
