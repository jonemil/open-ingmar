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
  I/O, core API declarations, VM lowering, and explicit symbol/literal binding.
  Depends on `ciq-mc`,
  `ciq-assembly`, and `ciq-prg` for DATA sources.

## Shared compiler/assembly pipeline

Source → validated semantic IR → symbolic VM assembly → CODE is the native
pipeline. IR and `.mcasm` text are optional inspection/resumption boundaries;
readers reconstruct the same models and reuse validation and encoding. Semantic
IR retains values, storage identity, types, control flow, and effect order
without VM slots, native import indexes, or section offsets.

The frontend resolves declarations/names and constructs semantic IR. The backend
selects instructions and assigns frame storage; `Function::lower()` delegates to
`backend::lower()`. Assembly receives selected instructions and resolves labels,
symbols, and literal references before encoding. See the
[IR specification](specs/ir/README.md),
[lowering specification](specs/lowering/README.md), and
[assembly specification](specs/assembly/model.md).

The API catalogue supplies runtime-provided declarations and availability;
operator rules and executable native profiles check supported uses before
instruction selection. Declarations contain no runtime implementations or VM
storage decisions. [API contracts](specs/sdk/declarations.md) distinguish these
responsibilities and the target/deployment inputs.

Compilation units collect source declarations across files before checking
calls, then lower into one CODE/DATA layout with owner-qualified identities.
Source files are not relocatable objects. Application generation consumes
validated units and CODE labels to produce lifecycle assembly, DATA, imports,
and unsigned PRG sections without an SDK template. Source language, class, and
application rules belong in their [specifications](specs/frontend/README.md),
not a second feature inventory here.

Keep `ciq-assembly` as shared machine assembly infrastructure and `ciq-asm` /
`ciq-dis` as paired tools. Semantic IR and codegen remain inside `ciq-compiler`.
An eventual `ciq-ir` could own semantic models/text without VM dependencies; a
VM backend could become `ciq-codegen`. Extract crates only when consumers or
dependency boundaries justify them. A linker, LLVM, optimization, and a full VM
remain deferred in [the plan](plan.md).

## Preservation, layout, and signing

Container reading/writing preserves ordered sections, duplicates, unknown IDs,
and opaque bytes. Supported structured views must reconstruct their inputs
without a template. The [PRG](specs/formats/prg-container.md),
[section project](specs/formats/section-project.md), and section specs define exact
preservation and binding rules. Numeric/raw preservation does not establish
relocation safety; length-changing edits must account for all affected
references and integrity fields, including opaque ones.

Symbols, literals, methods, branches, line mappings, and exceptions bind through
shared layout identities where their reference semantics are known. Identity is
separate from placement. The assembly spec owns deterministic layout and
pinning; general relocation and opaque-reference fixups remain incomplete.

Signing follows reconstruction. Without a supplied key, preserve signing
material. With one, replace developer/store signing sections with the supported
own-key signature. That does not preserve the original publisher's identity or
regenerate IQ signatures. [Signing](specs/formats/developer-signatures.md) and
[validation](validation.md) distinguish byte preservation, identity/integrity,
structured reconstruction, and runtime acceptance.

## Source projects and packages

Observed barrels distribute source/build metadata; IQ packages distribute device
PRGs and signed package metadata. Package aliases are not VM symbols. PRG,
cryptography, and archive responsibilities remain separate even when an export
coordinates them. The [package profiles](specs/formats/package-reading.md)
specify supported operations.

A future project layer resolves Jungle/manifest inputs into source, resources,
dependencies, target constraints, and export settings. It must feed the
compiler, resource generation, packaging, and signing through explicit resolved
inputs, without putting configuration parsing in the frontend or archive
library.
[Project-build requirements](specs/build/project-builds.md) define that intended boundary;
[the plan](plan.md) owns sequencing.
