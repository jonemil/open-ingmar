# Typed Lang values across functions

[app.mc](app.mc) supplies a source startup callback and String/Boolean instance
methods. [text.mc](text.mc) supplies typed module functions. Calls preserve
their result types through chained native calls, conditions, and printing.
The `first` and `second` markers check left-to-right argument evaluation;
statement calls discard both String and Void results.

From the repository root:

```sh
mkdir -p scratch/compiler-lang-values
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id 745e257013bc482ebd856d45ce049fe0 --entry LangApp.onStart \
  examples/compiler-lang-values/app.mc examples/compiler-lang-values/text.mc \
  scratch/compiler-lang-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-lang-values/unsigned.prg \
  scratch/compiler-lang-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-lang-values/generated.prg
```

Expected output, confirmed on SDK 9.2.0 / fr265:

```text
MIXED
mixed
false
first
second
FIRST
3
true
false
printed
true
discarded
```

See [function contracts](../../docs/specs/frontend/types.md#types-and-value-contracts)
for annotation syntax and defaults, [unit validation][unit] for call checking,
and [runtime evidence][evidence] for the SDK comparison. General dynamic type
inference, nullable user results, and other Lang types remain unsupported.

[unit]: ../../docs/specs/frontend/declarations.md
[evidence]: ../../docs/investigations/compiler-lang-values.md
