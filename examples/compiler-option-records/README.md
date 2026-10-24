# Structural option records

[app.mc](app.mc) checks optional Symbol fields, missing/extra keys, nested
records, inferred literal schemas, stable field mutation, class storage, and
ByteArray conversion using returned or reused options.
Compare [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-option-records
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.1.0 \
  --app-id d06b2552a0784ae99dd65e4b1c329ef1 \
  --entry MinimalConsoleApp.onStart examples/compiler-option-records/app.mc \
  scratch/compiler-option-records/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-option-records/unsigned.prg \
  scratch/compiler-option-records/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-option-records/generated.prg
```

Reference: SDK 9.2.0, strict checking, `fr265`, minimum API 3.1.0.
