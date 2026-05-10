# Contributing to AxonOS

## Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run the full test suite:
   ```bash
   cargo test --lib
   cargo test --examples
   cargo clippy --all-features -- -D warnings
   cargo fmt --check
   ```
5. Commit with clear messages
6. Open a Pull Request

## Code Standards

- `#![forbid(unsafe_code)]` — no unsafe blocks without RFC approval
- Every public item must be documented
- Every quantitative claim must carry an evidence label [L1]/[L2]/[L3]/[pending]
- Unit tests for all public functions
- Kani proofs for safety-critical finite-state properties

## RFC Process

For substantial architectural decisions, follow the [RFC process](https://github.com/AxonOS-org/axonos-rfcs).

## Security

Please report security issues to security@axonos.org. Do not open public issues.
