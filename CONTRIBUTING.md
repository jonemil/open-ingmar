# Contributing

Read the [README](README.md), [plan](docs/plan.md), and
[architecture](docs/architecture.md) before substantial changes. Keep changes
focused on the planned work and include relevant behavior checks. Use
[documentation conventions](docs/README.md) for prose and Rustdoc, and
[research conventions](docs/research.md) for evidence and attribution.

## Development

Rust 1.85 or newer is required. The ordinary Rust checks need no Garmin SDK,
device downloads, or developer key:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Use [the Markdown preview helper](scripts/README.md#preview-repository-documentation)
to check documentation links. GitHub CI runs these checks on stable Rust and
checks compilation with the declared minimum Rust version.

Changes involving VM behavior also need the applicable
[toolchain validation](docs/plan.md#toolchain-completion) and simulator evidence.
See [SDK support scripts](scripts/README.md) for local setup; CI does not perform
SDK builds or simulator sessions.

## Sources and reports

Implement independently from our documented formats, public contracts, and
programs compiled from our own sources. Do not disassemble or decompile Garmin's
compiler or SDK. Keep keys, credentials, downloaded reference projects, SDK and
device files, compiled specimens, and simulator captures outside Git.

For bug reports, include a minimal independently authored source example, the
command, expected/actual behavior, and relevant tool/API/device versions.
Remove credentials and private paths from logs. Document reproducible binary
observations without attaching proprietary artifacts.

## License

Unless explicitly stated otherwise, contributions intentionally submitted for
inclusion in this project are licensed under both [MIT](LICENSE-MIT) and
[Apache-2.0](LICENSE-APACHE), allowing recipients to choose either license.
Retain attribution and identify any separately licensed material.
