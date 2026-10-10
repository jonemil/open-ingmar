# Architecture

## Scope and boundaries

Build an independent Monkey C toolchain for Connect IQ. Keep container formats,
VM instructions, assembly, compilation, signing, and packaging independently
usable. LLVM's separation of concerns informs the design; LLVM is not a required
dependency.

The workspace uses resolver 3, edition 2024, and a committed lockfile. All eight
crates are currently unpublished.

- **`ciq-prg`**: ordered PRG sections, section manifests, and supported DATA,
  SYMBOLS, DEBUG, and resource structures. Standard library only.
- **`ciq-mc`**: concrete instruction definitions and symmetrical byte
  encoding/decoding. Preserve operand bit patterns. Standard library only.
- **`ciq-assembly`**: symbolic CODE model, text parsing/printing, label layout,
  reference binding, and encoding. Depends on `ciq-prg` and `ciq-mc`.
- **`ciq-asm`**: complete section-project reconstruction, template edits, and
  optional signing/verification. Depends on `ciq-prg`, `ciq-assembly`, and
  `ciq-sign`.
- **`ciq-dis`**: inspection and section-project export, with raw fallback.
  Depends on `ciq-prg` and `ciq-mc`. Keep it paired with `ciq-asm`.
- **`ciq-sign`**: PRG/detached signatures, private keys, and public SPKI keys.
  Depends on `ciq-prg`, RustCrypto RSA/SHA-1/SHA-256, and OS randomness.
- **`ciq-package`**: barrel/IQ inventory, bounded extraction, verification,
  packing, and IQ creation. Depends on `ciq-prg`, `ciq-sign`, `zip`,
  `sevenz-rust2`, and `quick-xml`; archive/XML dependencies stay here.
- **`ciq-compiler`**: restricted source parsing, semantic IR validation/text
  I/O, and VM lowering. Depends on `ciq-mc` and `ciq-assembly`.

See [the documentation guide](README.md) for the distinction between design,
format specifications, evidence records, and Rust API documentation.

## Representation and preservation

Container handling preserves section order, duplicate/unknown identifiers, and
opaque payloads. Supported code and metadata have structured views; unsupported
content remains raw. Exported section projects can reconstruct a complete PRG
without an original template. A single compiled function cannot yet produce a
complete application.

The [assembly language](design/assembly-source.md) serves handwritten code,
compiler output, and disassembly. Its [shared model](design/assembly-model.md)
keeps labels and symbol/string references until layout and encoding. Identity
and placement are distinct; optional constraints pin layout, while unconstrained
allocation must be deterministic.

CODE labels resolve supported DATA method targets and line PCs. DEBUG aliases
resolve line filename/function offsets. General relocation, branch/exception
fixups, extended CODE, resource assembly, and automatic symbol ID allocation
remain incomplete. Every length-changing edit must account for affected
references and integrity fields. Equal length alone does not prove validity.

## Complete section export and signing boundary

Project reconstruction is separate from signing. Without `-y`, preserve raw
signing material. With `-y <developer-key>`, reconstruct first, remove original
developer/store signing sections, and append one signature using the supplied
key. Private keys remain outside exported projects and Git. Standalone `sign`
and `verify` operations use the same signing layer.

The output need not reproduce the original developer's identity. Our intended
workflow uses our own key and tests executable behavior. This does not make an
output an update signed by the original publisher. See
[the signature profile](specs/developer-signatures.md) and
[signing evidence](investigations/developer-signing.md).

### Test expectations

- **Opaque preservation**: unchanged reading/writing must retain every byte,
  including signatures, without requiring an original private key.
- **Reassembly with signing**: accept expected signature/public-key changes,
  verify the new identity and integrity, check intended code/data changes, and
  independently compare unrelated sections.
- **Runtime compatibility**: successful parsing, encoding, or verification does
  not establish execution. Use the official simulator for behavior.

Expected signing differences do not excuse arbitrary binary mismatches.
Signature and preservation tests cover malformed inputs, tampering, key
replacement, and CLI workflows. Evidence records retain simulator observations.

## Shared compiler/assembly pipeline

The current flow is source → validated semantic IR → symbolic VM assembly →
CODE. IR and `.mcasm` text are optional serialization boundaries; the native
path does not print and reparse intermediate files. Both paths share validation
and encoding and must remain equivalent as representations evolve.

`ciq-compiler` owns semantic values and locals, positional parameters, ordered
calls, and source diagnostics. `backend::lower()` selects VM instructions and
assigns frame/local slots; `Function::lower()` delegates to it. Assembly
receives already-selected instructions and handles layout/binding/encoding
through `ciq-mc`. See [compiler design](design/compiler-ir.md),
[the source subset](specs/compiler-subset.md), and
[the IR format](specs/compiler-ir-text.md).

Generated functions currently use existing application DATA/module metadata.
Symbol binding consumes an explicit symbol source; it does not create method
records. Cross-function arity/type checking and complete metadata generation
remain future work. Signing, packaging, and project configuration do not belong
in the frontend.

### Agreed layer names and future extraction

Keep `ciq-assembly` as shared assembly infrastructure and `ciq-asm` / `ciq-dis`
as the paired tools. Semantic IR and codegen are distinct responsibilities
inside `ciq-compiler` today.

An eventual `ciq-ir` could own semantic types, validation, and text I/O without
VM or assembly dependencies. VM-specific lowering would remain in a backend,
possibly `ciq-codegen`. Extract crates when consumers or dependency boundaries
warrant it; the present names, APIs, and provisional IR schema need not be
frozen.

## Source projects and packages

Observed barrels distribute source/build metadata, not VM object files. IQ
archives distribute device PRGs plus package metadata/signatures. A PRG remains
the assembler's reconstruction unit; re-signing it alone does not regenerate IQ
signatures. Package operations have separate bounded profiles and lifecycle
rules. See [package design](design/package-crate.md) and
[format evidence](investigations/package-formats.md).

Multi-file and source/archive-barrel builds produce one CODE/DATA layout in the
observed SDK outputs. This supports collecting declarations before whole-program
layout, without requiring topological source order, a relocatable object format,
a separate linker, or an option to discard placement constraints.

A future build layer will resolve Jungle files and manifests into compiler
inputs/export settings and generate a barrel's packaged configuration. Keep
configuration resolution separate from compilation, archive encoding, and
signing. See [project-build design](design/project-builds.md).
