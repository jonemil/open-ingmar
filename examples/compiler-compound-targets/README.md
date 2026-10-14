# Field and indexed compound assignments

The [source specimen](targets.mc) checks Number fields and Number/Char array
elements, including RHS mutations and a logical expression. SDK 9.2.0 and our
generated Forerunner 265 app match all 15 lines of [expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-compound-targets
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 58cb2c25c6a94ad68e936ab46ff4ab9e --entry MinimalConsoleApp.onStart examples/compiler-compound-targets/app.mc examples/compiler-compound-targets/targets.mc scratch/compiler-compound-targets/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-compound-targets/unsigned.prg scratch/compiler-compound-targets/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-compound-targets/generated.prg
```

`self.count += self.value()` uses the old field value 10 even though the RHS
sets it to 20, then stores 13. Indexed compounds evaluate receiver and index
once, read the old element before the RHS, and use the captured location for
the final write. The array mutations demonstrate that its old value survives
changes to contents and length.

Number fields/elements support all ten compound operators; Char elements
support `+= Number`. Existing type and frame limits apply. Bounds can fail at
the initial read or at the final write after RHS mutations. See the
[indexed contract][indexed], [field contract][fields], and [evidence][evidence].

[indexed]: ../../docs/specs/compiler-subset.md#indexed-compound-assignment
[fields]: ../../docs/specs/compiler-classes.md#number-instance-fields
[evidence]: ../../docs/investigations/compiler-compound-targets.md
