# Source structural interfaces

[app.mc](app.mc) checks named and anonymous interface contracts, two unrelated
source implementations, nullable parameters, native scalar implementations, and
mutable fields. Reads/writes, compound assignments, increments, nullable
snapshots, and branching values preserve receiver identity and evaluation order.

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-interfaces
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id 937df7506e9b4ed7ae2a8f0196dded52 --entry MinimalConsoleApp.onStart \
  examples/compiler-interfaces/app.mc scratch/compiler-interfaces/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-interfaces/unsigned.prg \
  scratch/compiler-interfaces/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-interfaces/generated.prg
```

Compare the full console with [expected.txt](expected.txt). Reference environment:
SDK 9.2.0, fr265, minimum API 3.0.0 and target 5.2.0. See
[source interface contracts](../../docs/specs/frontend/README.md#source-interface-types)
and [SDK evidence](../../docs/investigations/compiler-structural-interfaces.md).
