# Architecture

## Implementation map

Use this map when ownership is unclear; search directly when an identifier or
error message is known. Source filenames below are relative to
`crates/ciq-compiler/src/`. Follow the contract link for meaning, then search the
owning files for the definition to read or change.

| Concern | Implementation entry points | Contract |
|---|---|---|
| Source syntax and names | [parser.rs], [names.rs] | [Source grammar] |
| Source receiver member candidates | [source_calls.rs] | [Source identities] |
| Source → IR candidates | [source_ir.rs], [source_types.rs], [unchecked.rs] | [Source semantics] |
| Source ambiguity reporting | [type_check.rs] | [Checking policy] |
| IR model and body checks | [lib.rs] | [IR validation] |
| Declarations and unit checks | [unit.rs], [source_fields.rs] | [Declarations] |
| CFG and frame flow | [control_flow.rs], [local_flow.rs], [type_flow.rs] | [IR control flow] |
| Field views | [field_flow.rs] | [Storage] |
| Record identities and views | [records.rs], [record_flow.rs] | [Records] |
| Native unions and source identity | [unions.rs], [source_union.rs], [source_objects.rs] | [Source identities] |
| Container shapes | [tuples.rs], [dictionary.rs] | [Types] |
| Interfaces and Method contracts | [structural.rs], [callbacks.rs] | [Interfaces] |
| Lexical assertions | [casts.rs] | [Assertions] |
| Operators | [operators.rs] | [Expressions] |
| IR text and shared type tags | [ir_text.rs], [type_text.rs] | [IR grammar] |
| Private container construction | [builders.rs] | [IR containers] |
| SDK declarations and member profiles | [api.rs], [native.rs] | [SDK contracts] |
| Pinned VM API identities | [native_bindings.rs] | [SDK contracts] |
| Literal and Symbol operand binding | [bindings.rs] | [Assembly] |
| VM instruction selection | [backend.rs] | [Lowering] |
| Application bootstrap and PRG DATA | [application.rs] | [Application generation] |
| Shared pipeline tests | [tests/common] | [Validation] |

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

The declaration parser returns one `ParsedUnit` per source file. Typedef
collection captures owner/file environments across the supplied files; targets
reuse annotation parsing with cached expansion and cycle/depth checks. Unit
compilation resolves declarations before calls. The isolated-function entry
selects a body from the same parser, retaining external-call binding without
unit ownership/signature validation. Base-expression and postfix frames keep
the expression-depth budget stable.

Candidate types and origins guide IR construction but are not validation
proofs. Body checks establish per-function invariants; unit checks establish
declaration, signature, field, and callback obligations. API declarations,
executable member profiles, and pinned VM identities have separate owners.
Shared type spellings retain each consumer's accepted domain. Exact source
assertions canonicalize to the source operation; record assertion views retain
conservative alias links without runtime conversion.

Source/unit validation and policy diagnostics share one CFG-aware source-origin
replay. Boundary overlap permits uncertainty without rewriting actual class
alternatives; field and dispatch metadata still require declared-contract
agreement.

Source identities stay separate from native projections, which cannot authorize
source dispatch or interface witnesses. Container elements retain identities in
`Type::SourceValue` and `ArrayElement::Source`; reads restore provenance, and unit
checks prove unresolved class members. Interface and callback contracts reuse
source-aware parameters/results. A native projection cannot invent a bound
Method signature; binding and invocation use the existing runtime Method API.

Frame flow shares capture tracking with candidate inference. Field flow solves
field/frame-local views together and retains captured receiver provenance across
the CFG. Current-instance fields use names; structural fields use canonical
groups of declaration locations, independently of runtime owner identity. Unit
checks anchor declarations; runtime field reads remain ordinary property reads.
Record flow reconstructs reference identities and mutable field views for both
source candidates and final checks. Record construction shares Dictionary
builders and member profiles, with per-key validation in IR checks.

Uncontextual Array literals use positional tuple contracts; expected Array
contracts select typed construction without first-element inference. Tuple
operations derive slot replacement/append, reverse/slice, and erased removal
views. These views do not change native method result contracts. Invariant
source-field lookup and operation metadata retain declarations until unit
validation verifies the owning classes. Container/interface shapes retain exact
witnesses instead of replacing their contents with opaque Object.

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

[parser.rs]: ../crates/ciq-compiler/src/parser.rs
[names.rs]: ../crates/ciq-compiler/src/names.rs
[source_ir.rs]: ../crates/ciq-compiler/src/source_ir.rs
[source_types.rs]: ../crates/ciq-compiler/src/source_types.rs
[lib.rs]: ../crates/ciq-compiler/src/lib.rs
[unit.rs]: ../crates/ciq-compiler/src/unit.rs
[source_fields.rs]: ../crates/ciq-compiler/src/source_fields.rs
[control_flow.rs]: ../crates/ciq-compiler/src/control_flow.rs
[local_flow.rs]: ../crates/ciq-compiler/src/local_flow.rs
[type_flow.rs]: ../crates/ciq-compiler/src/type_flow.rs
[field_flow.rs]: ../crates/ciq-compiler/src/field_flow.rs
[record_flow.rs]: ../crates/ciq-compiler/src/record_flow.rs
[unions.rs]: ../crates/ciq-compiler/src/unions.rs
[source_union.rs]: ../crates/ciq-compiler/src/source_union.rs
[source_objects.rs]: ../crates/ciq-compiler/src/source_objects.rs
[tuples.rs]: ../crates/ciq-compiler/src/tuples.rs
[dictionary.rs]: ../crates/ciq-compiler/src/dictionary.rs
[records.rs]: ../crates/ciq-compiler/src/records.rs
[structural.rs]: ../crates/ciq-compiler/src/structural.rs
[callbacks.rs]: ../crates/ciq-compiler/src/callbacks.rs
[casts.rs]: ../crates/ciq-compiler/src/casts.rs
[operators.rs]: ../crates/ciq-compiler/src/operators.rs
[ir_text.rs]: ../crates/ciq-compiler/src/ir_text.rs
[type_text.rs]: ../crates/ciq-compiler/src/type_text.rs
[builders.rs]: ../crates/ciq-compiler/src/builders.rs
[api.rs]: ../crates/ciq-compiler/src/api.rs
[native.rs]: ../crates/ciq-compiler/src/native.rs
[native_bindings.rs]: ../crates/ciq-compiler/src/native_bindings.rs
[bindings.rs]: ../crates/ciq-compiler/src/bindings.rs
[backend.rs]: ../crates/ciq-compiler/src/backend.rs
[application.rs]: ../crates/ciq-compiler/src/application.rs
[tests/common]: ../crates/ciq-compiler/tests/common/mod.rs
[Source grammar]: specs/frontend/grammar.md
[Source semantics]: specs/frontend/README.md
[IR validation]: specs/ir/README.md#validation
[Declarations]: specs/frontend/README.md#declarations-and-name-resolution
[IR control flow]: specs/ir/README.md#control-flow
[Storage]: specs/frontend/README.md#instance-fields
[Source identities]: specs/frontend/README.md#source-class-signatures
[Types]: specs/frontend/README.md#types-and-nullability
[Interfaces]: specs/frontend/README.md#structural-interfaces
[Assertions]: specs/frontend/README.md#lexical-casts
[Expressions]: specs/frontend/README.md#expressions-and-operators
[IR grammar]: specs/ir/grammar.md
[IR containers]: specs/ir/README.md#containers-and-private-construction
[SDK contracts]: specs/sdk/README.md
[Assembly]: specs/assembly/README.md
[Lowering]: specs/lowering/README.md
[Application generation]: specs/build/README.md#complete-application-generation
[Validation]: validation.md
[Records]: specs/frontend/README.md#symbol-keyed-records

[type_check.rs]: ../crates/ciq-compiler/src/type_check.rs
[Checking policy]: specs/frontend/README.md#type-checking-policy

[source_calls.rs]: ../crates/ciq-compiler/src/source_calls.rs

[unchecked.rs]: ../crates/ciq-compiler/src/unchecked.rs
