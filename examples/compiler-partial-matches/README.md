# Partial native type matches

[app.mc](app.mc) passes Number-or-String values through Number arguments,
results, parameter/field stores, container construction, and a native call.
It also narrows and widens Array views, matches nested Array and Dictionary
parameters, and checks shared mutations through the original and narrowed
references. Runtime inputs select matching alternatives. The actual types remain
in IR; compilation inserts no casts or runtime guards. Other alternatives may
still fail at runtime. Compare the full console with [expected.txt](expected.txt).

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-partial-matches
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 67912fb670174936b4f1cb9c9de3fd41 --entry MinimalConsoleApp.onStart \
  examples/compiler-partial-matches/app.mc scratch/compiler-partial-matches/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-partial-matches/unsigned.prg \
  scratch/compiler-partial-matches/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-partial-matches/generated.prg
```

Informative checking reports these boundaries; strict checking rejects them.
See [checking policies](../../docs/specs/frontend/README.md#type-checking-policy).
For the SDK reference, copy the [minimal console scaffold](../minimal-console/README.md)
into ignored scratch storage, replace `source/App.mc` with this source, and set
`project.typecheck = informative` in its Jungle file and the manifest minimum API to
5.0.0. Reference: SDK 9.2.0, `fr265`, optimization disabled.
