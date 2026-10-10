# Compiler probe

A standalone source input for our restricted Rust Monkey C frontend:

```sh
mkdir -p scratch/compiler-example
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/compiler-example/probe.mcasm
```

This emits a labeled five-byte function returning `41`. It is not a complete watch-app project and does not have a Jungle/manifest. [The compiler experiment](../../docs/investigations/first-compiler.md) documents explicit integration with Hello World's application scaffold, own-key signing, and simulator runs for `41` and a changed source returning `42`.

See [the supported grammar](../../docs/specs/compiler-subset.md) and [IR design](../../docs/design/compiler-ir.md). Keep generated assembly and binaries under ignored `scratch/`.

You can also emit semantic IR with `--emit ir`, resume it with `--from ir`, or generate raw CODE bytes with `--emit code`. These routes share the same models/encoder; see [complete stop/resume commands](../../docs/specs/compiler-ir-text.md#command-line-stages) and [the pipeline experiment](../../docs/investigations/compiler-pipeline.md).

[arithmetic.mc](arithmetic.mc) returns `40 + (8 - 5)`. Compile it with:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/arithmetic.mc scratch/compiler-example/arithmetic.mcasm
```

The eleven-byte body executes `subv` then `addv`, without constant folding. [The arithmetic experiment](../../docs/investigations/compiler-arithmetic.md) records simulator results `43` and `-3`, integration commands, and pipeline equivalence.

[locals.mc](locals.mc) initializes two locals and reads the first repeatedly, returning `37`:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/locals.mc scratch/compiler-example/locals.mcasm
```

The backend allocates two local slots and emits `argcincsp`, `lputv`, and `lgetv`. [Local-variable evidence](../../docs/investigations/compiler-locals.md) covers SDK comparisons, simulator execution, and IR text. Reassignment is not supported yet.

[calls.mc](calls.mc) calls externally supplied `Numbers.left()` and `Numbers.right()`, returning `49` with the helper definitions in [the call experiment](../../docs/investigations/compiler-calls.md). It emits symbolic assembly:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/calls.mc scratch/compiler-example/calls.mcasm
```

Raw CODE additionally needs `--symbols <symbol-source.mcasm>` from that scaffold. The frontend does not generate the module or its method metadata yet.

[parameters.mc](parameters.mc) defines a two-parameter `difference` function with a local after its parameters. [arguments.mc](arguments.mc) calls it with nested argument expressions. Compile each independently:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/parameters.mc scratch/compiler-example/parameters.mcasm
cargo run -p ciq-compiler -- examples/compiler-probe/arguments.mc scratch/compiler-example/arguments.mcasm
```

[The parameter/argument experiment](../../docs/investigations/compiler-arguments.md) integrates both generated bodies into the supplied module metadata, returning `48`. Calls still require explicit symbol bindings and a matching scaffold signature.
