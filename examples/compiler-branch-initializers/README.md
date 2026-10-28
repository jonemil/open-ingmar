# Conditional and short-circuit initialization

[app.mc](app.mc) checks ordered global/module initializers containing `?:`,
`&&`/`and`, and `||`/`or`. Trace calls reveal selected operands and nested branch
order. Unselected zero division remains unevaluated. Array aliases retain their
identity, early reads retain Null, and a conditional native constant keeps its
NumberFormat type. See [expected.txt](expected.txt).

```sh
mkdir -p scratch/compiler-branch-initializers
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 5.0.0 \
  --app-id 8c81c02fe8af422d8535691cf0540721 --entry MinimalConsoleApp.onStart \
  examples/compiler-branch-initializers/app.mc \
  scratch/compiler-branch-initializers/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-branch-initializers/unsigned.prg \
  scratch/compiler-branch-initializers/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-branch-initializers/generated.prg
```

SDK reference: 9.2.0, `fr265`, minimum API 5.0.0, `-O 0`, default checking policy.
Use the [minimal console scaffold](../minimal-console/README.md) with this source
and minimum API 5.0.0. Global trace prints precede console attachment; module
trace prints appear at its first access during `onStart`.
