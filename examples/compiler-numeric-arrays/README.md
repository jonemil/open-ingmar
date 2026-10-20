# Long, Float, and Double arrays

[app.mc](app.mc) exercises invariant arrays of all three numeric types, with
nullable elements, constant DATA loads, computed construction and branches,
contextual empty literals, reads/writes, indexed updates, and native Array
methods. Repeated loads retain independent mutable contents; slices/reversals
preserve element values. SDK/generated `fr265` programs match
[expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-numeric-arrays
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 1.3.0 --app-id a8375ea49e4f41f88b347b783f6bde01 --entry MinimalConsoleApp.onStart examples/compiler-numeric-arrays/app.mc scratch/compiler-numeric-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-numeric-arrays/unsigned.prg scratch/compiler-numeric-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-numeric-arrays/generated.prg
```

The native mutation/search/slice/reverse methods require API 1.3.0. These arrays
retain the existing restrictions on sorting, broader callback consumers,
nested containers, and whole-array conditional joins. See the
[source contracts](../../docs/specs/frontend/containers.md#numeric-arrays).
