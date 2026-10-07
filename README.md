# Open Connect IQ toolchain

An independent Rust toolchain for Garmin Connect IQ PRG containers and VM bytecode, with a possible future Monkey C compiler and runtime.

The first compatibility milestone is to read a Garmin-generated trivial PRG, change a simple instruction or constant, write the PRG, and observe the changed behavior in Garmin's official simulator.

The Rust workspace now reads and writes PRG section streams and disassembles/reassembles the instruction subset observed in two reference examples. Both examples round-trip byte for byte. The original Hello World also has verified simulator behavior; running the newly emitted PRGs remains the next host-side check.

Build and validate the workspace:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Rust 1.85 or newer is required by the manifest (tested here with 1.99). There are no external Rust dependencies. See [the architecture](docs/architecture.md) for crate responsibilities and [the initial experiment](docs/investigations/rust-bootstrap.md) for assembly commands, evidence, and limits.

- [Working instructions](AGENTS.md)
- [Plan and current status](docs/plan.md)
- [Architecture](docs/architecture.md)
- [Research and documentation conventions](docs/research.md)
- [Investigation index](docs/investigations/README.md)
- [Monkey C examples](examples/README.md)
- [SDK support scripts](scripts/README.md)
- [Original session handoff](connect-iq-rust-toolchain-codex-handoff.md)

Workspace members under `crates/` are `ciq-prg`, `ciq-mc`, `ciq-dis`, and `ciq-asm`. Keep `Cargo.lock` in version control for the command-line tools.

Downloaded reference projects belong in the Git-ignored `references/` directory. Local SDK artifacts and temporary experiments belong in the Git-ignored `scratch/` directory. Documentation should retain enough information to reproduce experiments without relying on these directories being present.

Version control includes our source, scripts, tests, documentation, container recipe, and independently created example resources. SDK distributions, downloaded catalogues, device packages, reference checkouts, signing keys, credentials, caches, and generated binaries or captures remain local. Keep new downloads and experiment output in `scratch/` or `references/`; `.gitignore` also covers root download archives, `tmp/`, example build directories, and common signing-key and Connect IQ binary extensions. Review `git status --short --untracked-files=all` and `git diff --cached` before committing. Ignore rules do not remove files already tracked by Git.
