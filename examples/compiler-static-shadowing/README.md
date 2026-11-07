# Static-field redeclarations

[app.mc](app.mc) covers separate parent/child static storage, independent lazy
initialization, differing types, protected/private access, readonly changes,
and distinct container values. Static bodies are called through their declaring
class; inherited bodies retain the runtime receiver behavior described in the
[static-member specification](../../docs/specs/frontend/README.md#inherited-static-members).
Compare [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-static-shadowing
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-static-shadowing/app.mc scratch/compiler-static-shadowing/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-static-shadowing/unsigned.prg \
  scratch/compiler-static-shadowing/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-static-shadowing/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
