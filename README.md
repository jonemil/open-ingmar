# Open Connect IQ toolchain

An independent Rust toolchain for Garmin Connect IQ PRG containers and VM bytecode, with a possible future Monkey C compiler and runtime.

The first compatibility milestone is to read a Garmin-generated trivial PRG, change a simple instruction or constant, write the PRG, and observe the changed behavior in Garmin's official simulator.

The Rust workspace now reads and writes PRG section streams and disassembles/reassembles the instruction subset observed in two reference examples. Both examples round-trip byte for byte. The modified, reassembled Hello World also runs in Garmin’s simulator: the own-key signed `41` → `42` edit is confirmed in both console output and the captured screen.

Export all sections and reassemble without the original PRG template (use a new output directory and filename):

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/minimal-project
cargo run -p ciq-asm -- project scratch/minimal-project scratch/minimal-project.prg
```

The export contains a manifest, assembly for supported ordinary code, structured symbol declarations, DATA strings and class/module declarations with opaque fallback spans, DEBUG strings and CODE-label-based line mappings, and raw binary payloads for the remaining sections. Losslessly representable symbol tables produce assembly input and an inspection listing; code and class fields share named symbols; DATA uses class aliases, typed class/string references, and shared CODE labels for methods, qualified by DATA owner when member names repeat. See [the project format](docs/specs/section-project.md). Project import enforces the declared payload lengths and resolves named method/line references; general relocation is not implemented. Without `-y`, signing material is preserved. With `-y`, the assembler replaces it with signatures from the supplied developer key. Keep exports under ignored `scratch/`.

Reassemble with your developer key, or sign an existing PRG separately (outputs must be new files):

```sh
cargo run -p ciq-asm -- project scratch/minimal-project scratch/minimal-signed.prg -y developer_key.der
cargo run -p ciq-asm -- sign examples/minimal-console/build/fr265/minimal-console.prg scratch/minimal-resigned.prg -y developer_key.der
cargo run -p ciq-asm -- verify scratch/minimal-signed.prg -y developer_key.der
```

See [the signing experiment](docs/investigations/developer-signing.md). Both SDK specimens verify; unchanged Minimal Console re-signs byte identically with our key. The signed `41` → `42` Hello World runs successfully on the host simulator; console output and the captured screen both show `42`. A [complete same-size function replacement](docs/investigations/function-replacement.md) that computes `40 + 2` through a local variable also runs successfully. A [larger appended function](docs/investigations/code-labels-and-relocation.md), selected through a shared code label, now prints and displays `43`. [Structured DEBUG and line metadata](docs/investigations/line-metadata.md) preserve both example PRGs byte for byte and support in-place grow/shrink experiments. Fresh host console and screen captures confirm the shrunk 5-byte function returning `45`; the grown variant has no separate retained runtime capture.

Inspect resolved symbols or annotated code (use `symbol-source` for symbol assembly or `code-symbolic` for self-contained named code and pinned strings):

```sh
cargo run -p ciq-dis -- symbols examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- code examples/minimal-console/build/fr265/minimal-console.prg
```

Inspect DATA classes, literal strings, and resource strings separately:

```sh
cargo run -p ciq-dis -- classes examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- strings examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- resource-strings examples/minimal-console/build/fr265/minimal-console.prg
```

`data-source` prints reassemblable DATA classes and strings with opaque fallback spans; `ciq-asm data <source.mcasm> <new-payload.bin>` serializes that section independently.

List or unpack source barrels and IQ distribution packages (destinations must be new):

```sh
cargo run -p ciq-package -- list examples/number-barrel/build/number-barrel.barrel
cargo run -p ciq-package -- unpack examples/number-barrel/build/number-barrel.barrel scratch/unpacked-library
cargo run -p ciq-package -- pack-barrel scratch/unpacked-library scratch/library-repacked.barrel
cargo run -p ciq-package -- list scratch/barrels/barrel-app.iq
cargo run -p ciq-package -- unpack scratch/barrels/barrel-app.iq scratch/unpacked-application
cargo run -p ciq-package -- repack-iq scratch/unpacked-application scratch/application-repacked.iq
cargo run -p ciq-package -- pack-iq scratch/unpacked-application scratch/application-resigned.iq -y developer_key.der
```

The reader preserves every ordinary member byte, including raw manifests and unknown files, and checks paths and resource limits before extraction. Both barrel specimens and all IQ members match independent extraction byte for byte. `ciq-package verify <archive> [--expected-key <public.der>]` now verifies the documented modern developer IQ profile. Deterministic `pack-barrel` now repacks a canonical extracted tree; both code and resource barrels are SDK-validated. See [packing behavior](docs/specs/barrel-packing.md). Verified [unchanged IQ repacking](docs/specs/iq-repacking.md) is also implemented; [own-key IQ creation](docs/specs/iq-creation.md) is also implemented, with external debug XML omitted by default. See [the supported reading profile](docs/specs/package-reading.md) and [verification evidence](docs/investigations/package-reading.md).

Build and validate the workspace:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Rust 1.85 or newer is required (workspace tests also pass with actual 1.85.0). Container and instruction libraries use only the standard library; the separate signing crate uses RustCrypto RSA and digest libraries, and `ciq-package` isolates ZIP/7z/XML dependencies. See [the architecture](docs/architecture.md) for crate responsibilities and [the initial experiment](docs/investigations/rust-bootstrap.md) for assembly commands, evidence, and limits.

- [Working instructions](AGENTS.md)
- [Plan and current status](docs/plan.md)
- [Architecture](docs/architecture.md)
- [Research and documentation conventions](docs/research.md)
- [Developer signing](docs/specs/developer-signatures.md)
- [Instruction reference](docs/specs/instructions.md)
- [Symbol assembly and named operands](docs/specs/symbol-assembly.md)
- [Shared CODE labels](docs/specs/code-labels.md)
- [DEBUG strings and line mappings](docs/specs/line-numbers-and-debug.md)
- [DATA class/module inspection](docs/specs/data-classes.md)
- [DATA string assembly](docs/specs/data-strings.md)
- [Resource string inspection](docs/specs/resources.md)
- [Barrel package profile](docs/specs/barrel-package.md)
- [IQ package/signing profile](docs/specs/iq-package.md)
- [IQ verification commands and reports](docs/specs/package-verification.md)
- [Package crate design and remaining stages](docs/design/package-crate.md)
- [First Monkey C compiler milestone](docs/design/first-compiler.md)
- [Future project builds and Jungle support](docs/design/project-builds.md)
- [Investigation index](docs/investigations/README.md)
- [Monkey C examples](examples/README.md)
- [SDK support scripts](scripts/README.md)
- [Original session handoff](connect-iq-rust-toolchain-codex-handoff.md)

[Multi-file and barrel examples](docs/investigations/multi-file-and-barrels.md) now round-trip and run: the multi-file assembly edit shows `47`, and reconstructed barrel/package programs show `46`. Barrel source/archive consumption produces identical CODE and DATA; the SDK creates distribution packages separately.

Workspace members under `crates/` are `ciq-prg`, `ciq-mc`, `ciq-sign`, `ciq-dis`, `ciq-asm`, and `ciq-package`. Keep `Cargo.lock` in version control for the command-line tools.

Downloaded reference projects belong in the Git-ignored `references/` directory. Local SDK artifacts and temporary experiments belong in the Git-ignored `scratch/` directory. Documentation should retain enough information to reproduce experiments without relying on these directories being present.

Version control includes our source, scripts, tests, documentation, container recipe, and independently created example resources. SDK distributions, downloaded catalogues, device packages, reference checkouts, signing keys, credentials, caches, and generated binaries or captures remain local. Keep new downloads and experiment output in `scratch/` or `references/`; `.gitignore` also covers root download archives, `tmp/`, example build directories, and common signing-key and Connect IQ binary extensions. Review `git status --short --untracked-files=all` and `git diff --cached` before committing. Ignore rules do not remove files already tracked by Git.
