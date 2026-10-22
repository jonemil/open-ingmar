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
[assembly specification](specs/assembly/README.md#model-and-layout).

The API catalogue supplies runtime-provided declarations and availability;
operator rules and executable native profiles check supported uses before
instruction selection. Declarations contain no runtime implementations or VM
storage decisions. [API contracts](specs/sdk/README.md#api-declarations) distinguish these
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

## Implementation conventions

Within `ciq-compiler`, `parser` and `names` handle source syntax and native-name
bindings. One declaration parser returns a `ParsedUnit` for each source file;
typedef collection first captures owner/file environments across the supplied
files. Targets reuse annotation parsing, with cached expansion and cycle/depth
checks, before bodies enter semantic IR. Unit compilation collects the parsed
declarations before call resolution. The isolated-function entry
selects one global body from that same result, retaining external-call binding
without unit ownership/signature validation. `source_ir` constructs semantic
operations. Candidate types and source
origins help construction but are not validation proofs. `Function` validation
checks per-body invariants, using `local_flow` to check inferred variable reads
against possible writes at each CFG point and sharing narrowing facts with
`type_flow`. `unit` adds declaration, signature, field, and
callback checks. `api` owns declarations, `native` owns executable profiles,
`native_bindings` owns the pinned VM identities shared by runtime type tests and
application generation, and `operators` owns operand/result rules. `type_text`
shares saved-type spellings while retaining each consumer's accepted domain.
`unions` normalizes native alternatives and adapts them to supported contracts;
`source_union` normalizes exact class alternatives while `source_objects`
tracks their provenance. Source identities remain separate from native API
projections, which cannot authorize source dispatch or interface witnesses.
`structural` owns finite source interface contracts and their saved tags;
`tuples` owns positional slot contracts, saved tags, indexed result joins, and
the derived contracts for slot replacement and append operations;
`native` checks declared member use, and `unit` proves source implementations.
Interface members reuse source-aware parameter/result contracts; API projection
does not replace exact member identities or authorize source dispatch.
Container content contracts own structural shapes and share the same witness
validation; they do not replace interface-valued elements with opaque Object.
`callbacks` owns fixed Method signatures, saved tags, and binding proofs. Typed
callback signatures reuse source-aware method contracts; native projection does
not authorize an invented target signature. Binding and invocation lower through
the existing Method runtime operations.
Base-expression and postfix parsing use separate frames so suffix extensions
preserve the established expression-depth budget.
`backend` selects VM instructions;
`application` generates the bounded bootstrap and metadata. Keep these
responsibilities separate even where they currently share a source file.

- Reuse an existing rule at its owning layer before adding another type/member
  whitelist. Keep context restrictions explicit: a parameter, field, join, and
  cast need not admit identical types. Share spelling/conversion machinery
  without silently broadening those contexts.
- Source, typed IR, and saved IR must reach the same authoritative validation.
  Candidate inference may be provisional; it must not authorize code that the
  final function/unit checks cannot establish. Keep evaluation order, identity,
  null proofs, and temporary ownership explicit through lowering.
- Prefer named enums and exhaustive matches for compiler cases. Raw opcode
  bytes, record tags, and field widths belong in their format definitions;
  internal directive dispatch should not depend on unexplained numeric cases.
- Distinguish language semantics, supported executable profiles, native binding
  IDs, and target capacity. Fixed ABI/format constants are appropriate; they do
  not belong in source-language rules. Keep native binding data separate from
  API declarations, and identify the profile behind unverified target choices.
- Introduce helpers or types for a repeated invariant or a concrete consumer.
  Prefer ordinary functions and data over registries, plugin layers, or generic
  pass frameworks without a demonstrated need. Do not infer an abstraction for
  unknown language/VM behavior or extract crates merely because files grow.
- Organize code by responsibilities that can be understood and checked together.
  Module boundaries need not match documentation filenames. Refactor a bounded
  concern with its relevant extension; avoid combining a semantic change with
  an unrelated structural rewrite. Follow the existing
  [validation policy](validation.md) for the affected scope.

## Preservation, layout, and signing

Container reading/writing preserves ordered sections, duplicates, unknown IDs,
and opaque bytes. Supported structured views must reconstruct their inputs
without a template. The [PRG](specs/formats/README.md#prg-container),
[section project](specs/formats/README.md#section-project), and section specs define exact
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
coordinates them. The [package profiles](specs/formats/packages.md#shared-reading-and-extraction)
specify supported operations.

A future project layer resolves Jungle/manifest inputs into source, resources,
dependencies, target constraints, and export settings. It must feed the
compiler, resource generation, packaging, and signing through explicit resolved
inputs, without putting configuration parsing in the frontend or archive
library.
[Project-build requirements](specs/build/README.md#planned-project-builds-and-targets) define that intended boundary;
[the plan](plan.md) owns sequencing.
