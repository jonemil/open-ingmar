# Investigation index

- [Local SDK inventory](sdk-inventory.md): SDK 9.2.0 files, launcher interface, and initial execution-environment limitations.
- [Hello World reference build](hello-world-build.md): successful SDK 9.2.0 build using project-local `fr265` device data, with artifact hashes recorded; baseline simulator behavior verified.
- [Simulator container](simulator-container.md): verified Ubuntu/Xvfb baseline and prepared interactive desktop mode.
- [Initial Rust workspace and PRG round trip](rust-bootstrap.md): pinned Ghidra/optimizer research, smaller console specimen, local framing/opcode specs, four working crates, and byte-identical round trips for both examples.
- [Complete section export/import](section-project.md): all section payloads plus an ordered manifest, reconstruction without the original template, raw fallback for unsupported code, and 17 passing tests.
- [Symbol decoding and instruction annotations](symbols.md): experimentally verified symbol records, names resolved from PRGs, escaped annotations and inspection sidecars, and 22 passing tests.

- [Supported instruction semantics](instruction-semantics.md): all 28 supported opcodes, compiled occurrence evidence, abstract stack effects, and unresolved calling/frame conventions.

Follow [the research conventions](../research.md) when recording findings. Runtime acceptance of the reassembled specimens and a same-size bytecode edit are next, as described in [the plan](../plan.md).
