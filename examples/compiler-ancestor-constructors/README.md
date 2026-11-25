# Ancestor constructor assignments

[app.mc](app.mc) initializes required Number, String, and Array fields through a
three-class constructor chain. It checks module aliases, parent-before-child
assignments, exposure after completion, repeated ordinary initialization without
replaying allocation defaults, and virtual calls after descendant fields are
ready. A final chain establishes root fields before invoking an intermediate
initializer that reads them. See [expected.txt](expected.txt).

BranchChild and its parent assign fields on both paths and return early on one
path. SnapshotRoot calls a virtual method from its allocation hook. The child
override reads the child's literal metadata default during that hook.

```sh
mkdir -p scratch/compiler-ancestor-constructors
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id b67c1691c9a5422a9d3202a0c016870c --entry MinimalConsoleApp.onStart \
  examples/compiler-ancestor-constructors/app.mc scratch/compiler-ancestor-constructors/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-ancestor-constructors/unsigned.prg \
  scratch/compiler-ancestor-constructors/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-ancestor-constructors/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 3.0.0, `-O 0`.
Use the [minimal console scaffold](../minimal-console/README.md) with this source.
