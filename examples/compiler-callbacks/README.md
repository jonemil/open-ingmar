# Typed callback signatures

[app.mc](app.mc) checks callback arguments/results, source/native and self
binding, nullable snapshots, required fields, Void invocation, exact source
results, callback containers, erased Method boundaries, and nested callbacks.

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-callbacks
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 3.0.0 --app-id 31cb64d7bbd44c7bbd12064f5f6fbb22 --entry MinimalConsoleApp.onStart examples/compiler-callbacks/app.mc scratch/compiler-callbacks/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-callbacks/unsigned.prg scratch/compiler-callbacks/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-callbacks/generated.prg
```

Compare the full console with [expected.txt](expected.txt). Reference environment:
SDK 9.2.0, fr265, minimum API 3.0.0 and target 5.2.0. See the
[callback contracts](../../docs/specs/frontend/README.md#typed-callbacks).
