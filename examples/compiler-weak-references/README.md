# Ordinary weak references

[app.mc](app.mc) exercises Array/Dictionary weak references, live and expired
retrieval, temporary strong references, aliases, native observers, and function
arguments/results. The [expected output](expected.txt) matches SDK/generated
`fr265` programs. `get()` produces `Object or Null`; guarded Object values keep
their targets alive until their strong references are released.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-weak-references
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.2.0 --app-id 60704812f1f54be5b4f878534ca08e89 --entry MinimalConsoleApp.onStart examples/compiler-weak-references/app.mc scratch/compiler-weak-references/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-weak-references/unsigned.prg scratch/compiler-weak-references/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-weak-references/generated.prg
```

WeakReference and its `get()`/`stillAlive()` methods require API 1.2.0. Ordinary
Object results retain opaque contracts; concrete Array/Dictionary methods need
narrower types. See the
[source contracts](../../docs/specs/compiler-subset.md#ordinary-weak-references).
