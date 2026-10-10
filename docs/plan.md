# Plan and current status

## Objective

Build an independent, open-source Connect IQ toolchain in Rust. Monkey C remains the intended source language for any future compiler; this is not a Rust-to-Connect-IQ compiler.

Use Garmin's compiler to generate reference specimens and Garmin's simulator as the compatibility oracle. The expected initial device is the Forerunner 265 (`fr265`), subject to verification against installed SDK device definitions.

## Current status

- Original design discussion is preserved in the [session handoff](../connect-iq-rust-toolchain-codex-handoff.md).
- Repository documentation and working conventions are established.
- SDK 9.2.0 is downloaded under `connectiq/`; [local inventory](investigations/sdk-inventory.md) records launcher details and missing native simulator dependencies.
- [Hello World](../examples/hello-world/README.md) compiles and runs successfully for `fr265` using the project-local device data, with [artifact hashes and build details](investigations/hello-world-build.md) recorded. Console output and the captured simulator screen both confirm the greeting and `41`.
- [Simulator launch](investigations/simulator-container.md) is verified in the host's Ubuntu/Xvfb container. The runner preserves console logs and captures the screen while the client is active, then ends the session.
- The same launcher now provides `--interactive` for a persistent X11/Xwayland desktop window. Local lifecycle and launch-argument checks passed; the actual host desktop connection remains to be verified.
- [SDK manager inspection](investigations/sdk-manager-device-endpoint.md) identified authenticated device download endpoints. The user has now supplied the `fr265` package and fonts under `scratch/garmin-data/`.
- A [standalone browser-login device downloader](../scripts/README.md) is available, with six passing offline tests. The actual package is present; the agent has not observed the user's live sign-in flow.
- [Initial research and Rust implementation](investigations/rust-bootstrap.md) now cover pinned Ghidra plugin and optimizer sources, observed PRG framing, and the opcode subset present in two compiled specimens.
- The Cargo workspace contains working `ciq-prg`, `ciq-mc`, `ciq-sign`, `ciq-dis`, `ciq-asm`, `ciq-package`, `ciq-compiler`, and `ciq-assembly` crates. 142 Rust tests pass with Rust 1.99; the earlier 94-test checkpoint also passed on actual Rust 1.85.0. Clippy and formatting pass. Cryptographic dependencies remain isolated in the signing crate (and CLI test fixtures); archive/XML dependencies are isolated in the package crate.
- [Minimal Console](../examples/minimal-console/README.md) compiles for `fr265`; its target `hello()` function contains only `System.println("Hello!")`, within the required watch-app scaffolding.
- Both reference PRGs have completed byte-identical full-code disassembly/reassembly and container reconstruction. The original Hello World and the own-key signed, reassembled `41` → `42` variant have verified simulator behavior. The modified variant prints and displays `42`; the unedited Minimal Console reconstruction has not separately been run.
- [Complete section export/import](investigations/section-project.md) is implemented: each specimen exports 13 ordered sections as code assembly and raw binary payloads, then reconstructs an identical PRG without reading the original template. Own-key signing is implemented and the modified Hello World has confirmed runtime acceptance.
- [Symbol decoding](investigations/symbols.md) now resolves names from the PRG, annotates symbol operands, and adds an inspection listing to exports. Both annotated exports round-trip correctly. [Symbol assembly](investigations/symbol-assembly.md) now implements declarations, named operands, constrained/deterministic serialization, and version 2 project import/export; both specimens still reconstruct identically. [DATA strings](investigations/data-and-resources.md) now serialize as declarations plus pinned opaque spans, with typed `news` references and byte-identical round trips. RESOURCES strings have read-only index inspection.
- [DATA class/module inspection](investigations/data-classes.md) now decodes record headers and field keys/types/values, resolves names and verified DATA/CODE references, and adds inspection-only class sidecars to full exports. Both real examples still reconstruct identically. [Constrained class assembly](investigations/class-assembly.md) now exports authoritative `.class` declarations and method fields. [Named class references](investigations/named-classes.md) share SYMBOLS aliases and support typed class/string targets with optional placement; relocation remains unimplemented.
- [Assembly source design](design/assembly-source.md) now specifies one language for human/compiler/disassembler output, with optional layout constraints. Symbol IDs, name-record placement, and entry order are distinct; unconstrained placement should be deterministic. Symbol syntax and layout constraints are implemented; DATA string declarations are also implemented; resource assembly, includes, and ID allocation remain future work.

- [Instruction reference](specs/instructions.md) now covers all 28 supported opcodes individually, with encodings, abstract stack effects, observed examples, and explicit uncertainty; [the investigation](investigations/instruction-semantics.md) records evidence and calling-frame conflicts.

## Milestones

1. Survey existing research: inspect the MonkeyC Ghidra plugin and monkeyc-optimizer. Document PRG layout, instruction encodings, references, available serializers, and integrity/version handling. Identify differences and unknowns.
2. Read a minimal Garmin PRG: implement the smallest useful `ciq-prg` and `ciq-mc` crates based on the observed format.
3. Round trip: reconstruct a complete runnable PRG, keeping undecoded sections raw, and test it in the official simulator. Byte-identical preservation is one test mode; successful assembly may emit different bytes and use our own signing key.
4. Modify one instruction: change a visible result such as `41` to `42`, keeping instruction length unchanged initially, and verify execution.
5. Replace a function: emit a complete instruction sequence while preserving the surrounding known-good container.
6. Construct a minimal independent PRG, then extend toward a textual assembler with symbolic references and fixups.

Do not begin a Monkey C parser, barrel linker, LLVM backend, or independent VM before the target format and bytecode are sufficiently understood.

## Next investigation

The initial survey of these primary sources is pinned in [the bootstrap investigation](investigations/rust-bootstrap.md):

- [Gualdimar/MonkeyC-Ghidra-Plugin](https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin)
- [markw65/monkeyc-optimizer](https://github.com/markw65/monkeyc-optimizer)

The same-size `41` → `42` Hello World edit now runs successfully in the simulator. A [13-byte arithmetic replacement for `probe()`](investigations/function-replacement.md) is now assembled, signature-verified, and runtime-validated. It computes `40 + 2` using a local slot, preserves the entry point and all surrounding sections, and round-trips exactly. The user ran it on the host successfully; fresh console and screen captures both show `42`. Constrained class/method declarations are now implemented and byte-identical round trips verified. [Shared code labels and controlled SDK size comparisons](investigations/code-labels-and-relocation.md) are now implemented/recorded. The 24-byte appended replacement for the original 13-byte `probe()` now runs successfully in the host simulator: fresh console and screen captures on 2026-10-09 both show `43`. [Structured DEBUG strings and line mappings](investigations/line-metadata.md) now share CODE labels and round-trip both reference PRGs exactly. Signed in-place growth (expected `44`) and shrinkage (`45`) are structurally/signature-verified. The user reported success; fresh host console and screen captures confirm the shrunk variant returning `45`. A separate grown-variant capture is not retained. [Multi-file and barrel/package experiments](investigations/multi-file-and-barrels.md) now compile and round-trip. Reconstructed multi-file output `46`, an assembly edit to `47`, barrel-app `46`, and an extracted IQ device PRG `46` all have fresh console/screen validation through container-broker. [Duplicate module method names](investigations/module-method-names.md) are now investigated, with owner-qualified export labels and a simulator-verified Left-only edit. The constant-return compiler milestone now runs in the simulator. Arithmetic is now simulator-verified; initialized locals are also simulator-verified; zero-argument module calls are also simulator-verified; next investigate argument-bearing calls alongside branch/exception investigation. General relocation remains restricted pending exception/header/opaque metadata handling. Keep existing placement constraints. These barrel builds compile source into one CODE/DATA layout and do not establish a need for a separate linker.

Complete section export/import is now available. Next extend raw sections into structured representations where evidence supports it, particularly class declarations and resources now that class/module fields, symbols, and DATA strings have readable inspection. The assembler now supports `-y <developer-key>`, standalone `sign`, and `verify`. [Signing evidence](investigations/developer-signing.md) covers both SDK examples and the signed Hello World `41` → `42` edit. Matching the original signature or obtaining its private key is not required. The user ran `sh scripts/run-simulator-container.sh --prg scratch/signing/hello-42.prg` on the host successfully. The agent inspected the resulting console log and screen: both show `42`, and the PRG hash matches the prepared output. Podman remains unavailable locally, but the container-broker plugin now provides verified execution in the existing Garmin simulator container.

Extend the reconnaissance to cover:

- PRG headers, sections, endianness, lengths, and version fields.
- Opcode definitions, operand widths, and instruction boundaries.
- Function, class, module, symbol, constant, and string representations.
- Read/write support and treatment of unknown data.
- Integrity, checksum, or signature behavior after modifications.
- A minimum evidence-based Rust API and the smallest observable reference application.

Continue cross-checking source-derived findings against tiny Garmin-generated specimens. Equal-length replacement avoids relocating offsets but does not make an old signature valid for changed bytes. Changed-bytecode acceptance is now verified for this same-size edit. The different arithmetic function body is also validated. In-place size variants now resolve supported method and line references; runtime validation and remaining relocation research are still needed. The signing boundary and current SDK CLI limitation are recorded in [the architecture](architecture.md#complete-section-export-and-signing-boundary).

## Multi-file and library checkpoint, 2026-10-10

Added independently authored `multi-file`, `number-barrel`, and `barrel-app` examples. The multi-file module is callable from lifecycle and view code; full reconstruction and a same-size assembly edit run successfully. Both archive and source barrel consumption produce identical CODE/DATA. The barrel is a source ZIP; the SDK IQ export is a 7z distribution package with a device PRG, external debug XML, and signing metadata. Our existing tools preserve and run its extracted PRG; IQ writing and package-signature regeneration remain unimplemented. See [evidence and hashes](investigations/multi-file-and-barrels.md). No Rust changes or new compiler/linker stage were required.

## Packaging checkpoint and next task, 2026-10-10

The user requested format research and a packaging plan and approved `ciq-package` as a separate crate. [The investigation](investigations/package-formats.md) now confirms ZIP Store/Deflate barrel repacking via real SDK consumption, resource payload placement, 7z/LZMA2 IQ framing, SPKI public-key encoding, duplicated product/PRG signatures, and SHA-256 signing of exact manifest bytes. [Barrel](specs/barrel-package.md) and [IQ](specs/iq-package.md) profiles distinguish observed behavior from unknown historical/multi-device/store variants.

Stage 1 of [the package-crate design](design/package-crate.md) is implemented: `ciq-package list` and bounded safe `unpack` for the supported barrel/IQ profiles. [The reading specification](specs/package-reading.md) records deliberate limits (including ZIP64 and encoded 7z headers); [the investigation](investigations/package-reading.md) records all 14 real member comparisons, 18 new tests, advisory/license review, and actual Rust 1.85.0 compatibility. Unknown ordinary payloads and exact XML bytes are preserved. [Package signature verification](specs/package-verification.md) is now implemented for the modern developer IQ profile; All initial package stages are now available for their documented profiles.

Stage 2 is complete for the documented modern developer profile: `ciq-sign` public-SPKI/detached signature APIs and `ciq-package verify` report exact-byte manifest verification, PRG signatures, product attribute equality, package/PRG key consistency, and optional expected public-key identity. [Evidence](investigations/package-verification.md) records SDK/independent-RSA agreement, focused tampering tests, and 94 passing tests on Rust 1.85.0 and 1.99. Old/store profiles remain explicitly unsupported.

Stage 3 is complete: [deterministic barrel packing](specs/barrel-packing.md) preserves canonical-tree payloads, and [SDK consumption](investigations/barrel-packing.md) produces identical CODE/DATA for code and resource barrels. Stage 4 is also complete: [unchanged IQ repacking](specs/iq-repacking.md) preserves the real SDK package’s five members and public-key identity, with deterministic output and independent archive decoding. Stage 5 is complete: [own-key IQ creation](specs/iq-creation.md) regenerates PRG/product/manifest signatures with explicit debug omission/preservation. The [edited package experiment](investigations/iq-creation.md) prints/displays `47` from its extracted PRG in the official simulator. Repeated module names are now investigated and supported in export naming. The constant-return compiler milestone now runs in the simulator; arithmetic is now simulator-verified; initialized locals are also simulator-verified; zero-argument module calls are also simulator-verified; next investigate argument-bearing calls alongside branch/exception research. Broader package compatibility can wait.

## Future complete-project support

The eventual toolchain must build actual application and barrel projects using their existing Jungle configuration. Track this in [the project-build plan](design/project-builds.md): investigate and specify Jungle resolution, introduce a build layer that supplies selected inputs to the compiler, and implement project-to-barrel export that generates the packaged Jungle/layout. Application compilation and IQ export should use the same resolved configuration. Jungle parsing and project export remain unimplemented; this future work does not change the immediate packaging/bytecode priorities.


## First compiler milestone

The [first compiler milestone](design/first-compiler.md) is now implemented: `ciq-compiler` compiles `function probe() { return 41; }` through a [small value-based semantic IR](design/compiler-ir.md) into shared assembly. Explicit integration into the known-good application project, own-key signing, and simulator execution verify both `41` and changed source `42`; see [evidence and reproduction](investigations/first-compiler.md). The [supported subset](specs/compiler-subset.md) rejects unsupported syntax with source locations. Readable `.mcasm` remains the low-level debugging boundary.

Integer addition/subtraction and grouping are complete. Initialized locals are also complete. Zero-argument user module calls are complete. Next investigate argument-bearing calls, then strings/`System.println`, using controlled specimens. Continue branch/exception research before supporting conditions or general relocation. Complete application generation and Jungle-driven project builds remain future work; neither general relocation nor broad package compatibility was needed for the first straight-line function.

## Native/text pipeline checkpoint

Before extending the source language, the user requested testing stage composition and textual resumption. [The checkpoint](investigations/compiler-pipeline.md) now implements source → validated IR → shared assembly model → CODE, without requiring intermediate text. [IR text](specs/compiler-ir-text.md) and `.mcasm` can each be saved, edited, read, and compiled onward through the same validation/lowering/encoding. `ciq-assembly` isolates the CODE model/parser/printer from PRG/signing orchestration. Native, IR-resumed, and assembly-resumed outputs match for every supported literal; an edited IR returning `43` runs successfully in the simulator. The format/API remain provisional. Arithmetic now preserves these boundary tests; retain them as locals/calls grow.

## Architecture discussion follow-up

Keep the current `ciq-assembly` shared-library name and `ciq-asm` / `ciq-dis` tool pair. [The clarified layer plan](design/compiler-ir.md#layer-responsibilities-and-naming) distinguishes frontend, semantic IR, VM codegen, assembly infrastructure, and concrete instruction encoding. IR/codegen remain inside `ciq-compiler` today. Backend lowering is now isolated in its own module; `ciq-ir` and possibly `ciq-codegen` are future extraction options, with IR independent of VM lowering/assembly. Dedicated IR CLI packaging is optional. No rename or new crate is required now. Keep representations and APIs provisional while testing native composition and text stop/resume. Arithmetic is complete; initialized locals are also complete; the first module-call form is also complete; next investigate argument-bearing calls, alongside the existing control-flow research.

## Arithmetic checkpoint, 2026-10-10

[Integer arithmetic](investigations/compiler-arithmetic.md) now supports binary `+`/`-`, left associativity, and parentheses. Semantic definitions retain explicit value dependencies; the backend schedules a validated pure expression tree. Native, saved-IR, and saved-assembly outputs agree. Official SDK 9.2.0/fr265 console and screen captures confirm `40 + (8 - 5)` → `43` and `2 - 5` → `-3`. All 129 tests, Clippy, and formatting pass. Next extend to local declarations/reads using controlled frame specimens; calls and control flow follow separately.

## Initialized-local checkpoint, 2026-10-10

[Local declarations/reads](investigations/compiler-locals.md) now resolve earlier names, represent semantic local identities separately from values/VM slots, and preserve ordered initialization and repeated reads. Controlled SDK output confirms two-local slot numbering. Generated two/three-local functions print/display `37` and `-6` in the official simulator; signing/preservation checks pass and the positive output round-trips exactly. Native, IR-resumed, and assembly-resumed CODE agree. All 136 workspace tests, Clippy, and formatting pass. Reassignment remains unsupported. Next study controlled call/frame specimens and implement a first call form; string/member calls, branches/exceptions, and complete projects remain separate follow-ups.

## Zero-argument module-call checkpoint, 2026-10-10

[Module calls](investigations/compiler-calls.md) now support `Numbers.member()` with scaffold-supplied integer-return callees. IR carries symbolic names and rejects call-containing schedules the current backend cannot preserve. `--symbols` binds CODE output to an explicit existing symbol source, distinguishing `globals/Numbers` from the short field symbol. Native, IR, and assembly paths agree. Simulator console/display confirms `49` with `left` → `right` logs and reversed subtraction `2` with `right` → `left` logs. Signing/preservation checks and positive exact round trip pass. All 142 workspace tests, Clippy, and formatting pass. Next investigate parameter slots/argument-bearing calls, then strings/member calls such as `System.println`; complete module metadata/project generation remains deferred.
