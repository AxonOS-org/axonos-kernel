<!--
SPDX-License-Identifier: Apache-2.0 OR MIT
Copyright (c) 2026 Denis Yermakou <connect@axonos.org>
Part of the AxonOS project — https://github.com/AxonOS-org
-->

AxonOS Kernel — Dual Licence
============================

This workspace (axonos-kernel) is licensed under your choice of either:

  * Apache License, Version 2.0
    (see LICENSE-APACHE, or http://www.apache.org/licenses/LICENSE-2.0)

  * MIT License
    (see LICENSE-MIT, or http://opensource.org/licenses/MIT)

at your option. See NOTICE for required Apache-2.0 attribution.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
licence, shall be dual-licensed as above, without any additional terms or
conditions. This is the "inbound = outbound" model used by the Rust project.

GitHub's licence detector recognises this standard `LICENSE` filename; the
per-licence texts live in LICENSE-APACHE and LICENSE-MIT.


---

## Why this file is not called `LICENSE`

It was, and that had a cost nobody saw until a map made it visible.

GitHub detects a repository's licence by reading `LICENSE` and matching it
against known texts. This file is an explanation of dual licensing, not a
licence text, so the detector matched nothing and reported `NOASSERTION` —
which is what an audit tool, a package index, or a lawyer's checklist reads as
*unlicensed*.

The kernel has carried `LICENSE-APACHE` and `LICENSE-MIT` in full since the
beginning. Both were correct and neither was being read, because a file with
prose in it stood where the detector looks first.

The explanation is worth keeping and belongs under a name that is not
load-bearing. `LICENSE` is now a symlink-free copy of the MIT text, which is
what a detector expects to find and what a reader looking for terms wants
first.

<sub>© 2026 Denis Yermakou — The AxonOS Project · connect@axonos.org</sub>
