# Number-array literals

[app.mc](app.mc) constructs arrays from Numbers, parameters, locals, arithmetic,
calls, and guarded nullable results. It also checks Char inference. API minimum:
1.3.0; target: 5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-number-array-literals
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id 45cb2c25c6a94ad68e936ab46ff4ab9e --entry start \
  examples/compiler-number-array-literals/app.mc \
  scratch/compiler-number-array-literals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-number-array-literals/unsigned.prg \
  scratch/compiler-number-array-literals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-number-array-literals/generated.prg
```

Expected output:

```text
[7, 8, 9]
[3, -3, 4]
10
20
[10, 20]
[5, 65]
42
[1, 2]
[12, 13]
[b, z]
```

The first element selects Number or Char; mixed elements are rejected without
coercion. Empty literals remain unsupported. See the [source contract][source]
and [runtime evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#number-arrays
[evidence]: ../../docs/investigations/compiler-number-array-literals.md
