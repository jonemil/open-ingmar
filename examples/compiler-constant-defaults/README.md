# Constant defaults and runtime aggregates

[app.mc](app.mc) distinguishes nonempty scalar-literal Array defaults from
calculated elements, constant references, and empty Array/ByteArray storage.
Forward aliases capture Null for runtime slots; later aliases retain mutable
container identity. Computed Long/String initializers and module trace calls
check runtime evaluation order. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-constant-defaults
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id c3907c415fdf40139a859dd4fc430721 --entry MinimalConsoleApp.onStart \
  examples/compiler-constant-defaults/app.mc \
  scratch/compiler-constant-defaults/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-constant-defaults/unsigned.prg \
  scratch/compiler-constant-defaults/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-constant-defaults/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
and minimum API 5.0.0. Startup prints precede console attachment; module trace
prints appear at first access during `onStart`.
