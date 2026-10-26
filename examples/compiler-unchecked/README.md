# Unchecked value operations

[app.mc](app.mc) deliberately mismatches annotations at returns, arguments,
storage, arithmetic operands, index keys/receivers, member/field receivers, and
conditions. Field cases include reads, writes, compound updates, and a
conditional RHS. Runtime inputs still support the executed operations. Checking off preserves their actual values;
annotations do not cause conversions. Compare [expected.txt](expected.txt).

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-unchecked
cargo run -p ciq-compiler -- --unit --emit prg --type-check off \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 58cdd61b9fd04ab8ba564323684a418b --entry MinimalConsoleApp.onStart \
  examples/compiler-unchecked/app.mc scratch/compiler-unchecked/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-unchecked/unsigned.prg \
  scratch/compiler-unchecked/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-unchecked/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`.
Copy the [minimal console scaffold](../minimal-console/README.md) into ignored
scratch storage, replace its source with this file, set the manifest minimum API
to 5.0.0 and `project.typecheck = off` in its Jungle file, then build with
`scripts/build-example.sh`. See the
[checking policy](../../docs/specs/frontend/README.md#type-checking-policy)
for the supported unchecked profile and remaining restrictions.
