# Dictionary values, lookup, and mutation

[app.mc](app.mc) checks `{}`, local copies/reassignment, Dictionary arguments and
results, chained `size`/`isEmpty` calls, and repeated construction in a loop.
Typed functions preserve invariant key/value annotations, including nullable
String values. Empty literals acquire their types from results, call arguments,
and reassignment. Typed references can pass to opaque Dictionary parameters.
Nonempty constants cover all supported key types, Number/String/null values,
duplicate keys, repeated loads, local replacement, and shared pool strings.
Indexed reads/writes and `get`/`put`/`hasKey`/`remove` exercise nullable lookup,
alias visibility, Null values versus missing keys, and fresh constant loads.
Printed markers check operand order, including conditional keys/RHS values.
Computed literals check duplicate effects, nullable values, Array-produced
scalars, short-circuit keys, and private construction across branches/loops.
`toString` checks empty/typed/opaque references, scalar keys and Null values,
mutation snapshots, chaining, and discarded results. Multi-entry checks avoid
assuming a String entry order.
`keys`/`values` check typed Array results, empty and nullable entries, membership
without assuming order, independent mutations, and effectful receivers.
SDK and generated `fr265` programs match [expected output](expected.txt).
The launch-state guard checks nullable callback typing; this launch supplies Null.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-dictionary
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 869aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-dictionary/app.mc scratch/compiler-dictionary/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-dictionary/unsigned.prg scratch/compiler-dictionary/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-dictionary/generated.prg
```

Inherited Object methods and wider value groups remain planned. See
[the source profile](../../docs/specs/compiler-subset.md#dictionary-values) and
[SDK evidence](../../docs/investigations/compiler-dictionary.md).
