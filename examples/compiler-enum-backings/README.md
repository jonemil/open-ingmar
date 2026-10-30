# Named enum backing types

[app.mc](app.mc) and [types.mc](types.mc) exercise nominal enum identities with
Float, String, mixed, Null, container, Boolean, and source-object backing types.
Mixed-enum functions use only the matching runtime member; default SDK checking
permits a partial native match. Containers retain invariant enum identities.
See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-enum-backings
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 7392fa5b529049abbb3e85c6f6acd928 --entry MinimalConsoleApp.onStart \
  examples/compiler-enum-backings/app.mc examples/compiler-enum-backings/types.mc \
  scratch/compiler-enum-backings/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-enum-backings/unsigned.prg \
  scratch/compiler-enum-backings/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-enum-backings/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0.
