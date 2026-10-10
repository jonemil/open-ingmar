# Plan and current status

## Objective

Build an independent Rust toolchain for Monkey C and Connect IQ. Use Garmin's
compiler for reference specimens and its simulator for runtime compatibility.
The current reference environment is SDK 9.2.0 and Forerunner 265 (`fr265`).

[Architecture](architecture.md) defines implementation boundaries; design pages
describe planned behavior and investigations record the supporting evidence.

## Current capabilities

- Eight workspace crates implement PRG handling, instructions, assembly,
  disassembly, signing, packaging, and the restricted compiler. See
  [crate responsibilities](architecture.md#scope-and-boundaries).
- Full section export/import reconstructs both SDK reference PRGs byte for byte,
  with structured supported metadata and opaque fallback.
- Named symbols, DATA strings/classes, method CODE labels, DEBUG strings, and
  line mappings support the documented constrained assembly profiles.
- Named branch/subroutine operands resolve signed relative displacements;
  symbolic CODE export shares method/line/branch labels. An assembler-produced
  loop edit is simulator-verified. Ordered exception records and named offsets
  round-trip losslessly; a moved handler is simulator-verified.
- Own-key PRG signing/verification and supported barrel/IQ reading, packing,
  verification, and IQ creation are implemented.
- Source → IR → assembly → CODE works in memory and through saved text. Integer
  arithmetic, initialized locals, user-module calls, integer parameters,
  positional arguments, string literals/locals, `System.println` statements,
  and nested `if/else` statements using `==` or `>` are simulator-verified.
  Conditions support scoped arm locals, early returns, and forward joins.
  Fixed-type integer/string reassignment is [also simulator-verified][assignments].
  `while` loops with mutable locals and nested branches are [verified][loops],
  including zero and multiple iterations. Innermost `break`/`continue` and
  optional `else` are [also verified](investigations/compiler-loop-control.md).
- Compilation units collect global/module functions across files, resolve forward
  calls, and check integer argument counts/types. Native/unit-IR/assembly CODE
  agrees; [the simulator](investigations/compiler-unit.md) verifies all four
  generated bodies and reversed arguments.
- A [complete console app](investigations/application-bootstrap.md) now generates
  lifecycle CODE, DATA/modules, native imports, and all unsigned container fields
  without SDK payloads. Own-key signed greeting `41`/`42` and two-file `48` runs
  are verified; the View is blank and output is in the console.
- Void functions and qualified statement calls are
  [simulator-verified](investigations/compiler-void.md), including early loop
  returns, argument order, discarded integer results, and void startup. Saved
  unit IR produces identical complete unsigned PRGs.
- AppBase-derived source classes and explicit `self` calls are
  [simulator-verified](investigations/compiler-class.md). Unit ownership and
  instance contracts resolve same-name methods separately from globals/modules;
  saved class IR regenerates the same complete PRG. Lifecycle/View/base setup
  remains generated.
- The latest implementation checkpoint passes 216 Rust tests, Clippy, and
  formatting on Rust 1.99. The earlier 94-test checkpoint also passed Rust 1.85.

## Next work

1. Extend the [console application bootstrap](design/console-application.md) to
   source-authored lifecycle methods. Class ownership, current-receiver calls,
   void returns, and statement calls are implemented. Next add typed native-base
   calls and source initialization, then callback parameter types and the
   null/object/array operations required for View creation. Keep complete-PRG
   native/text parity and simulator checks. Broader application profiles and
   resource assets remain open.
2. Implement [Jungle-driven project builds](design/project-builds.md), including
   application/barrel inputs and generated export configuration.
3. Revisit DATA string authoring and shared address-region rules as complete
   applications expose actual placement needs. Section/offset research is not a
   prerequisite for the minimal application; address concrete blockers when
   encountered. The [boundary experiment](investigations/string-address-boundary.md)
   proves a working `0x01000000` literal offset with raised simulator memory;
   the upper limit remains unverified. Expand parsing/layout/binding together
   without extrapolating to CODE/method offsets. Device budgets remain a separate
   [build-layer concern](design/project-builds.md#target-constraints-and-section-layout).

Prioritize complete applications before broader section-size experiments.
Larger generated programs should help identify which format and device limits
need investigation; their observed limits must not become universal assumptions.

Exception source lowering still needs stack/frame research.

Broader package variants, a separate linker, LLVM, and a full VM are deferred.
Possible IR/codegen crate extraction follows demonstrated need, not a naming
scheme. See [compiler design](design/compiler-ir.md).

An independent runtime remains a possible longer-term direction. It should reuse
`ciq-prg` and `ciq-mc`, beginning with headless execution and a small Toybox API
subset before watch GUI/device simulation. This needs further VM semantics
research and is not part of the current implementation plan.

## Evidence and reproduction

Detailed chronology, source inputs, commands, hashes, and runtime observations
belong in [investigations](investigations/README.md). Key checkpoints:

- [Initial format/instruction survey](investigations/rust-bootstrap.md),
  [Hello World](investigations/hello-world-build.md), and
  [simulator setup](investigations/simulator-container.md).
- [Signing](investigations/developer-signing.md),
  [function replacement](investigations/function-replacement.md),
  [CODE labels/size changes](investigations/code-labels-and-relocation.md), and
  [line metadata](investigations/line-metadata.md).
- [Multi-file/barrels](investigations/multi-file-and-barrels.md),
  [package reading](investigations/package-reading.md),
  [verification](investigations/package-verification.md),
  [barrel packing](investigations/barrel-packing.md),
  [IQ repacking](investigations/iq-repacking.md), and
  [IQ creation](investigations/iq-creation.md).
- [Initial compiler](investigations/first-compiler.md),
  [native/text pipeline](investigations/compiler-pipeline.md),
  [arithmetic](investigations/compiler-arithmetic.md),
  [locals](investigations/compiler-locals.md),
  [module calls](investigations/compiler-calls.md),
  [parameters/arguments](investigations/compiler-arguments.md), and
  [strings/printing](investigations/compiler-strings.md).

Use [SDK support scripts](../scripts/README.md) for device downloads, reference
builds, and simulator sessions. The build helpers default to ignored
`scratch/garmin-data/` for device definitions/fonts. Temporary artifacts are
reproducible inputs/outputs, not checked-in evidence requirements.

[nested]: investigations/compiler-nested-conditions.md

[assignments]: investigations/compiler-assignments.md

[loops]: investigations/compiler-loops.md
