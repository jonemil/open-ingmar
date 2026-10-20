# Lang.format

[app.mc](app.mc) checks erased Array arguments, heterogeneous and nullable
contents, source-object string conversion, Unicode, substitution order, and
left-to-right effects. Typed arrays retain their concrete element contracts.

API minimum: 3.1.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.
`Lang.format` itself requires API 1.0.0; this specimen also uses ByteArray.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-lang-format
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.1.0 \
  --app-id 9c77dc1202384df19b7be6ddc92a48bc --entry MinimalConsoleApp.onStart \
  examples/compiler-lang-format/app.mc scratch/compiler-lang-format/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-lang-format/unsigned.prg \
  scratch/compiler-lang-format/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-lang-format/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[module-call contract](../../docs/specs/sdk/profiles.md#native-module-and-ui-profiles)
and [evidence](../../docs/investigations/compiler-lang-format.md).
