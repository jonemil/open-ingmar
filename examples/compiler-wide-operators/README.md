# Wider numeric String and Char operators

[app.mc](app.mc) checks String concatenation and identity comparison with Long,
Float, Double, and their nullable forms. It also checks Char/Long addition,
Char equality with each numeric width, immutable Char weak results, compound
local/indexed writes, truncation of wide character offsets, and operand
evaluation order.
SDK/generated `fr265` programs match [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-wide-operators
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 3.1.0 --app-id 3ac513d86a1f42dc9ed8f82a9129b5b1 --entry MinimalConsoleApp.onStart examples/compiler-wide-operators/app.mc scratch/compiler-wide-operators/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-wide-operators/unsigned.prg scratch/compiler-wide-operators/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-wide-operators/generated.prg
```

The String conversion methods used to obtain nullable Long/Double values require
API 3.1.0. Char forms themselves require 1.3.0. Wider Char ordering and floating
Char addition are rejected by SDK checking and our compiler. See the
[source contracts](../../docs/specs/compiler-subset.md#operator-contracts).
