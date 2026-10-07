# Open Connect IQ toolchain

An independent Rust toolchain for Garmin Connect IQ PRG containers and VM bytecode, with a possible future Monkey C compiler and runtime.

The first compatibility milestone is to read a Garmin-generated trivial PRG, change a simple instruction or constant, write the PRG, and observe the changed behavior in Garmin's official simulator.

This repository currently contains planning and research documentation. No format claims have been verified here yet, and no Rust crates have been created.

- [Working instructions](AGENTS.md)
- [Plan and current status](docs/plan.md)
- [Architecture](docs/architecture.md)
- [Research and documentation conventions](docs/research.md)
- [Investigation index](docs/investigations/README.md)
- [Monkey C examples](examples/README.md)
- [SDK support scripts](scripts/README.md)
- [Original session handoff](connect-iq-rust-toolchain-codex-handoff.md)

The repository root is intended to become a Cargo workspace with members under `crates/`. Create the workspace manifest alongside the first working crate rather than adding placeholder implementations before researching the format.

Downloaded reference projects belong in the Git-ignored `references/` directory. Local SDK artifacts and temporary experiments belong in the Git-ignored `scratch/` directory. Documentation should retain enough information to reproduce experiments without relying on these directories being present.

Version control includes our source, scripts, tests, documentation, container recipe, and independently created example resources. SDK distributions, downloaded catalogues, device packages, reference checkouts, signing keys, credentials, caches, and generated binaries or captures remain local. Keep new downloads and experiment output in `scratch/` or `references/`; `.gitignore` also covers root download archives, `tmp/`, example build directories, and common signing-key and Connect IQ binary extensions. Review `git status --short --untracked-files=all` and `git diff --cached` before committing. Ignore rules do not remove files already tracked by Git.
