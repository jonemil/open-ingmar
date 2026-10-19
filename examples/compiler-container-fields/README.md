# Object and container fields

[app.mc](app.mc) exercises required/nullable Object fields and invariant typed
Array/Dictionary fields. It checks contextual literals, mutation and replacement,
Object class recovery, numeric aliases, stored callbacks, independent instances,
and strong ownership through field replacement and container enumeration.

API minimum: 5.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-container-fields
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 9c77dc1202384df19b7be6ddc92a48bc --entry MinimalConsoleApp.onStart \
  examples/compiler-container-fields/app.mc scratch/compiler-container-fields/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-container-fields/unsigned.prg \
  scratch/compiler-container-fields/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-container-fields/generated.prg
```

Compare the console with [expected.txt](expected.txt). See the
[field contract](../../docs/specs/compiler-classes.md#object-and-container-fields)
and [evidence](../../docs/investigations/compiler-container-fields.md).
