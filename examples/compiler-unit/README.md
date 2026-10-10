# Multiple-function compiler example

[entry.mc](entry.mc) defines global `probe`; [numbers.mc](numbers.mc) defines
three integer-return functions in module `Numbers`. Compile both files as one
unit, including the forward calls from the first file. Run from the repository
root:

```sh
mkdir -p scratch/compiler-unit-example
cargo run -p ciq-compiler -- --unit examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/compiler-unit-example/unit.mcasm
cargo run -p ciq-compiler -- --unit --emit ir examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/compiler-unit-example/unit.ciqir
cargo run -p ciq-compiler -- --unit --from ir scratch/compiler-unit-example/unit.ciqir scratch/compiler-unit-example/resumed.mcasm
```

Outputs must be new files. The unit checks member identities and argument
counts/types across files. Lowering gives each declaration a distinct CODE
label; calls retain symbolic module/member operands. Raw encoding needs explicit
symbol bindings. Lifecycle methods, application metadata, DATA placement,
resources, and signing are still separate work.

`left()` prints `1` and returns `45`; `right()` prints `2` and returns `2`.
`difference(a, b)` returns `a + 3 - b`. `probe()` therefore prints `1`, `2`,
then `47`, and returns `48`. The [simulator experiment][evidence] confirms this
behavior when all four generated methods replace their SDK reference bodies.
Reversing the arguments produces prints `2`, `1` and result `-40`.

See [source/IR/CLI rules](../../docs/specs/compiler-unit.md). The
[console application example](../compiler-application/README.md) also generates
a complete application from these same two files with `--entry probe`.

[evidence]: ../../docs/investigations/compiler-unit.md
