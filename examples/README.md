# Monkey C examples

Application/library directories are standalone Connect IQ projects containing
their own manifest, Jungle configuration, source, resources, and build
instructions. These are independently written reference inputs for Garmin's SDK
and future Rust toolchain experiments.

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
