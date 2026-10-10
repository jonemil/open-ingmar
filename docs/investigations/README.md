# Investigation index

- [Local SDK inventory](sdk-inventory.md): SDK 9.2.0 files, launcher interface, and initial execution-environment limitations.
- [Hello World reference build](hello-world-build.md): successful SDK 9.2.0 build using project-local `fr265` device data, with artifact hashes recorded; baseline simulator behavior verified.
- [Simulator container](simulator-container.md): verified Ubuntu/Xvfb baseline and prepared interactive desktop mode.
- [Initial Rust workspace and PRG round trip](rust-bootstrap.md): pinned Ghidra/optimizer research, smaller console specimen, local framing/opcode specs, four working crates, and byte-identical round trips for both examples.
- [Complete section export/import](section-project.md): all section payloads plus an ordered manifest, reconstruction without the original template, raw fallback for unsupported code, and 17 passing tests.
- [Symbol decoding and instruction annotations](symbols.md): experimentally verified symbol records, names resolved from PRGs, escaped annotations and inspection sidecars, and 22 passing tests.

- [Supported instruction semantics](instruction-semantics.md): all 28 supported opcodes, compiled occurrence evidence, abstract stack effects, and unresolved calling/frame conventions.

- [Symbol assembly and named-code round trips](symbol-assembly.md): authoritative symbol declarations, optional layout constraints, version 2 projects, both real PRGs reconstructed identically, and 31 passing tests.

- [DATA strings and initial resource inspection](data-and-resources.md): class record framing, 52/53 string records, typed references, constrained edits, byte-identical projects, read-only resource indexes, and 42 passing tests.

- [DATA class/module fields and method bindings](data-classes.md): read-only record/field inspection, verified method references, inspection sidecars, byte-identical real projects, and 47 passing tests.

- [Developer signing and the verified `41` → `42` edit](developer-signing.md): independently verified SDK signature coverage, own-key assembly, exact unchanged re-signing, 52 passing tests, and confirmed host simulator execution with console/display `42`.

- [Same-size arithmetic function replacement](function-replacement.md): a complete 13-byte `probe()` replacement, signature and preservation checks, and successful host execution of arithmetic/local-frame instructions.

- [Lossless class assembly and relocation inventory](class-assembly.md): authoritative class/field declarations, byte-identical full exports, and evidence boundaries for code references.

- [Named class declarations and typed DATA references](named-classes.md): shared symbol aliases, class/string references, optional placement, and byte-identical exports.

- [Shared CODE labels and controlled size comparisons](code-labels-and-relocation.md): verified method/line relocation observations and a successfully executed appended 24-byte replacement showing `43` in console and screen.

- [Structured DEBUG and line metadata](line-metadata.md): version 4 projects, exact real-example round trips, 67 passing tests, and signed in-place grow/shrink variants and confirmed shrunk-function execution returning `45`.

Follow [the research conventions](../research.md) when recording findings. Runtime acceptance of the same-size Hello World edit is confirmed; the arithmetic function replacement is also confirmed; the larger appended function is also confirmed; structured DEBUG/line metadata is implemented and the bounded in-place shrink experiment has confirmed simulator execution, as described in [the plan](../plan.md).

- [Multi-file source, barrels, and IQ packages](multi-file-and-barrels.md): three new examples, source/archive comparison, exact PRG reconstruction, assembly-edited cross-file calls, and four successful simulator checks through container-broker.

- [Barrel/IQ format and packaging research](package-formats.md): independent ZIP repacking accepted by the SDK, resource placement, 7z coder/header inspection, verified exact-byte manifest signing, and the proposed `ciq-package` stages.

- [Rust package reading and safe extraction](package-reading.md): sixth workspace crate, exact barrel/IQ member preservation, 18 new tests, bounded archive adapters, and actual Rust 1.85.0 validation.

- [Modern IQ package signature verification](package-verification.md): public SPKI/detached signing APIs, per-product integrity/key checks, optional expected identity, SDK/independent-RSA agreement, and 94 passing tests.

- [Deterministic Rust barrel packing](barrel-packing.md): canonical extracted-tree writer, payload preservation, repeated ZIP byte equality, and SDK CODE/DATA equivalence for code and resource barrels.
