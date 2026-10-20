# Native View construction and arrays

[app.mc](app.mc) constructs native Views, passes them through typed calls and
array reads, and exercises shared references, search/removal, bulk append,
reverse/slice, and contextual empty reassignment. API minimum: 2.1.0; target:
5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-native-views
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 2.1.0 \
  --app-id 47cb2c25c6a94ad68e936ab46ff4ab9e --entry start \
  examples/compiler-native-views/app.mc scratch/compiler-native-views/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-native-views/unsigned.prg \
  scratch/compiler-native-views/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-native-views/generated.prg
```

Expected output:

```text
make
created
2
3
0
true
true
1
3
1
1
0
3
0
```

The created objects are tested as references; the bootstrap still displays its
generated blank View. The [initial View example][initial] supplies the source
callback; drawing remains subsequent work. See the [source contract][source]
and [evidence][evidence].

[source]: ../../docs/specs/sdk/README.md#native-views
[evidence]: ../../docs/investigations/compiler-native-views.md
[initial]: ../compiler-initial-view/README.md
