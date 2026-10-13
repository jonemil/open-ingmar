# Native calls as statements

[app.mc](app.mc) exercises calls on locals, parameters, literals, and computed
receivers while discarding their final results. It includes Array mutations,
chained calls, nullable results, a guarded receiver, logical arguments, and a
loop. API minimum: 1.3.0; target: 5.2.0; runtime: SDK 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-native-statements
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 1.3.0 \
  --app-id b90fbcc785fc4dcdafad78e22e3a1e61 --entry start \
  examples/compiler-native-statements/app.mc \
  scratch/compiler-native-statements/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-native-statements/unsigned.prg \
  scratch/compiler-native-statements/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-native-statements/generated.prg
```

Expected output:

```text
[b, c, d]
[b, c, d]
receiver
value
receiver
[b, c, d]
b
done
```

Mutation affects shared references; dropping a result does not suppress the
call. The receiver is evaluated before the argument. The logical argument
skips its right-hand helper, so `unexpected` must not appear. Calls returning
nullable values execute without printing those values. See the [source
contract][source], [IR contract][ir], and [evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#native-value-calls
[ir]: ../../docs/specs/compiler-ir-text.md#native-value-calls
[evidence]: ../../docs/investigations/compiler-native-statements.md
