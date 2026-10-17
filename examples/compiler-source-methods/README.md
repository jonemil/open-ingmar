# Source receiver callbacks

[app.mc](app.mc) binds source methods through a source instance, explicit
`self`, and an opaque Object with a dynamic selector. Calls share the original
receiver's fields and keep that receiver alive after its variable is replaced.
The [expected output](expected.txt) matches SDK/generated `fr265` programs.

From the repository root:

```sh
mkdir -p scratch/compiler-source-methods
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 949ff3758b28492cb54821459b9c90fa --entry MinimalConsoleApp.onStart examples/compiler-source-methods/app.mc scratch/compiler-source-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-source-methods/unsigned.prg scratch/compiler-source-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-source-methods/generated.prg
```

Bindings remain erased Methods: invocation returns `Object or Null` and does
not check the target signature. See the
[source contract](../../docs/specs/compiler-subset.md#method-values-and-invocation).
