# Open Ingmar

![Open Ingmar pirate banner](assets/banner.png)

Open Ingmar is an independent Rust toolchain for Garmin Connect IQ: PRG
containers, VM instructions, assembly/disassembly, signing, barrel/IQ packaging,
and a restricted Monkey C compiler.

This is an independent project, unaffiliated with Garmin.

The compiler uses semantic IR and symbolic assembly to generate complete console
applications. Source classes, module/self calls, integer arithmetic, string
literals/printing, locals, conditions, and loops are supported within the
[documented source subset](docs/specs/compiler-subset.md). General project/Jungle
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
workflows; [architecture](docs/architecture.md)
describes library responsibilities and dependencies.

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
pipeline. See [compiler design](docs/design/compiler-ir.md) for the stages and
[IR syntax](docs/specs/compiler-ir-text.md) for saved text.

The [native call example](examples/compiler-native-calls/README.md) exercises
inherited Number-to-String conversion on locals and parameters.

The [typed Lang example](examples/compiler-lang-values/README.md) passes String
and Boolean values through module and instance methods.

The [nullable Number example](examples/compiler-nullable-numbers/README.md)
checks native search/conversion results before using them as Numbers.

The [substring example](examples/compiler-substring/README.md) exercises nullable
String results, guarded returns, and nullable/negative indices.

The [Number method example](examples/compiler-number-methods/README.md) groups
conversion, comparison, equality, and hashing checks.

The [String method example](examples/compiler-string-methods/README.md) groups
text operations, base conversion, comparison, equality, and hashing.

The [Boolean method example](examples/compiler-boolean-methods/README.md) groups
comparison, equality, hashing, and string conversion.

The [immutable weak example](examples/compiler-immutable-weak/README.md) checks
receiver-specific results on Number, String, and Boolean.

The [Char method example](examples/compiler-char-methods/README.md) groups
character conversion, comparison, equality, and hashing.

The [Char literal example](examples/compiler-char-literals/README.md) exercises
BMP literals and escapes through source, IR, and native calls.

The [Char operator example](examples/compiler-char-operators/README.md) exercises
numeric comparisons, mixed addition, and Char concatenation.

The [String operator example](examples/compiler-string-operators/README.md)
exercises scalar concatenation, nullable inputs, and identity equality.

The [Number literal example](examples/compiler-number-literals/README.md)
exercises signed 32-bit constants and immediate-width boundaries.

The [Number-array literal example](examples/compiler-number-array-literals/README.md)
checks inferred elements, typed calls, and evaluation order.

The [empty-array example](examples/compiler-empty-arrays/README.md) checks
contextual construction in returns, arguments, and local assignments.

The [native View example](examples/compiler-native-views/README.md) checks
object construction, typed references, and View-array operations.

The [initial View example](examples/compiler-initial-view/README.md) supplies
the source lifecycle callback with a validated one-view positional result.

The [visible View example](examples/compiler-visible-view/README.md) constructs
a source View and draws text through its typed onUpdate callback.

The [field counter example](examples/compiler-fields/README.md) checks Number
field initialization, independent instances, and state across drawing callbacks.

The [object-call example](examples/compiler-object-calls/README.md) calls source
methods through copied and reassigned references to constructed Views.

The [source signature example](examples/compiler-source-signatures/README.md)
preserves exact View class contracts through parameters, factories, and returns.

The [retained View example](examples/compiler-reference-fields/README.md) uses
constructor arguments and reference fields across lifecycle callbacks.

The [input counter](examples/compiler-input-counter/README.md) returns a
View/delegate pair and redraws retained state after selection callbacks.

The [Number comparison example](examples/compiler-number-comparisons/README.md)
checks ordering, inequality, operand order, and null guards.

The [Number products example](examples/compiler-number-products/README.md) checks
multiplication, division, remainder, precedence, and operand effects.

The [Number negation example](examples/compiler-number-negation/README.md)
exercises unary expressions, precedence, and guarded nullable operands.

The [Boolean negation example](examples/compiler-boolean-negation/README.md)
exercises logical inversion, precedence, and operand effects.

The [short-circuit example](examples/compiler-short-circuit/README.md) exercises
Boolean conditions, skipped calls, loops, and nullable guards.

The [logical value example](examples/compiler-logical-values/README.md)
exercises Boolean results and call order across expression joins.

The [Char array example](examples/compiler-char-arrays/README.md) exercises
String conversion, typed array calls, and read-only Array methods.

The [increment/decrement example](examples/compiler-increments/README.md) checks
Number update statements on locals, self fields, and captured array elements.

The [compound-target example](examples/compiler-compound-targets/README.md)
checks field/indexed updates, captured locations, and RHS mutations.

The [local-compound example](examples/compiler-compound-locals/README.md) checks
scalar updates, typed results, loops, and staged operands.

The [unary-plus example](examples/compiler-number-plus/README.md) checks
Number identity, precedence, guarded operands, and single evaluation.

The [Number bitwise example](examples/compiler-number-bits/README.md) checks
bit operations, signed shifts, precedence, and ordered operands.

The [typed-array write example](examples/compiler-typed-array-writes/README.md)
checks Char replacement and shared native/source View references.

The [array-write example](examples/compiler-array-writes/README.md) checks
Number element replacement, aliases, and operand order.

The [array-read example](examples/compiler-array-reads/README.md) exercises
element access, postfix chains, and array/index evaluation order.

The [Array transform example](examples/compiler-array-transforms/README.md)
checks element-preserving reverse and slice results.

The [Char-array append example](examples/compiler-array-add/README.md) checks
mutation through shared references and the typed `add` result.

The [Array mutation example](examples/compiler-array-mutation/README.md) covers
bulk append, removals, aliases, and conditional effects.

The [native statement example](examples/compiler-native-statements/README.md)
executes calls while discarding their results.

The [array literal example](examples/compiler-array-literals/README.md) constructs
Char arrays with ordered element effects and logical expressions.

The [Number-array example](examples/compiler-number-arrays/README.md) exercises
UTF-8 conversion, typed element reads, and generic Array methods.

The [nullable contract example](examples/compiler-nullable-contracts/README.md)
passes nullable values through module and instance calls.

The [lifecycle example](examples/compiler-lifecycle/README.md) supplies source
initialization and a typed native startup callback with a null-state check.

## Disassemble and assemble

Build the [Minimal Console reference](examples/minimal-console/README.md), then
export into a new directory and reassemble into a new file:

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/minimal-project
cargo run -p ciq-asm -- project scratch/minimal-project scratch/minimal-project.prg
```

The [section project](docs/specs/section-project.md) contains an ordered manifest,
assembly for supported code/metadata, and raw payloads for unsupported sections.
Labels bind supported method, line, branch, and exception references; general
relocation and opaque-reference fixups are incomplete. Equal lengths alone do
not establish runtime validity.

Without `-y`, reconstruction preserves signing material. Supply your own key to
replace signatures after reconstruction:

```sh
cargo run -p ciq-asm -- project scratch/minimal-project scratch/minimal-signed.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/minimal-signed.prg -y keys/developer_key.der
```

Standalone `sign` and `verify` use the same [signing profile](docs/specs/developer-signatures.md).
Inspection commands and editable section forms are documented in the specs below.

## Packages and reference tools

`ciq-package` lists, extracts, verifies, and packs supported barrel/IQ profiles.
Extraction preserves ordinary member bytes, raw manifests, and unknown files,
with path and resource limits. See [reading](docs/specs/package-reading.md),
[barrel packing](docs/specs/barrel-packing.md),
[IQ repacking](docs/specs/iq-repacking.md), and
[own-key IQ creation](docs/specs/iq-creation.md).

Use [SDK support scripts](scripts/README.md) for device downloads, reference
builds, and simulator sessions. Simulator execution establishes runtime
compatibility separately from successful parsing, encoding, or verification.
The reference environment is SDK 9.2.0 and Forerunner 265 (`fr265`).

## Documentation

- [Working instructions](AGENTS.md)
- [Documentation conventions](docs/README.md)
- [Plan](docs/plan.md)
- [Architecture](docs/architecture.md)
- [Research conventions](docs/research.md)
- [Official Garmin reference reading map](docs/investigations/official-garmin-documentation.md)
- [Developer signing](docs/specs/developer-signatures.md)
- [Instruction reference](docs/specs/instructions.md)
- [Symbol assembly and named operands](docs/specs/symbol-assembly.md)
- [Shared CODE labels](docs/specs/code-labels.md)
- [DEBUG strings and line mappings](docs/specs/line-numbers-and-debug.md)
- [DATA class/module inspection](docs/specs/data-classes.md)
- [DATA string assembly](docs/specs/data-strings.md)
- [DATA scalar arrays](docs/specs/data-arrays.md)
- [Resource string inspection](docs/specs/resources.md)
- [Barrel package profile](docs/specs/barrel-package.md)
- [IQ package/signing profile](docs/specs/iq-package.md)
- [IQ verification commands and reports](docs/specs/package-verification.md)
- [Package crate design](docs/design/package-crate.md)
- [Compiler goals and acceptance](docs/design/first-compiler.md)
- [Future project builds and Jungle support](docs/design/project-builds.md)
- [Investigation index](docs/investigations/README.md)
- [Monkey C examples](examples/README.md)
- [SDK support scripts](scripts/README.md)

## Local files and version control

Keep developer keys in ignored `keys/`; scripts default to
`keys/developer_key.der`. Download reference projects into ignored `references/`.
Keep SDK distributions, device data, credentials, caches, generated binaries,
and captures local; temporary experiment output belongs in `scratch/`.

Version control includes our source, scripts, tests, documentation, container
recipe, and independently created example resources. Documentation retains
reproduction details without requiring local artifacts to exist. Review
`git status --short --untracked-files=all` and `git diff --cached` before
committing; ignore rules do not remove files already tracked by Git.

## License and contributions

Copyright 2026 Jon Emil Jahren and contributors. Our source, documentation, and
examples are licensed under either [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. Third-party dependencies and external
reference materials retain their own licenses.

See [contributing](CONTRIBUTING.md) for development checks, investigation
boundaries, and contribution licensing.
