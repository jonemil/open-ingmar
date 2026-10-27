# Recursive contracts

[app.mc](app.mc) uses recursive Array/Dictionary aliases, nullable values, and a
structural interface whose method returns the same interface. Recursive edges
retain their contracts through indexing, construction, calls, fields, and saved
IR. Compare [expected.txt](expected.txt).

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-recursive-contracts
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 40e139c0f867462d85d9d4282ca9271b --entry MinimalConsoleApp.onStart \
  examples/compiler-recursive-contracts/app.mc scratch/compiler-recursive-contracts/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-recursive-contracts/unsigned.prg \
  scratch/compiler-recursive-contracts/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-recursive-contracts/generated.prg
```

SDK reference: 9.2.0, `fr265`, strict checking, minimum API 5.0.0, `-O 0`.
Copy the [minimal console scaffold](../minimal-console/README.md) into ignored
scratch storage, replace its source with this file, set minimum API 5.0.0 and
`project.typecheck = strict`, then build with `scripts/build-example.sh`.
