# Object observers on containers

[app.mc](app.mc) checks Array reference equality and stable hashes across aliasing
and mutation, comparisons with other scalar/container types, and left-to-right
receiver/argument evaluation. SDK/generated `fr265` programs match the
[expected output](expected.txt); actual hash numbers are runtime supplied.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-object-observers
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 3.0.0 --app-id 25f7f4d37da1493da03b7b020a0b278a --entry MinimalConsoleApp.onStart examples/compiler-object-observers/app.mc scratch/compiler-object-observers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-object-observers/unsigned.prg scratch/compiler-object-observers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-object-observers/generated.prg
```

The inherited observers require API 1.0.0; this specimen also uses native Array
mutation/slice/reverse methods from 1.3.0 and nullable slice bounds. See the
[shared equality contracts](../../docs/specs/frontend/lang-values.md#object-equality-and-container-observers).
Sorting remains deferred; [bound Methods](../compiler-methods/README.md) and
ordinary weak references have
[a separate specimen](../compiler-weak-references/README.md).
