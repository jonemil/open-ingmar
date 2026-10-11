# Native Number method calls

[app.mc](app.mc) calls inherited `Number.toString()` on integer locals, stores
one String result, and calls [numbers.mc](numbers.mc), which converts an integer
parameter. The runtime provides the method implementation; the compiler resolves
its contract through `Lang.Object`.

From the repository root:

```sh
mkdir -p scratch/compiler-native-calls
cargo run -p ciq-compiler -- --unit --emit prg \
  --app-id 9ab482e0a39942f59cb771d037b86a01 --entry start \
  examples/compiler-native-calls/app.mc examples/compiler-native-calls/numbers.mc \
  scratch/compiler-native-calls/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-native-calls/unsigned.prg \
  scratch/compiler-native-calls/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-native-calls/generated.prg
```

Observed output on SDK 9.2.0 / fr265:

```text
41
-1
0
```

Only named Number locals/parameters and zero-argument `toString()` expressions
are supported. String results can be stored or printed; user function returns
remain integer/void. See [source rules][source], [native call IR][ir], and
[experiment evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#native-value-calls
[ir]: ../../docs/specs/compiler-ir-text.md#native-value-calls
[evidence]: ../../docs/investigations/compiler-native-calls.md
