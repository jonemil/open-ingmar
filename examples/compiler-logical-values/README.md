# Logical value results

[app.mc](app.mc) and [flags.mc](flags.mc) exercise Boolean results from `&&` and
`||` in initialization, assignment, returns, calls, comparisons, and native
receivers. Effectful helpers show evaluation order across joins.
API context: 1.0.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-logical-values
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id 4c3717ac341640d88c77d6a493caef25 --entry start \
  examples/compiler-logical-values/app.mc \
  examples/compiler-logical-values/flags.mc \
  scratch/compiler-logical-values/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-logical-values/unsigned.prg \
  scratch/compiler-logical-values/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-logical-values/generated.prg
```

Expected output, with no `bad-*` lines:

```text
false
true
false
true
true
true
left
right
xtrue
receiver
native-argument
false
arg-first
arg-middle
arg-right
arg-last
xtrue7
stmt-first
stmt-second
3false
false
true
loop
loop
3
```

Values crossing a branch use temporary locals. Their slots count toward the
ordinary frame limit. A stored Boolean result does not carry a null proof for
its original operands. See the [source contract][source] and [evidence][evidence].

[source]: ../../docs/specs/frontend/README.md#short-circuit-expressions
[evidence]: ../../docs/investigations/compiler-logical-values.md
