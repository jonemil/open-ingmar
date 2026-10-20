# Symbol values

[app.mc](app.mc) exercises Symbol literals and identity, native methods, user
arguments/results, immutable weak values, and a native member name. Symbol IDs
are build-local: the [expected output](expected.txt) compares identities rather
than requiring SDK/generated IDs to match. Development symbol names are retained
for `toString()`.
`NullableSymbols` checks nullable arguments/results and conditional joins,
guarded methods/selectors, null comparisons, forwarding, loops, and local
snapshots retained after reassignment.
`SymbolContainers` checks nullable/non-null Arrays, Symbol Dictionary keys and
values, mutation, collection methods, enumeration snapshots, and bound native
member identities. Symbol-bearing literals use runtime construction.
`SymbolStore` checks required/nullable fields, Symbol and Null defaults, instance
isolation, mutable snapshots, and a stored selector. Metadata-only names are
retained for development reflection in both Object-rooted and AppBase fields.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-symbols
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.3.0 --app-id e576c7ce420f4b809264f37324f9fb19 --entry MinimalConsoleApp.onStart examples/compiler-symbols/app.mc scratch/compiler-symbols/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-symbols/unsigned.prg scratch/compiler-symbols/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-symbols/generated.prg
```

Symbol requires API 1.0.0; `weak()` requires 1.2.0 and `toNumber()` requires
2.3.0. Nullable Symbol and container storage retain these method requirements.
Pooled Symbol container payloads remain outside this specimen.
[Method values/invocation](../compiler-methods/README.md) have a separate specimen. See the
[source contracts](../../docs/specs/frontend/lang-values.md#symbol-values).
