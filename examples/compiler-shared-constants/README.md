# Shared constants

[app.mc](app.mc) checks global/module heap values, readonly bindings with mutable
contents, alias identity, owner-relative reads, ByteArray/WeakReference/Method
values, and lazy module initialization. In this simulator session, global startup
prints precede console attachment; [expected.txt](expected.txt) starts at `onStart`.

```sh
mkdir -p scratch/compiler-shared-constants
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 9df9279337df4bc79c190bd70f6d2121 --entry MinimalConsoleApp.onStart \
  examples/compiler-shared-constants/app.mc scratch/compiler-shared-constants/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-shared-constants/unsigned.prg \
  scratch/compiler-shared-constants/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-shared-constants/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Copy the [minimal console scaffold](../minimal-console/README.md) into scratch,
replace its source with this file, and set minimum API 5.0.0 before building.
