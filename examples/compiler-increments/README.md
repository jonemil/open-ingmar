# Number updates and Char increment

[app.mc](app.mc) checks prefix/postfix statement updates on locals, self fields,
and Number/Char-array elements. Calls in array/index targets print markers so
repeated or reordered evaluation is visible. It also checks signed boundaries, guarded
nullable reads, loops, and writes visible through a shared Char array. SDK 9.2.0
and generated `fr265` programs match all 31 lines of
[expected output](expected.txt).

From the repository root, using fresh output paths:

```sh
mkdir -p scratch/compiler-increments
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 2.1.0 --app-id 869aef856dca4c458cf6879ea717883e --entry MinimalConsoleApp.onStart examples/compiler-increments/app.mc scratch/compiler-increments/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-increments/unsigned.prg scratch/compiler-increments/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-increments/generated.prg
```

Both `n++;` and `++n;` replace `n` with its value plus one; decrement subtracts
one. These forms are statements and produce no expression result. Parameters
remain immutable. Char locals and array elements support increment; Char
decrement and other numeric classes are outside this profile. See
[the source contract][source] and [runtime evidence][evidence].

[source]: ../../docs/specs/compiler-subset.md#increment-and-decrement
[evidence]: ../../docs/investigations/compiler-increments.md
