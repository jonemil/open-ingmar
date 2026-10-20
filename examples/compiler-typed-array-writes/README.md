# Char and View array writes

This [source specimen](writes.mc) checks indexed replacement in Char and View
arrays. It exercises aliases, operand calls, a staged logical expression,
Char conversion, and native/source View references. SDK 9.2.0 and our generated
Forerunner 265 application match the [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-typed-array-writes
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 54cb2c25c6a94ad68e936ab46ff4ab9e --entry MinimalConsoleApp.onStart examples/compiler-typed-array-writes/app.mc examples/compiler-typed-array-writes/writes.mc scratch/compiler-typed-array-writes/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-typed-array-writes/unsigned.prg scratch/compiler-typed-array-writes/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-typed-array-writes/generated.prg
```

Receiver, index, and value evaluate once in that order. Values must match the
array's invariant element type; Char is distinct from Number, and Null or
BehaviorDelegate cannot fill a View slot. Source View instances can fill View
slots, which retain the native contract rather than exact source-class identity.

See the [shared source contract][source] and [experiment][evidence].

[source]: ../../docs/specs/frontend/statements.md#array-element-writes
[evidence]: ../../docs/investigations/compiler-typed-array-writes.md
