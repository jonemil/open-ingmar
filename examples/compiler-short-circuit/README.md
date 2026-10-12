# Short-circuit conditions

[app.mc](app.mc) and [flags.mc](flags.mc) exercise Boolean `&&`/`||` conditions,
postfix/comparison precedence, negation, nullable Number/String guards, and loop
retesting. Required operand calls print their names; skipped calls would print
`bad-*` markers. API context: 1.0.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-short-circuit
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.0.0 \
  --app-id a4e6303c6e17416b96b6c340e984a774 --entry start \
  examples/compiler-short-circuit/app.mc \
  examples/compiler-short-circuit/flags.mc \
  scratch/compiler-short-circuit/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-short-circuit/unsigned.prg \
  scratch/compiler-short-circuit/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-short-circuit/generated.prg
```

Expected output, with no `bad-*` lines:

```text
skip-and
and skipped
skip-or
or skipped
and-left
and-right
and both
or-left
or-right
or both
precedence
negated
inverted
8
or guarded
9
empty
ok
loop
0
loop
1
8
```

This example focuses on `if`/`while` condition trees, including parentheses and
negation. The [logical value example](../compiler-logical-values/README.md)
exercises assignments, arguments, and returns.
See the [source contract][source] and [evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#short-circuit-expressions
[evidence]: ../../docs/investigations/compiler-short-circuit.md
