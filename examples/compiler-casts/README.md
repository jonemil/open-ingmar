# Lexical assertions

[app.mc](app.mc) asserts typed Arrays/Dictionaries, option records, tuples,
nullable/native unions, structural/native interfaces, source-class containers
and unions, and a Method signature. Record aliases retain shared mutation.
[Expected output](expected.txt) checks the original runtime values remain usable.

From the repository root:

```sh
mkdir -p scratch/compiler-casts
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 91645fbd41764dba942e4de3adf060ed \
  --entry MinimalConsoleApp.onStart examples/compiler-casts/app.mc \
  scratch/compiler-casts/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-casts/unsigned.prg \
  scratch/compiler-casts/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-casts/generated.prg
```

SDK reference: 9.2.0, strict checking, `fr265`, minimum API 5.0.0.
For reproduction, copy the [minimal console scaffold](../minimal-console/README.md)
into ignored scratch storage and replace `source/App.mc` with this `app.mc`.
Set the manifest minimum API to 5.0.0 and `project.typecheck = strict` in its
Jungle file, then build that directory with `scripts/build-example.sh`.
Assertions emit no conversion or runtime type check. The explicit `instanceof`
in the union case supplies its own runtime test.
