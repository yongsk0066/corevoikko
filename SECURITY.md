# Security Policy

## Supported Versions

Only the latest release is supported with security updates. This applies to all components: the Rust crates, the npm package (`@yongsk0066/voikko`), and the Finnish dictionary data (`voikko-fi`).

## Reporting a Vulnerability

If you find a security vulnerability in any part of this project, please file an issue at https://github.com/yongsk0066/corevoikko/issues.

Include as much detail as you can: affected component, steps to reproduce, and potential impact. You should expect an initial response within a few days.

## Dependency Auditing

The CI pipeline runs `cargo audit` on every push and pull request to detect known vulnerabilities in Rust dependencies. The npm package has no runtime JavaScript dependencies -- all logic runs in the WASM binary compiled from Rust.
