# Void startup and module calls

[main.mc](main.mc) calls functions in [workers.mc](workers.mc), then falls through
without an integer result. The compiler infers a void result, emits its return,
and generates a complete console application. The application wrapper invokes
`main` once without printing a result.

From the repository root, use fresh outputs:

```sh
mkdir -p scratch/compiler-void
cargo run -p ciq-compiler -- --unit --emit prg \
  --app-id 3c94c9e5ba2c43fa849d64ecff33264e \
  examples/compiler-void/main.mc examples/compiler-void/workers.mc \
  scratch/compiler-void/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-void/unsigned.prg \
  scratch/compiler-void/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-void/generated.prg
```

Expected console output, with a blank native View:

```text
start
1
2
47
3
2
zero
99
done
```

The trace checks left-to-right argument calls, an explicit void return, an early
loop return, zero loop iterations, and discarding the integer returned by
`Numbers.answer()`. It does not print that function's returned `7` or an
automatic result for `main`. Changing `Numbers.left()` to return `51` changes
`47` to `48`.

Integer-return functions still require a value on every path. Void functions
accept `return;` or falling through the closing brace. Mixing those return forms
is rejected. User-module statement calls accept integer arguments and discard
integer or void results; a void call cannot appear in an expression.

See [source rules](../../docs/specs/frontend/statements.md#void-functions-and-statement-calls),
[IR](../../docs/specs/ir/calls.md#void-returns-and-statement-calls),
and [simulator evidence](../../docs/investigations/compiler-void.md). Application
classes and receiver/object operations are still supplied by the generated
bootstrap, rather than source class declarations.
