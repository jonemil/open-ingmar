# Plan

## Next work

1. Extend the [console application bootstrap](design/console-application.md) to
   source-authored lifecycle methods. Add typed native-base calls and source
   initialization, then callback parameter types and the
   null/object/array operations required for View creation. Keep complete-PRG
   native/text parity and simulator checks. Broader application profiles and
   resource assets remain open.
   The official subpage review also records inherited callback signatures and
   container source forms. Validate the documented method-argument limit before
   broadening calls; investigate extended code separately when needed.
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

## Objective

Build an independent Rust toolchain for Monkey C and Connect IQ. Use Garmin's
compiler for reference specimens and its simulator for runtime compatibility.
The current reference environment is SDK 9.2.0 and Forerunner 265 (`fr265`).

[Architecture](architecture.md) defines implementation boundaries; design pages
describe current structure and intent; investigations record supporting evidence.

Use the [official documentation reading map][official-docs] when extending
language, lifecycle, or project configuration support. Its documented contracts
guide SDK experiments; they do not replace binary-format evidence.

[official-docs]: investigations/official-garmin-documentation.md

Use [investigations](investigations/README.md) for supporting evidence and
[SDK support scripts](../scripts/README.md) for device downloads, reference
builds, and simulator sessions.
