# Number unary plus

The [source specimen](plus.mc) checks unary plus on Number boundaries, literals,
calls, grouped signs, postfix methods, guarded nullable reads, and an array
write. SDK 9.2.0 and our generated Forerunner 265 app match all 18 lines of
[expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-number-plus
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 56cb2c25c6a94ad68e936ab46ff4ab9e --entry MinimalConsoleApp.onStart examples/compiler-number-plus/app.mc examples/compiler-number-plus/plus.mc scratch/compiler-number-plus/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-plus/unsigned.prg scratch/compiler-number-plus/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-plus/generated.prg
```

Unary plus preserves the Number value and evaluates its operand once. It binds
after postfix calls and before binary operators. Repeated or mixed prefixes
need parentheses: `+(+n)` and `+(-n)` are accepted, while `+ +n` and `+-n`
are rejected. `++` is a separate statement update, not a unary-plus expression.

Our profile requires a non-null Number, including a fresh nullable Number read
under an established null proof. Saved IR retains `.positive` for that check;
lowering emits its operand without an additional VM operation.
See the [source contract][source], [IR syntax][ir], and [SDK evidence][evidence].

[source]: ../../docs/specs/frontend/operators.md#expressions-and-operators
[ir]: ../../docs/specs/ir/model.md
[evidence]: ../../docs/investigations/compiler-number-plus.md
