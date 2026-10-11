# Native Number method calls

[app.mc](app.mc) calls inherited `Number.toString()` on integer locals, stores
one String result, and calls [numbers.mc](numbers.mc), which converts an integer
parameter. It also formats a computed Number, passes a String local as a format,
and prints markers while evaluating two calls inside a receiver expression.
The runtime provides method implementations; the catalogue supplies contracts.

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
042
07
receiver
argument
0045
```

Number expressions support `toString()` and `format(String)`. String results can
be stored or printed; user function returns remain integer/void. The marker calls
run left to right before formatting their sum. See [source rules][source],
[native call IR][ir], and
[experiment evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#native-value-calls
[ir]: ../../docs/specs/compiler-ir-text.md#native-value-calls
[evidence]: ../../docs/investigations/compiler-native-calls.md
