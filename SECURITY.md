# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅ Active development |

## Reporting a Vulnerability

Please report security vulnerabilities to **security@axonos.org**.

Do not open public issues for security reports.

We will respond within 48 hours and provide a timeline for fix and disclosure.

## Security Properties

- Memory safety via Rust's type system (no buffer overflows)
- Capability-based isolation (structural, not runtime)
- HMAC attestation for all intent observations
- Stimulation interlock with heartbeat monitoring
- `#![forbid(unsafe_code)]` across all modules except two targeted SPSC blocks
