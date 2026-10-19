# Investigation index

Browse by question or component. Records describe experiments at their stated
SDK/version and scope; they are not current feature inventories. Use
[the plan](../plan.md) for upcoming work and
[research conventions](../research.md) when recording evidence.

- [Core Lang completion](core-lang-completion.md): remaining contract inventory
  and the boundary before broader SDK expansion.
- [Runtime object typing](compiler-object-tests.md): class/member predicates,
  branch refinement, lexical casts, and nullable receiver failure.
- [Object-rooted classes](compiler-root-objects.md): implicit/explicit bases,
  return-only allocation hooks, native Object imports, and receiver state.
- [Source comparators](compiler-comparators.md): exact structural signatures,
  runtime callback identity, stateful ordering, and rejected parameter variants.
- [Default Array sorting](compiler-array-sort.md): comparable scalar types,
  shared mutation, Void results, and Comparator interface declarations.
- [Wider numeric Dictionaries](compiler-numeric-dictionaries.md): numeric
  keys/values, signed-zero keys, and separate NaN key evidence.
- [Numeric boundaries](compiler-numeric-boundaries.md): isolated division
  failures and floating-to-integer conversion limits.
- [Object observers](compiler-object-observers.md): Array identity/hashes,
  scalar/container comparisons, and native dispatch order.
- [Ordinary weak references](compiler-weak-references.md): retrieval, expiration,
  opaque Object results, and hidden temporary ownership.
- [Symbols](compiler-symbols.md): value identity, native methods, name binding,
  and development reflection metadata.
- [Methods](compiler-methods.md): bound-receiver ownership, erased invocation,
  nullable/Void results, and runtime lookup failures.
- [Wider numeric arrays](compiler-numeric-arrays.md): Long/Float/Double DATA
  records, nullable elements, construction, and native methods.
- [Wider String/Char operators](compiler-wide-operators.md): numeric operand
  rules, indexed compounds, and immutable Char weak results.
- [Float/Double values and methods](compiler-floating-values.md): floating
  literals, promotion, conversion, nullable parsing, and instruction evidence.

- [Example toolchain audit](example-toolchain-audit.md): coverage of all example
  projects, unsupported opcode/record groups, and reconstruction checks.

- [Long values](compiler-long-values.md): signed literals, 64-bit instruction
  operands, typed value transport, and simulator checks.
- [Dictionary](compiler-dictionary.md): empty allocation, native observers,
  reference calls, and outstanding constant-literal encoding.
- [Object containers](compiler-object-containers.md): erased mixed storage,
  nominal enum erasure, class recovery, and strong reference ownership.
- [Lang.format](compiler-lang-format.md): mixed substitutions, module dispatch,
  evaluation order, and missing-parameter runtime failure.
- [Comparison Object arguments](compiler-comparison-objects.md): declared
  domains, dynamic scalar comparisons, and isolated runtime type errors.
- [Numeric aliases](compiler-numeric-unions.md): arithmetic promotion through
  Numeric/Integer/Decimal signatures, fields, containers, and nullable guards.
- [ByteArray](byte-arrays.md): compact constants, zero-filled allocation,
  indexed stores, and structured reconstruction in the simulator.
- [Operator coverage](compiler-operator-coverage.md): logical keyword aliases,
  Number inversion, and remaining prerequisite boundaries.
- [Conditional expressions](compiler-conditionals.md): lazy arms, precedence,
  strict result-type joins, runtime truthiness, and VM stack joins.
- [Increment/decrement](compiler-increments.md): statement-only SDK syntax,
  direct arithmetic lowering, target capture, and Number boundary behavior.
- [Compound targets](compiler-compound-targets.md): field/indexed reads,
  captured locations, RHS mutations, and read/write bounds failures.
- [Local compounds](compiler-compound-locals.md): read/binary/write lowering,
  evaluation order, type preservation, and simulator comparison.
- [Number unary plus](compiler-number-plus.md): identity lowering, bounded
  type contracts, SDK prefix syntax, and simulator comparison.
- [Number bit operations](compiler-number-bits.md): instruction bytes,
  unusual precedence, signed shifts, and count-boundary probes.
- [Char/View writes](compiler-typed-array-writes.md): invariant element
  contracts, shared references, and bounds failures.
- [Number-array writes](compiler-array-writes.md): operand order, alias
  mutation, saved IR validation, and bounds failures.

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
- [Null-check opcodes](null-check-opcodes.md): null equality/inequality,
  structured reconstruction, and simulator results.
- [Stacked member get](member-get-opcode.md): compound field updates, stacked
  symbols, and structured reconstruction.
- [Inversion opcode](invert-opcode.md): shared Number/Boolean encoding and
  structured reconstruction.
- [Array-fill opcode](array-fill-opcode.md): retaining element stores,
  evaluation order, and write-time bounds checks.
- [Constant arrays](constant-arrays.md): DATA framing, scalar elements, array
  copy behavior, and editable load references.
- [Natural lowering](natural-lowering.md): source/IR representation review and
  direct Boolean/null instruction selection.
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

- [Number comparisons](compiler-number-comparisons.md): ordering/inequality
  instructions, operand order, and non-null proofs.

- [Number products](compiler-number-products.md): multiplication/division/remainder,
  signs, precedence, operand effects, and runtime/process failures.

- [Input counter](compiler-input-counter.md): native delegate metadata,
  View/delegate results, and matching SDK/generated touchscreen behavior.

- [Source-reference fields](compiler-reference-fields.md): constructor arguments,
  initialized reference slots, and retained state through shutdown.
- [Source-class signatures](compiler-source-signatures.md): exact reference
  contracts through module/self calls and saved IR.
- [Source object calls](compiler-object-calls.md): explicit receivers, aliases,
  same-class reassignment, and source/SDK runtime comparison.
- [Number instance fields](compiler-fields.md): field encoding, current-instance
  mutation, independent defaults, and a displayed counter.
- [Visible source View](compiler-visible-view.md): native inheritance, typed
  drawing callbacks, font constants, rendered output, and shutdown observations.
- [Source initial View](compiler-initial-view.md): positional callback results,
  source DATA binding, and runtime startup.
- [Native Views](compiler-native-views.md): object construction, typed references,
  invariant arrays, and constructor API levels.
- [Empty arrays](compiler-empty-arrays.md): contextual construction, aliases,
  sized Null-filled allocation, typed calls, and allocation API requirements.
- [Number-array literals](compiler-number-array-literals.md): inferred elements,
  concrete saved-IR contracts, and call order.
- [Number arrays](compiler-number-arrays.md): UTF-8 conversion, numeric element
  contracts, invariant mutation, and API introduction boundaries.
- [Array literals](compiler-array-literals.md): typed construction, element
  effects, allocation ordering, and append-based initialization.
- [Native statements](compiler-native-statements.md): discarded call results,
  stack cleanup, nullable results, and operand effects.
- [Array mutation](compiler-array-mutation.md): bulk append, removals, generic
  argument contracts, shared references, and self-appending.
- [Array append](compiler-array-add.md): mutation, typed self result, aliases,
  and receiver/argument evaluation order.
- [Array transforms](compiler-array-transforms.md): generic element
  substitution, reverse/slice bounds, and SDK/generated output comparison.
- [Array reads](compiler-array-reads.md): instruction encoding, operand order,
  nullable index guards, and fatal bounds errors.
- [Char arrays](compiler-char-arrays.md): element contracts, read-only methods,
  API boundaries, and Unicode conversion.
- [Logical values](compiler-logical-values.md): temporary storage, operand/call
  order across joins, and shared IR validation.
- [Short-circuit conditions](compiler-short-circuit.md): conditional effects,
  precedence, loops, and negated null proofs.
- [Boolean negation](compiler-boolean-negation.md): precedence, repeated prefixes,
  and single operand evaluation in complete applications.
- [Number negation](compiler-number-negation.md): unary lowering, minimum
  wraparound, and SDK literal postfix discrepancy.
- [Number literals](compiler-number-literals.md): signed immediates, width
  boundaries, and parsing limits.
- [String operators](compiler-string-operators.md): concatenation conversions,
  reference identity, and grouped simulator comparison.
- [Char operators](compiler-char-operators.md): operand/result contracts and
  grouped simulator comparison.
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
