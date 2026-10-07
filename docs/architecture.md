# Architecture

## Scope and boundaries

Use LLVM's separation of concerns as an architectural inspiration without introducing an LLVM dependency initially. Keep container handling and VM instruction definitions independently usable by a future compiler or runtime.

The Cargo workspace now contains four crates:

```text
Cargo.toml                 # workspace manifest, resolver 3 / edition 2024
Cargo.lock                 # committed for reproducible CLI builds
crates/
  ciq-prg/                 # container reader and writer
  ciq-mc/                  # instruction model and encoding/decoding
  ciq-asm/                 # assembly library and CLI
  ciq-dis/                 # disassembly library and CLI
docs/
  investigations/          # evidence and experiment records
  specs/                   # local format and instruction specifications
references/                # ignored external research checkouts
scratch/                   # ignored local SDK specimens and experiments
```

| Crate | Current responsibility | Dependencies |
|---|---|---|
| `ciq-prg` | Read/write ordered opaque PRG sections; preserve unknown payloads and duplicate identifiers | Standard library |
| `ciq-mc` | Decode/encode the observed instruction subset, preserving operand bit patterns | Standard library |
| `ciq-dis` | Print section inventory or ordinary code as assembly text | `ciq-prg`, `ciq-mc` |
| `ciq-asm` | Parse assembly and replace equal-length ordinary code in an existing PRG template | `ciq-prg`, `ciq-mc` |

This follows LLVM's useful division between a binary container layer, a machine instruction layer, and tools that share them. We have no compiler intermediate representation yet. These initial APIs can change as research exposes symbol, reference, and relocation requirements. All packages are currently unpublished and have no external Rust dependencies.

## Initial implementation

The container model supports lossless preservation of data it does not understand. The first tools decode ordinary code, leaving the remaining sections opaque. Assembly reuses a known-good PRG template; it does not yet build an independent executable from a single function. Length-changing edits require evidence about every affected offset, reference, and integrity field before implementation. Same-length edits still require signature and runtime validation.

Bytecode decoding and encoding are symmetrical. The current operand model preserves raw bits without resolving references. Future symbolic assembly should preserve symbolic references until layout and encoding; derive reference and fixup kinds from observed data rather than assuming conventional addresses or linker semantics. Labels, branch relocation, extended code, and signature generation are deferred.

Start with a byte-identical disassembly/reassembly baseline. Then test a same-size instruction edit (`41` to `42`), and subsequently an equivalent but different function body. Semantic equivalence is judged through observed simulator behavior, not inferred from byte equality or successful assembly.

The future compiler can perform whole-program analysis over applicable Monkey C source files. Do not require topological source ordering or introduce a separate object format without evidence that it is needed.

## Local specifications

The initial specs are [PRG framing](specs/prg-container.md) and [the bytecode subset and assembly grammar](specs/bytecode-subset.md), supported by [the Rust bootstrap investigation](investigations/rust-bootstrap.md). Symbols and calling conventions remain research topics. Each specification should explain the behavior directly and link to local investigation records for its evidence.

Checked-in code should stand on its own and reference local specifications when an explanation is needed. Keep external repository paths and comparison details in research notes. Preserve attribution and any licensing obligations if material is ever incorporated; documenting findings locally does not erase their provenance.
