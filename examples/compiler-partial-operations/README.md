# Partial native operations

[app.mc](app.mc) uses Number-or-String operands, Array indices, and native/source
member receivers. Source dispatch covers missing members, Null, mixed native
receivers, distinct result contracts, and discarded Void calls. Matching inputs execute without casts or extra guards. Unary plus
also receives a String, checking that its no-op lowering preserves the value.
Informative checking reports uncertainty; strict checking rejects it. Other
runtime alternatives may fail; known disjoint operations remain compile errors.
Compare the complete console with [expected.txt](expected.txt).

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-partial-operations
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 2d97be77ab3d483e94eaf08d10489a9a --entry MinimalConsoleApp.onStart \
  examples/compiler-partial-operations/app.mc scratch/compiler-partial-operations/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-partial-operations/unsigned.prg \
  scratch/compiler-partial-operations/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-partial-operations/generated.prg
```

SDK reference: 9.2.0, `fr265`, informative checking, minimum API 5.0.0, `-O 0`.
Copy the [minimal console scaffold](../minimal-console/README.md) into ignored
scratch storage, replace `source/App.mc` with this source, set the manifest
minimum API to 5.0.0 and `project.typecheck = informative` in its Jungle file.
Build with `scripts/build-example.sh`. See the
[checking policy](../../docs/specs/frontend/README.md#type-checking-policy).
