# Object and reference Array sorting

[app.mc](app.mc) checks default sorting through Object arrays and custom
Comparator ordering of mixed values, source objects, ByteArrays, Symbols,
Methods, WeakReferences, and Views. It exercises alias-visible mutation,
independent copies, empty/singleton arrays, callback state, receiver/argument
order, and reference identity/lifetimes.

API minimum: 5.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-object-sort
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 9c77dc1202384df19b7be6ddc92a48bc --entry MinimalConsoleApp.onStart \
  examples/compiler-object-sort/app.mc scratch/compiler-object-sort/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-object-sort/unsigned.prg \
  scratch/compiler-object-sort/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-object-sort/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[sorting contracts](../../docs/specs/frontend/README.md#default-array-sorting)
and [evidence](../../docs/investigations/compiler-object-sort.md).
