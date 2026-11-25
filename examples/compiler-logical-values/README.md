# Logical value results

[app.mc](app.mc) and [flags.mc](flags.mc) exercise Boolean results from `&&` and
`||` in initialization, assignment, returns, calls, comparisons, and native
receivers. Effectful helpers show evaluation order across joins.
The final cases cover native truth tests, skipped dynamic operands, and the
integer operation used when both logical operands execute.
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

See [expected.txt](expected.txt) for the console output. Each `bad-*` marker
belongs to a skipped operand.

A value-producing logical branch retains its tested value on the stack.
An operand that needs extra blocks uses temporary locals that count toward the
frame limit. See the [source contract][source] and [evidence][evidence].

[source]: ../../docs/specs/frontend/README.md#short-circuit-expressions
[evidence]: ../../docs/investigations/compiler-logical-values.md
