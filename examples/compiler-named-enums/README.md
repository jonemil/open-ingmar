# Named enum contracts

[app.mc](app.mc) and [later.mc](later.mc) check nominal enum identities through
global/module declarations, typedefs, functions, class fields, arrays,
dictionaries, nullable reads, and lexical assertions. Enum values use Number
methods and arithmetic while retaining their identity at typed boundaries.
See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-named-enums
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id ffb2aaae51f241399b9604e4f2c70721 --entry MinimalConsoleApp.onStart \
  examples/compiler-named-enums/app.mc \
  examples/compiler-named-enums/later.mc \
  scratch/compiler-named-enums/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-named-enums/unsigned.prg \
  scratch/compiler-named-enums/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-named-enums/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with both source
files and minimum API 5.0.0. The Dictionary literal uses an explicit assertion
because this SDK loses the nominal enum key type when inferring its return type.
