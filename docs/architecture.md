# Architecture

## Scope and boundaries

Use LLVM's separation of concerns as an architectural inspiration without introducing an LLVM dependency initially. Keep container handling and VM instruction definitions independently usable by a future compiler or runtime.

The Cargo workspace now contains eight crates:

```text
Cargo.toml                 # workspace manifest, resolver 3 / edition 2024
Cargo.lock                 # committed for reproducible CLI builds
crates/
  ciq-prg/                 # container reader and writer
  ciq-mc/                  # instruction model and encoding/decoding
  ciq-sign/                # developer signatures and key handling
  ciq-asm/                 # assembly library and CLI
  ciq-dis/                 # disassembly library and CLI
  ciq-package/             # barrel/IQ inventory and safe extraction
  ciq-compiler/            # restricted Monkey C frontend and VM lowering
  ciq-assembly/            # shared CODE model, parser, printer, and encoder
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
| `ciq-sign` | Encode/verify PRG and detached developer signatures; private-key and public-SPKI handling | `ciq-prg`, RustCrypto RSA/SHA-1/SHA-256, OS randomness |
| `ciq-asm` | Reconstruct section projects or replace equal-length code using shared assembly APIs; optional signing and verification | `ciq-prg`, `ciq-assembly`, `ciq-sign` |
| `ciq-compiler` | Parse source/IR, validate semantic value IR, lower to the shared assembly model | `ciq-mc`, `ciq-assembly` |
| `ciq-assembly` | Typed ordinary CODE assembly model, parsing/printing, label layout, reference binding, and encoding | `ciq-prg`, `ciq-mc` |
| `ciq-package` | Inventory/extract barrels and IQ archives; verify the modern developer IQ profile | `ciq-prg`, `ciq-sign`, `zip`, `sevenz-rust2`, `quick-xml` |

This follows LLVM's useful division between a binary container layer, a machine instruction layer, and tools that share them. The first frontend now uses a small value-based semantic IR, described in [the compiler decision](design/compiler-ir.md). These initial APIs can change as research exposes symbol, reference, and relocation requirements. All packages are currently unpublished. Container and instruction libraries remain standard-library-only; cryptographic dependencies belong in `ciq-sign`. The assembler CLI uses this crate after reconstruction, without coupling cryptography to the assembly library or container parser.

## Initial implementation

The container model supports lossless preservation of data it does not understand. The tools decode ordinary code, symbols, DATA strings, class/module declarations, DEBUG strings, and line mappings while preserving opaque DATA spans and other section payloads. Symbol declarations resolve typed named code operands. Structured symbol source can reconstruct known table layouts; numeric operands remain supported. Assembly can reconstruct a complete exported section project without the original PRG, or use the earlier code-only template workflow. It does not yet build an executable from a single function alone. Length-changing edits require evidence about every affected offset, reference, and integrity field before implementation. Same-length edits still require signature and runtime validation.

Bytecode decoding and encoding are symmetrical. The current operand model preserves raw bits without resolving references. Future symbolic assembly should preserve symbolic references until layout and encoding; derive reference and fixup kinds from observed data rather than assuming conventional addresses or linker semantics. Shared CODE labels resolve class method targets and structured line PCs; DEBUG aliases resolve line filename/function offsets. Branch relocation, general in-place relocation, and extended code remain deferred. Developer signature generation and verification are implemented separately in `ciq-sign`.

The agreed assembly direction is one language and shared model for handwritten source, compiler output, and disassembly. Declarations describe content and references; optional constraints pin identifiers or placement when needed. Without placement constraints, use deterministic legal allocation. Symbol identity and name-record location must be represented separately. See [assembly source and optional layout constraints](design/assembly-source.md) for the proposed syntax and implementation sequence.

Byte-identical disassembly/reassembly is a useful preservation test and has already passed for both specimens. It is not a requirement for successful assembly: the intended output is a runnable program with the intended behavior, signed with the user's supplied key. The same-size instruction edit (`41` to `42`) now runs successfully in the official simulator; an equivalent arithmetic function body using a local slot now also runs successfully. Length-changing code still requires relocation research. Semantic equivalence is judged through observed simulator behavior.

The future compiler can perform whole-program analysis over applicable Monkey C source files. Do not require topological source ordering or introduce a separate object format without evidence that it is needed.

For complete projects, a future build layer will resolve existing Jungle files and manifests into compiler inputs and export settings. It will also map a barrel project's selected files into the archive layout and generate its packaged Jungle configuration. Keep configuration resolution separate from language compilation, archive encoding, and signing; crate/API choices remain deferred. See [the project-build plan](design/project-builds.md) for scope, research, and acceptance examples.

## Complete section export and signing boundary

Implemented on 2026-10-08: export undecoded sections as raw binary, with a manifest retaining identifiers and order. Ordinary code is represented as assembly when supported, otherwise raw binary. Import reconstructs the complete container from these files without requiring the original PRG as a template. See [the project format](specs/section-project.md) and [verification record](investigations/section-project.md).

Signing is a separate stage, exposed through the assembler's `-y <developer-key>` option. Assembly with this option reconstructs the program, then signs it using the supplied key, removes original developer/store signing sections, and appends one developer signature with the supplied public-key components. Original signature payloads can be retained as raw evidence for preservation tests; they are not the output signing identity when `-y` is supplied. Private keys remain external to the exported project and Git. Standalone `sign` and `verify` commands are also implemented. See [developer signatures](specs/developer-signatures.md) for supported key formats, signature variants, and verification limits.

User clarification (2026-10-08): the original developer's private key and signature identity are unnecessary for the intended reassembly workflow. The objective is signing our output with our own key, not reproducing another developer's signature. Byte differences caused by code reconstruction or replacing signatures are acceptable. Signature generation now reproduces the SDK output for unchanged Minimal Console; the own-key signed `41` → `42` Hello World now has confirmed simulator behavior. SDK 9.2.0 `monkeyc --help` advertises `-y/--private-key` for signing compiler builds, but does not expose a standalone existing-PRG resign operation in that help output.

Garmin's [getting-started documentation](https://developer.garmin.com/connect-iq/connect-iq-basics/getting-started/) specifies developer-key signing and requires the original key for updates to an existing store app. Signing a research output with our own key therefore does not make it an update signed by the original developer.

### Test expectations

Separate two test modes:

- **Opaque preservation:** reading and writing unchanged sections without resigning should retain every byte, including existing signatures. This remains possible for an externally obtained PRG without possessing its signing key, because no new signature is generated.
- **Reassembly with signing:** when `-y` supplies a different key, differences in the regenerated developer signature and associated public-key fields are expected and must not fail the test. Verify the new signature against the supplied key, verify the intended code/data and raw-section preservation, and test the resulting behavior in the simulator. Do not require equality to the input signature or key.

For an otherwise unchanged reconstruction, compare preserved non-signature sections separately from regenerated signing material. For intentional program changes, assert the expected decoded changes and retain checks on unrelated raw sections. This is a targeted allowance for expected differences, not permission to ignore arbitrary binary mismatches or invalid output signatures. Executable signing tests now cover both payload variants, key replacement, expected code changes, raw-section preservation, tampering, malformed inputs, and CLI workflows. Independent public-RSA checks verify the real SDK examples and the prepared modified output; the modified Hello World has confirmed simulator acceptance and prints/displays `42`.

## Local specifications

The specs include [PRG framing](specs/prg-container.md), [the bytecode subset and assembly grammar](specs/bytecode-subset.md), [the per-instruction reference](specs/instructions.md), [section projects](specs/section-project.md), and [symbol tables](specs/symbols.md), [DATA strings](specs/data-strings.md), and [resource string inspection](specs/resources.md). Symbol assembly and constrained DATA string serialization are implemented. [Class/module fields](specs/data-classes.md) have inspection and lossless constrained assembly. Class/symbol/string aliases resolve in DATA; [code labels](specs/code-labels.md) now resolve method targets and [line PCs](specs/line-numbers-and-debug.md); general relocation, remaining field semantics, resource assembly, ID allocation, and calling conventions remain research topics. Each specification should explain the behavior directly and link to local investigation records for its evidence.

Checked-in code should stand on its own and reference local specifications when an explanation is needed. Keep external repository paths and comparison details in research notes. Preserve attribution and any licensing obligations if material is ever incorporated; documenting findings locally does not erase their provenance.

## Source libraries and distribution packages

[Experiments on 2026-10-10](investigations/multi-file-and-barrels.md) compile three Monkey C source files into one CODE/DATA layout. Consuming a barrel archive or its source project produces identical executable code and data. The observed barrel archive contains source and build metadata; it is not a VM object file. A separately exported IQ archive holds a device PRG plus external metadata/signatures. Its extracted PRG remains the unit our assembler reconstructs. Do not equate an IQ archive with a linkable object or assume PRG re-signing updates package-level signatures.

These observations support collecting source declarations before complete-program layout. They do not require a separate linker or an option to ignore placement constraints. [Repeated module names](investigations/module-method-names.md) now have owner-qualified exported labels with conservative fallback for shared targets. This preserves the flat assembly namespace. Branch/exception fixups, resource libraries, annotations, and multi-device distribution remain bounded follow-up investigations.

## Package crate

`ciq-package` is the sixth workspace member, implementing library/CLI inventory and safe extraction for ZIP barrels and 7z IQ archives. It owns bounded archive adapters, package manifest/product views, and extraction lifecycle; PRG decoding remains in `ciq-prg` and cryptographic primitives in `ciq-sign`. The package verifier uses both crates to check the documented modern developer IQ profile. It never loads a private key; optional expected identity is an external SPKI public key. Separate reports cover CRCs, manifest/product signatures, attribute equality, and key consistency; unsigned extras are identified. [The reading specification](specs/package-reading.md) records supported profiles and limits; [reader evidence](investigations/package-reading.md) includes exact SDK member comparisons and dependency checks; [verification evidence](investigations/package-verification.md) adds independent RSA agreement and 94 passing workspace tests with Rust 1.85.0 and 1.99. Raw manifest bytes are retained because manifest signing does not canonicalize XML. Archive/XML dependencies stay isolated from the standard-library-only PRG/instruction crates. [The design](design/package-crate.md) now includes implemented [deterministic barrel packing](specs/barrel-packing.md), and [verified unchanged IQ repacking](specs/iq-repacking.md), and [own-key IQ creation](specs/iq-creation.md). The IQ creator stages generated PRGs and changes only product signing attributes in raw XML; signing primitives remain in `ciq-sign`. Bounded single-LZMA encoded-header decoding runs before archive metadata allocation. The barrel writer preserves raw manifests and streams ordinary source/resource payloads into a bounded ZIP buffer; it uses no private key.

The [first compiler milestone](design/first-compiler.md) emits a tiny Monkey C function into the shared assembly representation, initially using known-good application scaffolding. Language parsing/lowering remains separate from layout/encoding, project configuration, signing, and packaging; The initial investigation chose a small value-based IR; `ciq-compiler` implements one integer expression-return function with addition, subtraction, grouping, initialized locals, and user module calls with integer parameters/arguments, and lowers it to VM assembly. Readable `.mcasm` remains available for debugging generated VM instructions in either direction. The preliminary compiler API separates inspectable semantic operations from assembly printing; IR text is provisional, with shared validation and tested native/text resumption paths.

## Shared compiler/assembly pipeline

`ciq-assembly` separates CODE syntax/model/encoding from `ciq-asm` PRG/signing orchestration. Source and [IR text](specs/compiler-ir-text.md) produce the same validated semantic function; lowering creates a typed assembly `Program`. Assembly text is a serialization of that model and uses the same encoder after parsing. The compiler's native CODE path does not print/reparse intermediate files and has no normal signing/archive dependency. Full DATA/resource/project generation is not moved into this new layer. [Pipeline evidence](investigations/compiler-pipeline.md) covers byte equivalence, old project reconstruction, and an IR edit executed in the simulator.

## Agreed layer names and future extraction

Keep `ciq-assembly` as the shared symbolic VM assembly infrastructure and `ciq-asm` / `ciq-dis` as the paired tools. Semantic IR and VM codegen currently both live in `ciq-compiler`; instruction selection happens in `backend::lower()`, with `Function::lower()` as a delegating convenience API. Their responsibilities are distinct even before a crate split: semantic IR describes computation, while codegen chooses VM instructions and manages stack/local/frame requirements. Assembly infrastructure receives those selected instructions and handles syntax, layout, reference binding, and encoding through `ciq-mc`.

A future `ciq-ir` would share semantic types, validation, and text I/O between frontend, backend, and other consumers. It should not depend on codegen/assembly; move VM-specific lowering out of the IR type when extracting it. The backend module depends on IR and assembly; an eventual `ciq-codegen` library would retain that direction. Separate crates and dedicated CLI packages remain optional choices as the project grows, not prerequisites for the next arithmetic milestone. Preserve both native and text-resumed pipelines as the models evolve. See [the responsibility table and agreed names](design/compiler-ir.md#layer-responsibilities-and-naming).

The [arithmetic checkpoint](investigations/compiler-arithmetic.md) verifies dependency-based lowering, native/IR/assembly byte equivalence, and simulator results `43` and `-3`. Shared expression-value graphs, reassignment, object calls, and control flow remain unsupported by the frontend.

[Initialized locals](investigations/compiler-locals.md) now separate source name resolution, semantic local IDs, and backend slot allocation. Initialization statements execute in order; repeated reads use stored values. Both text/native paths and two/three-local simulator bodies are verified.

[Zero-argument module calls](investigations/compiler-calls.md) now preserve source/IR evaluation order and remain symbolic through lowering. Explicit symbol binding resolves user-module names against existing scaffold metadata, distinguishing module identities from short field symbols. Raw CODE emission can consume an external symbol source without adding a PRG/signing dependency to the compiler. Callee implementations and DATA records are still supplied externally.

[Parameters and arguments](investigations/compiler-arguments.md) now extend signature/value IR, shared text I/O, and VM lowering together. Generated parameterized callees can be selected through existing scaffold DATA method records. Positional reads precede backend slot assignment; argument dependencies preserve left-to-right nested call execution. Whole-program arity/type checking and module metadata generation remain separate work.
