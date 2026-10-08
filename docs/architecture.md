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
| `ciq-prg` | Read/write ordered opaque PRG sections and section-project manifests; preserve unknown payloads and duplicate identifiers | Standard library |
| `ciq-mc` | Decode/encode the observed instruction subset, preserving operand bit patterns | Standard library |
| `ciq-dis` | Print section inventory/code assembly, or export all sections to a project directory | `ciq-prg`, `ciq-mc` |
| `ciq-asm` | Parse assembly; reconstruct a complete section project or replace equal-length code in a PRG template | `ciq-prg`, `ciq-mc` |

This follows LLVM's useful division between a binary container layer, a machine instruction layer, and tools that share them. We have no compiler intermediate representation yet. These initial APIs can change as research exposes symbol, reference, and relocation requirements. All packages are currently unpublished and have no external Rust dependencies.

## Initial implementation

The container model supports lossless preservation of data it does not understand. The tools decode ordinary code and symbol tables for inspection, retaining non-code payloads as raw assembly inputs. Symbol names annotate numeric code operands; they are not yet accepted as assembly operands. Assembly can reconstruct a complete exported section project without the original PRG, or use the earlier code-only template workflow. It does not yet build an executable from a single function alone. Length-changing edits require evidence about every affected offset, reference, and integrity field before implementation. Same-length edits still require signature and runtime validation.

Bytecode decoding and encoding are symmetrical. The current operand model preserves raw bits without resolving references. Future symbolic assembly should preserve symbolic references until layout and encoding; derive reference and fixup kinds from observed data rather than assuming conventional addresses or linker semantics. Labels, branch relocation, extended code, and signature generation are deferred.

Byte-identical disassembly/reassembly is a useful preservation test and has already passed for both specimens. It is not a requirement for successful assembly: the intended output is a runnable program with the intended behavior, signed with the user's supplied key. Test a same-size instruction edit (`41` to `42`), and subsequently an equivalent but different function body. Semantic equivalence is judged through observed simulator behavior.

The future compiler can perform whole-program analysis over applicable Monkey C source files. Do not require topological source ordering or introduce a separate object format without evidence that it is needed.

## Complete section export and signing boundary

Implemented on 2026-10-08: export undecoded sections as raw binary, with a manifest retaining identifiers and order. Ordinary code is represented as assembly when supported, otherwise raw binary. Import reconstructs the complete container from these files without requiring the original PRG as a template. See [the project format](specs/section-project.md) and [verification record](investigations/section-project.md).

Keep signing as a separate implementation stage, exposed through the assembler's intended `-y <developer-key>` option. Assembly with this option should reconstruct the program, sign the result using the supplied key, and replace the original developer signature and associated public-key components. Original signature payloads can be retained as raw evidence for preservation tests; they are not the output signing identity when `-y` is supplied. Private keys remain external to the exported project and Git. This option is a planned interface, not currently implemented.

User clarification (2026-10-08): the original developer's private key and signature identity are unnecessary for the intended reassembly workflow. The objective is signing our output with our own key, not reproducing another developer's signature. Byte differences caused by code reconstruction or replacing signatures are acceptable. Support for generating PRG signatures still needs implementation and validation; the earlier interpretation that all signing work was deferred was too broad. SDK 9.2.0 `monkeyc --help` advertises `-y/--private-key` for signing compiler builds, but does not expose a standalone existing-PRG resign operation in that help output.

Garmin's [getting-started documentation](https://developer.garmin.com/connect-iq/connect-iq-basics/getting-started/) specifies developer-key signing and requires the original key for updates to an existing store app. Signing a research output with our own key therefore does not make it an update signed by the original developer.

### Test expectations

Separate two test modes:

- **Opaque preservation:** reading and writing unchanged sections without resigning should retain every byte, including existing signatures. This remains possible for an externally obtained PRG without possessing its signing key, because no new signature is generated.
- **Reassembly with signing:** when `-y` supplies a different key, differences in the regenerated developer signature and associated public-key fields are expected and must not fail the test. Verify the new signature against the supplied key, verify the intended code/data and raw-section preservation, and test the resulting behavior in the simulator. Do not require equality to the input signature or key.

For an otherwise unchanged reconstruction, compare preserved non-signature sections separately from regenerated signing material. For intentional program changes, assert the expected decoded changes and retain checks on unrelated raw sections. This is a targeted allowance for expected differences, not permission to ignore arbitrary binary mismatches or invalid output signatures. These signing tests become executable when signing support is implemented; current exact-byte tests exercise only opaque preservation and unchanged code reassembly.

## Local specifications

The specs include [PRG framing](specs/prg-container.md), [the bytecode subset and assembly grammar](specs/bytecode-subset.md), [section projects](specs/section-project.md), and [symbol tables](specs/symbols.md). Symbol editing, data layouts, and calling conventions remain research topics. Each specification should explain the behavior directly and link to local investigation records for its evidence.

Checked-in code should stand on its own and reference local specifications when an explanation is needed. Keep external repository paths and comparison details in research notes. Preserve attribution and any licensing obligations if material is ever incorporated; documenting findings locally does not erase their provenance.
