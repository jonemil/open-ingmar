# Immutable-value weak calls

[app.mc](app.mc) and [values.mc](values.mc) exercise `weak()` on Number,
String, and Boolean. Each result retains the receiver's type. The example
covers arithmetic, conditions, string chaining, typed arguments/returns,
locals, repeated calls, guarded nullable receivers, and receiver effects.
It requires API 1.2.0.

From the repository root:

```sh
mkdir -p scratch/compiler-immutable-weak
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.2.0 \
  --app-id 88a08a66fa6e4a00b7a19870f7a38943 --entry start \
  examples/compiler-immutable-weak/app.mc \
  examples/compiler-immutable-weak/values.mc \
  scratch/compiler-immutable-weak/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-immutable-weak/unsigned.prg \
  scratch/compiler-immutable-weak/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-immutable-weak/generated.prg
```

Expected output, observed on SDK 9.2.0 / fr265:

```text
41
-40
hello
HELLO
true
false
condition
true
true
true
7
9
TEXT
receiver
3
```

Garmin's type checker retains a WeakReference result for these calls. The
[investigation](../../docs/investigations/compiler-immutable-weak.md) records
the initial casted probe and SDK comparison with checking disabled. Our
specialized contract follows the documented immutable-value exception and
observed runtime behavior. Ordinary object weak references use
[a separate profile](../compiler-weak-references/README.md).
