# Local compound assignments

The [source specimen](compound.mc) exercises all ten Number compound operators,
signed arithmetic/shifts, self reads, a staged logical RHS, loop counters,
nullable-local checks, and String/Char `+=`. SDK 9.2.0 and our generated
Forerunner 265 app match all 22 lines of [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-compound-locals
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 57cb2c25c6a94ad68e936ab46ff4ab9e --entry MinimalConsoleApp.onStart examples/compiler-compound-locals/app.mc examples/compiler-compound-locals/compound.mc scratch/compiler-compound-locals/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-compound-locals/unsigned.prg scratch/compiler-compound-locals/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-compound-locals/generated.prg
```

`n += rhs;` reads the current local, evaluates the RHS once, then stores the
binary result in the same slot. Existing binary contracts and fixed local
types apply. A nullable local needs a proof before the read and a fresh check
after the write for later non-null use.

This example uses bare initialized locals. The
[field/indexed example](../compiler-compound-targets/README.md) covers those
additional targets. Parameters and grouped local targets remain unsupported. Saved IR uses ordinary
read/binary/assign operations. See the [source contract][source] and
[investigation][evidence].

[source]: ../../docs/specs/frontend/README.md#local-and-parameter-compound-assignment
[evidence]: ../../docs/investigations/compiler-compound-locals.md
