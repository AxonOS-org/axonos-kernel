# Contributing

We welcome external contributions and forks. The project is dual-licensed
Apache-2.0 OR MIT, which means you may use, modify, and redistribute this
code subject to the (light) attribution requirements described below.

## Fork in three clicks

```text
┌─────────────────────────────────────────────────────────────────────┐
│  1. Click "Fork" on the GitHub page                                 │
│     → https://github.com/AxonOS-org/axonos-kernels                  │
│                                                                     │
│  2. Clone your fork locally                                         │
│     $ git clone https://github.com/YOUR-USERNAME/axonos-kernels    │
│     $ cd axonos-kernels                                            │
│                                                                     │
│  3. Verify the workspace builds and tests pass                      │
│     $ cargo test --workspace                                       │
│     → 66 tests passing                                             │
└─────────────────────────────────────────────────────────────────────┘
```

That's it. You now have a working local copy. The same `cargo test`
output you see is the one CI will produce on every push. If anything
fails out of the box, please file an issue — that is itself a useful
contribution.

## After the fork: what you may do

Under Apache-2.0 OR MIT (your choice — these are dual licences), you may:

- **Read, study, and learn from** every line of code without restriction.
- **Modify** the source for your own purposes, public or private.
- **Redistribute** modified or unmodified copies, in source or binary form.
- **Commercialise** products built on top of this code, including
  closed-source proprietary products.
- **Sublicense** the code as part of a larger work, including under more
  restrictive terms (for the parts that are yours).
- **Patent** your improvements — the upstream patent grant under
  Apache-2.0 does not restrict your patenting of your own work.

## After the fork: what you must do (Apache-2.0)

If you redistribute the code (in source or binary form), Apache-2.0
imposes four small obligations:

1. **Keep the licence and copyright notices.** Every source file
   carries an SPDX line at the top:

   ```rust
   // SPDX-License-Identifier: Apache-2.0 OR MIT
   // Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
   // Part of the AxonOS project — https://github.com/AxonOS-org
   ```

   Preserve these headers in derivative works. Do not strip them.

2. **State changes.** If you modify a file, note in that file (or in a
   changelog distributed with it) that you have modified it. The exact
   form is your choice; a comment near the top is typical.

3. **Include the NOTICE file** if you distribute a derivative work.
   This file lives in the workspace root ([NOTICE](NOTICE)) and contains
   the upstream attribution. Apache-2.0 § 4(d) requires that this NOTICE
   accompany derivative distributions.

4. **Include the licence text** itself ([LICENSE-APACHE](LICENSE-APACHE)).

That is the entire Apache-2.0 compliance burden. If you choose the MIT
licence instead (your option under our dual licensing), the burden is
even smaller: include the [LICENSE-MIT](LICENSE-MIT) text.

## What you may NOT do

- **Use the "AxonOS" name to identify modified or derivative software**
  in a way that could imply endorsement by, or affiliation with, the
  original project. Trademark policy is described in
  [NOTICE](NOTICE). You may state "based on AxonOS" or "derived from
  AxonOS" as factual descriptors. You may not call your fork "AxonOS
  Pro" or similar.

- **Misrepresent the upstream.** Apache-2.0 § 6 prohibits using the
  contributors' names to endorse or promote products derived from this
  work without specific prior written permission.

- **Strip authorship attribution from individual files.** SPDX headers
  and copyright lines must remain.

## Contributing back upstream

If you have improvements, fixes, or new RFC proposals you would like
considered for the upstream project:

1. **For small fixes** (typos, build issues, individual lint
   resolutions): open a pull request on the relevant repository.

2. **For substantive design changes** (new crate, new architectural
   pattern, breaking API change): open an issue first to discuss the
   approach, or — for changes that affect the wire format or the
   capability surface — submit an RFC against
   [`axonos-rfcs`](https://github.com/AxonOS-org/axonos-rfcs) following
   the template in that repository.

3. **For security issues**: please do **not** open a public issue. Email
   `security@axonos.org` with a description and (if possible) a
   reproducer. We acknowledge within 1 business day for verifiable
   disclosures and follow the standard 90-day coordinated disclosure
   timeline.

### Contributor licence

By submitting a pull request, you agree that your contribution will be
dual-licensed under Apache-2.0 OR MIT, on the same terms as the rest of
the project. We do not require a separate CLA; this is the standard
"inbound = outbound" model used by the Rust project itself.

If your contribution requires alternative licensing for legal or
employer-policy reasons, please raise this in advance via the issue
tracker and we will discuss.

## Code style

- `cargo fmt --all` before every commit.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- `cargo test --workspace` must pass.
- New unsafe blocks require a Kani harness verifying their safety.
- New public APIs require rustdoc with at least one usage example.
- New error types should be exhaustive enums, not `Box<dyn Error>`.

Run the full local CI mirror before submitting:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --all-features
```

If you have access to a Cortex-M target:

```bash
cargo build --workspace --release --target thumbv7em-none-eabihf
```

## Communication

- **General discussion:** `info@axonos.org`
- **Architectural decisions:** open an issue or RFC
- **Security:** `security@axonos.org`
- **Partnerships:** `connect@axonos.org`

---

**Maintainer:** Denis Yermakou · [denis@axonos.org](mailto:denis@axonos.org)

By contributing you affirm you have the right to submit the work under
the project's dual licence.
