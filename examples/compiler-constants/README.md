# Scalar constants

[app.mc](app.mc) and [values.mc](values.mc) cover forward/cross-file references,
owner scope, qualified and self references, non-heap literals, pure Number
expressions, receiver effects, and lexical shadowing. See
[expected.txt](expected.txt) for console output.

From the repository root, using new output paths:

```sh
mkdir -p scratch/compiler-constants
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 7a2be25116cc441ea611855fba1de319 --entry MinimalConsoleApp.onStart \
  examples/compiler-constants/app.mc examples/compiler-constants/values.mc scratch/compiler-constants/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-constants/unsigned.prg \
  scratch/compiler-constants/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-constants/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`. Copy the
[minimal console scaffold](../minimal-console/README.md) into scratch, replace its
source with both files, set minimum API 5.0.0, and build with
`scripts/build-example.sh`. Unannotated source constants expose inferred values;
this example uses the default SDK checking policy.
