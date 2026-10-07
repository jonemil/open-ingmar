# Architecture

## Scope and boundaries

Use LLVM's separation of concerns as an architectural inspiration without introducing an LLVM dependency initially. Keep container handling and VM instruction definitions independently usable by a future compiler or runtime.

The intended repository layout is:

```text
Cargo.toml                 # workspace manifest, added with the first crate
crates/
  ciq-prg/                 # container reader and writer
  ciq-mc/                  # instruction model, encoding/decoding, references
  ciq-asm/                 # future textual assembler
  ciq-dis/                 # future disassembler CLI
docs/
  investigations/          # evidence and experiment records
  specs/                   # local format and instruction specifications
references/                # ignored external research checkouts
scratch/                   # ignored local SDK specimens and experiments
```

These are intended responsibilities, not established APIs or a requirement to create every crate at once. Decide dependencies and public types from the actual format findings.

## Initial implementation

The first container model should support lossless preservation of data it does not understand. Decode only enough structure to locate and safely modify a known instruction. Length-changing edits require evidence about every affected offset, reference, and integrity field before implementation.

Keep bytecode decoding and encoding symmetrical. Preserve symbolic references until layout and encoding when building new instruction streams; derive reference and fixup kinds from observed data rather than assuming conventional addresses or linker semantics.

The future compiler can perform whole-program analysis over applicable Monkey C source files. Do not require topological source ordering or introduce a separate object format without evidence that it is needed.

## Local specifications

Write format and instruction specifications under `docs/specs/` as findings become available. Candidate subjects are PRG layout, bytecode, symbols, and calling conventions. Each specification should explain the behavior directly and link to local investigation records for its evidence.

Checked-in code should stand on its own and reference local specifications when an explanation is needed. Keep external repository paths and comparison details in research notes. Preserve attribution and any licensing obligations if material is ever incorporated; documenting findings locally does not erase their provenance.
