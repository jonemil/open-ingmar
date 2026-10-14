# Number bitwise operations and shifts

The [source specimen](bits.mc) checks Number `&`, `|`, `^`, `~`, `<<`, and
`>>`, signed boundaries, Monkey C precedence, ordered calls, a logical operand,
guarded nullable Numbers, and an array write. SDK 9.2.0 and our generated
Forerunner 265 app match the [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-number-bits
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 55cb2c25c6a94ad68e936ab46ff4ab9e --entry MinimalConsoleApp.onStart examples/compiler-number-bits/app.mc examples/compiler-number-bits/bits.mc scratch/compiler-number-bits/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-bits/unsigned.prg scratch/compiler-number-bits/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-number-bits/generated.prg
```

Monkey C groups shifts and AND with multiplication; OR/XOR share addition's
level. Each level associates left to right. For example, `1 + 2 << 2` yields
9, and `2 << 1 * 3` yields 12. Parentheses make alternative grouping explicit.
Operands must be non-null Numbers; Char and Boolean are distinct contracts.

The compiler passes shift counts unchanged. The separate
[count-boundary probe][evidence] records behavior for -1, 32, and 33 in the
tested simulator. See the [source contract][source] and [IR syntax][ir].

[source]: ../../docs/specs/compiler-subset.md#arithmetic-and-lowering-contract
[ir]: ../../docs/specs/compiler-ir-text.md
[evidence]: ../../docs/investigations/compiler-number-bits.md#count-boundaries
