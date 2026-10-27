# Monkey C examples

Application/library directories are standalone Connect IQ projects containing
their own manifest, Jungle configuration, source, resources, and build
instructions. These are independently written reference inputs for Garmin's SDK
and future Rust toolchain experiments.

- [compiler-unchecked](compiler-unchecked/README.md): unchecked value boundaries,
  operators, index keys, and conditions with explicit saved IR assertions.
- [compiler-partial-operations](compiler-partial-operations/README.md): partial
  operator, index-key, and native member-receiver unions.
- [compiler-partial-matches](compiler-partial-matches/README.md): gradual native
  union matches at call, return, storage, and container-element boundaries.
- [compiler-casts](compiler-casts/README.md): lexical container, record, interface,
  union, and Method assertions, shared references, and single operand evaluation.
- [compiler-option-records](compiler-option-records/README.md): optional
  Symbol-keyed records, nested fields, mutation, and ByteArray conversion calls.
- [compiler-object-tests](compiler-object-tests/README.md): runtime class and
  member predicates, refinement, lexical casts, and evaluation order.
- [compiler-tuples](compiler-tuples/README.md): positional/nested/source slots,
  typed construction, fields, callbacks, indexing, and evaluation order.
- [compiler-literal-inference](compiler-literal-inference/README.md): empty,
  homogeneous, computed, Symbol, and source literals with mutable slot inference.
- [compiler-tuple-interfaces](compiler-tuple-interfaces/README.md): source-class
  interface witnesses in tuple slots, nested containers, fields, and callbacks.
- [compiler-tuple-methods](compiler-tuple-methods/README.md): tuple copies and
  slices, search, default/custom sorting, bound methods, and weak handles.
- [compiler-source-arrays](compiler-source-arrays/README.md): exact source
  elements, unions, typed mutation/search, custom sorting, and source shape loss.
- [compiler-nested-arrays](compiler-nested-arrays/README.md): nested container
  contracts, recursive literals, shared inner references, and shallow copies.
- [ir-any](ir-any/README.md): hand-authored dynamic IR, saved-stage compilation,
  and an SDK source comparison for Any calls, arithmetic, indexing, and narrowing.
- [compiler-root-objects](compiler-root-objects/README.md): Object-rooted source
  construction, independent fields, observers, callbacks, and comparators.
- [compiler-comparators](compiler-comparators/README.md): checked source
  Comparator implementations, stateful runtime dispatch, and custom sorting.
- [compiler-array-sort](compiler-array-sort/README.md): default scalar sorting,
  shared mutation, copies, and edge-sized inputs.
- [compiler-object-containers](compiler-object-containers/README.md): mixed
  Object storage, nullable snapshots, class recovery, and reference lifetimes.
- [compiler-object-sort](compiler-object-sort/README.md): Object default ordering
  and custom Comparator sorting of non-null reference elements.
- [compiler-lang-format](compiler-lang-format/README.md): value-returning native
  module calls, erased Array arguments, substitutions, and evaluation order.
- [compiler-comparison-objects](compiler-comparison-objects/README.md): declared
  Object arguments for comparison/equality and runtime numeric dispatch.
- [compiler-numeric-containers](compiler-numeric-containers/README.md): numeric
  alias Arrays and Dictionary keys/values, nullable reads, and mixed constants.
- [compiler-numeric-fields](compiler-numeric-fields/README.md): numeric alias
  fields, constructor initialization, nullable snapshots, and instance isolation.
- [compiler-scalar-fields](compiler-scalar-fields/README.md): concrete scalar
  fields, exact widths, nullable snapshots, and initialization contracts.
- [compiler-container-fields](compiler-container-fields/README.md): Object
  erasure, typed container fields, conditional writes, and reference ownership.
- [compiler-numeric-unions](compiler-numeric-unions/README.md): Numeric,
  Integer, and Decimal arithmetic, nullable guards, and runtime promotion.
- [compiler-typedefs](compiler-typedefs/README.md): scoped and cross-file aliases,
  containers, nullable source identities, fields, and casts.
- [compiler-recursive-contracts](compiler-recursive-contracts/README.md): guarded
  aliases, recursive interface calls, nested containers, and field witnesses.
- [compiler-interfaces](compiler-interfaces/README.md): named/anonymous structural
  contracts, dynamic methods, mutable fields, snapshots, and evaluation order.
- [compiler-callbacks](compiler-callbacks/README.md): fixed Method signatures,
  typed binding, invocation, nullable storage, containers, and nested callbacks.
- [compiler-byte-arrays](compiler-byte-arrays/README.md): constant/computed
  literals, allocation, byte indexing, mutation, observers, and callbacks.
- [compiler-source-methods](compiler-source-methods/README.md): source/self
  callbacks, shared fields, and retained receivers.
- [compiler-numeric-dictionaries](compiler-numeric-dictionaries/README.md): wider
  numeric keys/values, nullable values, construction, and generic methods.
- [compiler-numeric-arrays](compiler-numeric-arrays/README.md): Long/Float/Double
  constants, nullable elements, mutation, and generic Array methods.
- [compiler-wide-operators](compiler-wide-operators/README.md): String/Char
  operators with wider numeric values and immutable Char weak results.
- [compiler-float-values](compiler-float-values/README.md): Float/Double values,
  promotion, native methods, conversions, and nullable parsing.
- [compiler-boolean-arrays](compiler-boolean-arrays/README.md): Boolean constants,
  typed mutation/native methods, and conditional construction.
- [compiler-nullable-arrays](compiler-nullable-arrays/README.md): nullable Number
  and String elements, contextual construction, guarded reads, and mutation.
- [compiler-string-arrays](compiler-string-arrays/README.md): pooled String
  constants, ordered construction, typed reads/writes, and Array methods.
- [compiler-dictionary](compiler-dictionary/README.md): typed/opaque Dictionaries,
  scalar/computed literals, nullable lookup, mutation, and native methods.
- [compiler-conditionals](compiler-conditionals/README.md): lazy conditional
  expressions, typed joins, operand order, nullable results, and shared arrays.
- [compiler-increments](compiler-increments/README.md): Number updates and Char
  increment statements with checked local, field, and indexed targets.
- [compiler-compound-targets](compiler-compound-targets/README.md): field/indexed
  updates, captured locations, and RHS mutation order.
- [compiler-compound-locals](compiler-compound-locals/README.md): local compound
  updates, scalar contracts, loops, and staged operands.
- [compiler-number-plus](compiler-number-plus/README.md): checked Number
  identity, postfix precedence, guarded operands, and single evaluation.
- [compiler-number-bits](compiler-number-bits/README.md): Number bit operations,
  signed shifts, precedence, and operand order.
- [compiler-typed-array-writes](compiler-typed-array-writes/README.md): Char
  replacement and shared native/source View references.
- [compiler-array-writes](compiler-array-writes/README.md): Number element
  replacement, alias visibility, and operand order.
- [compiler-native-calls](compiler-native-calls/README.md): inherited Number
  conversion on local and parameter receivers.
- [compiler-fields](compiler-fields/README.md): instance-owned Number state
  initialized independently and rendered across View updates.
- [compiler-object-calls](compiler-object-calls/README.md): calls through local
  source View references, aliases, and same-class reassignment.
- [compiler-source-signatures](compiler-source-signatures/README.md): source View
  parameter/result contracts, factories, and cross-file reference forwarding.
- [compiler-number-comparisons](compiler-number-comparisons/README.md): ordering,
  inequality, and direct non-null branch proofs.
- [compiler-number-products](compiler-number-products/README.md): signed Number
  multiplication/division/remainder, precedence, and operand order.
- [compiler-input-counter](compiler-input-counter/README.md): View/delegate
  results, retained state, and Boolean selection callbacks.
- [compiler-reference-fields](compiler-reference-fields/README.md): constructor
  arguments and retained View references across lifecycle callbacks.
- [compiler-probe](compiler-probe/README.md): a standalone function input to our
  Rust frontend, using Hello World scaffolding for execution.
- [hello-world](hello-world/README.md): minimal watch application showing a
  greeting and a numeric probe.
- [minimal-console](minimal-console/README.md): one target function calling
  `println("Hello!")`, with basic watch-app lifecycle scaffolding and no custom
  rendering.
- [multi-file](multi-file/README.md): three source files sharing a module
  function, with console/display `46`.
- [module-names](module-names/README.md): two modules with the same `answer()`
  spelling, returning `21` and `22`.
- [compiler-native-calls](compiler-native-calls/README.md): inherited Number
  conversion on local and parameter receivers.
- [compiler-probe](compiler-probe/README.md): standalone inputs for the Rust
  frontend, including literal returns, grouped integer arithmetic, initialized
  locals, external module calls, and integer parameters/arguments.
- [number-barrel](number-barrel/README.md): the numeric module packaged as a
  source library.
- [barrel-app](barrel-app/README.md): the library consumed through an archive or
  source-project Jungle.

Application example build scripts use
[scripts/build-example.sh](../scripts/build-example.sh) for the SDK, Java,
device-data, and key setup. Output names follow the example directory name.

Keep generated artifacts in each project's ignored `build/` directory. Signing
keys and SDK installations stay outside the examples.

- [Void startup and module calls](compiler-void/README.md): complete generated
  application with effect-only calls, bare returns, and void fallthrough.

- [Source class and receiver calls](compiler-class/README.md): AppBase ownership
  and nested instance-method calls in a complete generated application.

- [Source application lifecycle](compiler-lifecycle/README.md): initialization,
  native startup callback typing, and a null-state check across two source files.
