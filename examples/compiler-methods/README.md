# Bound Method values

[app.mc](app.mc) binds native methods through Object `method(Symbol)`, invokes
zero/one/two-argument callbacks, and observes receiver lifetime, aliases, hashes,
nullable/Void results, returned containers, and nullable Method ownership.
`MethodStore` and AppBase fields exercise required/nullable callback storage,
local snapshots, replacement, and release. `MethodArrays` checks typed/nullable
elements, reverse/slice copies, mutation, and ordered literal evaluation.
`MethodDictionaries` checks Method keys/values, nullable lookup, enumeration
ownership, and key/value evaluation order. The
[expected output](expected.txt) matches SDK/generated `fr265` programs.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-methods
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id 949ff3758b28492cb54821459b9c90fa --entry MinimalConsoleApp.onStart examples/compiler-methods/app.mc scratch/compiler-methods/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-methods/unsigned.prg scratch/compiler-methods/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-methods/generated.prg
```

Method binding and invocation require API 1.0.0. This specimen also uses weak
references from 1.2.0 and native Array methods from 1.3.0. Erased invocation
returns `Object or Null`; save and guard results before Object member calls.
Concrete callback signatures and Method constructors remain deferred.
[Source callbacks](../compiler-source-methods/README.md) exercise direct
instance/self binding. See the
[source contracts](../../docs/specs/frontend/README.md#method-values-and-invocation).
