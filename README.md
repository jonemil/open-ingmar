# Open Ingmar

![Open Ingmar pirate banner](assets/banner.png)

Open Ingmar is an independent Rust toolchain for Garmin Connect IQ: PRG
containers, VM instructions, assembly/disassembly, signing, barrel/IQ packaging,
and a restricted Monkey C compiler.

This is an independent project, unaffiliated with Garmin.

The compiler uses semantic IR and symbolic assembly to generate complete console
applications. Source classes, module/self calls, numeric operations, string
literals/printing, locals, conditions, and loops are supported within the
[documented source subset](docs/specs/frontend/README.md). General project/Jungle
builds, resource generation, and broader application profiles remain planned.
See [the plan](docs/plan.md) and [architecture](docs/architecture.md).

## Crates and tools

| Crate | Purpose |
|---|---|
| [ciq-compiler](crates/ciq-compiler/README.md) | Restricted Monkey C compiler, semantic IR, and application generation |
| [ciq-dis](crates/ciq-dis/README.md) | PRG inspection and editable section export |
| [ciq-asm](crates/ciq-asm/README.md) | PRG reconstruction, section assembly, signing, and verification |
| [ciq-package](crates/ciq-package/README.md) | Barrel/IQ reading, extraction, packing, and verification |
| [ciq-prg](crates/ciq-prg/README.md) | Library for PRG sections and structured metadata |
| [ciq-mc](crates/ciq-mc/README.md) | Library for VM instruction encoding/decoding |
| [ciq-assembly](crates/ciq-assembly/README.md) | Shared symbolic assembly model and text I/O |
| [ciq-sign](crates/ciq-sign/README.md) | Library for signatures and developer keys |

Typical commands from the workspace root (paths are placeholders):

```sh
cargo run -p ciq-compiler -- input.mc scratch/output.mcasm
cargo run -p ciq-dis -- all input.prg scratch/project
cargo run -p ciq-asm -- project scratch/project scratch/output.prg -y keys/developer_key.der
cargo run -p ciq-package -- list application.iq
```

Cargo builds each tool before running it. The examples below link to full
workflows; [architecture](docs/architecture.md) describes library
responsibilities and dependencies.

## Build and check

Rust 1.85 or newer is required. Keep `Cargo.lock` in version control for the CLI
tools. Follow the [validation policy](docs/validation.md) to choose checks:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Build API documentation with `cargo doc --workspace --no-deps`. See
[preview setup](scripts/README.md#preview-repository-documentation) for reading
the repository Markdown in a browser and checking its links.

## Compile an application

The [source class example](examples/compiler-class/README.md) generates a
complete unsigned console application from two source files. Sign it with a
local developer key and run it in the official simulator. The example documents
commands, expected output, and current lifecycle/View restrictions.

Readable IR and assembly provide inspection and editing points in the compiler
pipeline. See the [IR specification](docs/specs/ir/README.md#model-and-validation) for its semantic
model and saved text.

Browse [the examples](examples/README.md) for native calls, core Lang objects,
operators, arrays, and source classes. Each example owns its build commands and
expected behavior; the source and IR specs define supported contracts.

## Disassemble and assemble

Build the [Minimal Console reference](examples/minimal-console/README.md), then
export into a new directory and reassemble into a new file:

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/minimal-project
cargo run -p ciq-asm -- project scratch/minimal-project scratch/minimal-project.prg
```

The [section project](docs/specs/formats/README.md#section-project) contains an ordered
manifest, assembly for supported code/metadata, and raw payloads for unsupported
sections. Labels bind supported method, line, branch, and exception references;
general relocation and opaque-reference fixups are incomplete. Equal lengths
alone do not establish runtime validity.

Without `-y`, reconstruction preserves signing material. Supply your own key to
replace signatures after reconstruction:

```sh
cargo run -p ciq-asm -- project scratch/minimal-project scratch/minimal-signed.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/minimal-signed.prg -y keys/developer_key.der
```

Standalone `sign` and `verify` use the same [signing
profile](docs/specs/formats/developer-signatures.md). Inspection commands and
editable section forms are documented in the specs below.

## Packages and reference tools

`ciq-package` lists, extracts, verifies, and packs supported barrel/IQ profiles.
Extraction preserves ordinary member bytes, raw manifests, and unknown files,
with path and resource limits. See
[reading](docs/specs/formats/packages.md#shared-reading-and-extraction),
[barrel packing](docs/specs/formats/packages.md#barrel-packing),
[IQ repacking](docs/specs/formats/packages.md#iq-repacking), and
[own-key IQ creation](docs/specs/formats/packages.md#iq-creation).

Use [SDK support scripts](scripts/README.md) for device downloads, reference
builds, and simulator sessions. Simulator execution establishes runtime
compatibility separately from successful parsing, encoding, or verification. The
reference environment is SDK 9.2.0 and Forerunner 265 (`fr265`).

## Documentation

Start with [the specifications index](docs/specs/README.md) for the toolchain's
contracts and stage boundaries.

- [Plan](docs/plan.md) and [architecture](docs/architecture.md).
- [Monkey C frontend](docs/specs/frontend/README.md), including core Lang.
- [Semantic IR](docs/specs/ir/README.md), [lowering](docs/specs/lowering/README.md),
  [assembly](docs/specs/assembly/README.md), and [VM bytecode](docs/specs/vm/README.md).
- [SDK declarations/profiles](docs/specs/sdk/README.md),
  [builds](docs/specs/build/README.md), and [container formats](docs/specs/formats/README.md).
- [Examples](examples/README.md) and [support scripts](scripts/README.md).
- [Investigation index](docs/investigations/README.md) and
  [official Garmin reading map](docs/investigations/official-garmin-documentation.md).
- [Working instructions](AGENTS.md), [documentation conventions](docs/README.md),
  and [research conventions](docs/research.md).

## Local files and version control

Keep developer keys in ignored `keys/`; scripts default to
`keys/developer_key.der`. Download reference projects into ignored
`references/`. Keep SDK distributions, device data, credentials, caches,
generated binaries, and captures local; temporary experiment output belongs in
`scratch/`.

Version control includes our source, scripts, tests, documentation, container
recipe, and independently created example resources. Documentation retains
reproduction details without requiring local artifacts to exist. Review `git
status --short --untracked-files=all` and `git diff --cached` before committing;
ignore rules do not remove files already tracked by Git.

## License and contributions

Copyright 2026 Jon Emil Jahren and contributors. Our source, documentation, and
examples are licensed under either [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. Third-party dependencies and external
reference materials retain their own licenses.

See [contributing](CONTRIBUTING.md) for development checks, investigation
boundaries, and contribution licensing.
