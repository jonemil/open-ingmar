# Investigation index

Browse by question or component. Records describe experiments at their stated
SDK/version and scope; they are not current feature inventories. Use
[the plan](../plan.md) for upcoming work and
[research conventions](../research.md) when recording evidence.

## Reference environment

- [Official Garmin documentation](official-garmin-documentation.md): language,
  lifecycle, and Jungle reading map.
- [SDK inventory](sdk-inventory.md): files and launcher environment.
- [Hello World build](hello-world-build.md): reference specimen and device data.
- [Simulator container](simulator-container.md): startup failures, virtual
  display, baseline execution, and desktop connection.
- [Rust bootstrap](rust-bootstrap.md): source attribution and initial PRG
  round trips.

## PRG formats and assembly

- [Section projects](section-project.md): complete export/import and raw fallback.
- [Symbols](symbols.md): records and resolved instruction annotations.
- [Instruction semantics](instruction-semantics.md): stack effects and unknowns.
- [Symbol assembly](symbol-assembly.md): identity and optional placement.
- [DATA and resources](data-and-resources.md): string records and resource indexes.
- [DATA classes](data-classes.md): fields and method references.
- [Class assembly](class-assembly.md): lossless declarations and relocation limits.
- [Named classes](named-classes.md): aliases and typed DATA references.
- [Repeated method names](module-method-names.md): owner-qualified bindings.
- [Developer signing](developer-signing.md): signature coverage and own-key edits.
- [Function replacement](function-replacement.md): same-size arithmetic edit.
- [CODE labels and size changes](code-labels-and-relocation.md): appended bodies
  and method/line relocation.
- [DEBUG and line metadata](line-metadata.md): structured records and grow/shrink.
- [String address boundary](string-address-boundary.md): high-offset literals
  under raised simulator memory.
- [Control flow](control-flow.md): branches and exception-bearing functions.
- [Branch labels](branch-labels.md): relative references and loop edits.
- [Exception assembly](exception-assembly.md): shared boundaries and moved handlers.

## Projects and packages

- [Multi-file sources and barrels](multi-file-and-barrels.md): SDK project and
  archive comparisons.
- [Package formats](package-formats.md): ZIP/7z profiles and manifest signing.
- [Package reading](package-reading.md): member preservation and bounded extraction.
- [Package verification](package-verification.md): integrity and signing identity.
- [Barrel packing](barrel-packing.md): deterministic output and SDK compatibility.
- [IQ repacking](iq-repacking.md): preserved signatures and independent decoding.
- [IQ creation](iq-creation.md): own-key signing and simulator execution.

## Compiler

- [Char literals](compiler-char-literals.md): immediate encoding, source escapes,
  supplementary-character discrepancy, and raw IR bits.
- [Char methods](compiler-char-methods.md): conversion boundaries, typed values,
  and grouped simulator comparison.
- [Immutable weak results](compiler-immutable-weak.md): receiver-specific
  contracts, SDK checker discrepancy, and runtime comparison.
- [Boolean methods](compiler-boolean-methods.md): inherited contracts, mixed
  comparison arguments, and grouped simulator validation.
- [String methods](compiler-string-methods.md): method inventory, grouped
  simulator validation, base conversion, and UTF-8 index observations.
- [Number methods](compiler-number-methods.md): full method inventory, deferred
  prerequisites, and grouped simulator validation.
- [Nullable user contracts](compiler-nullable-contracts.md): union signatures,
  call validation, and matching SDK/generated application output.
- [Substring results](compiler-substring.md): nullable String validation and
  observed range normalization on fr265.
- [Nullable Number results](compiler-nullable-numbers.md): native search and
  conversion, null checks, and guarded consumers.
- [Typed Lang values](compiler-lang-values.md): String/Boolean arguments,
  results, and module/instance calls.
- [Native value calls](compiler-native-calls.md): inherited Number conversion
  and explicit receiver invocation.
- [Initial frontend](first-compiler.md): constant-return source and scaffold use.
- [Native/text pipeline](compiler-pipeline.md): IR/CODE parity and terminology.
- [Arithmetic](compiler-arithmetic.md): grouped expressions and lowering.
- [Locals](compiler-locals.md): slot mapping and repeated reads.
- [Module calls](compiler-calls.md): binding and observable call order.
- [Parameters and arguments](compiler-arguments.md): context, slots, nested calls.
- [Strings and printing](compiler-strings.md): DATA placement and result discard.
- [Terminal conditions](compiler-conditions.md): branches and local joins.
- [Nested conditions](compiler-nested-conditions.md): scope and continuing joins.
- [Reassignment](compiler-assignments.md): fixed types and read/write order.
- [While loops](compiler-loops.md): initialization and repeated execution.
- [Loop control](compiler-loop-control.md): nested break/continue targets.
- [Compilation units](compiler-unit.md): cross-file declarations and body binding.
- [Console application bootstrap](application-bootstrap.md): lifecycle/imports,
  sorted fields, and template-free PRG generation.
- [Void functions](compiler-void.md): startup, early return, and discarded results.
- [Boolean values](compiler-booleans.md): literals, stored decisions, mutable
  loop flags, equality, and printing in a source callback.
- [Classes and receiver calls](compiler-class.md): ownership and self calls.
