# Array transforms

[app.mc](app.mc) exercises `Array<Char>.reverse()` and `slice()`, element-type
preservation through user calls, and documented slice-bound behavior. API context:
1.3.0; runtime validation: SDK 9.2.0 / fr265.

From the repository root:

```sh
mkdir -p scratch/compiler-array-transforms
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id a2547f9f400c4b7094c85256a7f81801 --entry start \
  examples/compiler-array-transforms/app.mc \
  scratch/compiler-array-transforms/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-array-transforms/unsigned.prg \
  scratch/compiler-array-transforms/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-array-transforms/generated.prg
```

Expected output:

```text
[a, c, b, a]
0
4
[a, b, c, a]
[b, c]
1
2
[a, b, c, a]
[c, a]
[a, b, c, a]
[]
```

The source remains unchanged by `reverse`. `slice` includes its start and omits
its end; null bounds select the corresponding edge, negative bounds count from
the end, and out-of-range bounds clip to the array. The [source contract][spec]
and [SDK evidence][evidence] describe the supported profile.

[spec]: ../../docs/specs/frontend/containers.md#char-arrays
[evidence]: ../../docs/investigations/compiler-array-transforms.md
