# Compiler probe

A standalone source input for our restricted Rust Monkey C frontend:

```sh
mkdir -p scratch/compiler-example
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/compiler-example/probe.mcasm
```

This emits a labeled five-byte function returning `41`. It is not a complete
watch-app project and does not have a Jungle/manifest.
[The compiler experiment](../../docs/investigations/first-compiler.md) documents
explicit integration with Hello World's application scaffold, own-key signing,
and simulator runs for `41` and a changed source returning `42`.

See [the supported grammar](../../docs/specs/frontend/README.md) and
[IR design](../../docs/specs/ir/model.md#semantic-model). Keep generated assembly and
binaries under ignored `scratch/`.

You can also emit semantic IR with `--emit ir`, resume it with `--from ir`, or
generate raw CODE bytes with `--emit code`. These routes share the same
models/encoder; see [complete stop/resume
commands][ref-complete-stop-resume-commands] and
[the pipeline experiment](../../docs/investigations/compiler-pipeline.md).

[arithmetic.mc](arithmetic.mc) returns `40 + (8 - 5)`. Compile it with:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/arithmetic.mc scratch/compiler-example/arithmetic.mcasm
```

The eleven-byte body executes `subv` then `addv`, without constant folding. [The
arithmetic experiment][ref-the-arithmetic-experiment] records simulator results
`43` and `-3`, integration commands, and pipeline equivalence.

[locals.mc](locals.mc) initializes two locals and reads the first repeatedly,
returning `37`:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/locals.mc scratch/compiler-example/locals.mcasm
```

The backend allocates two local slots and emits `argcincsp`, `lputv`, and
`lgetv`. [Local-variable evidence](../../docs/investigations/compiler-locals.md)
covers SDK comparisons, simulator execution, and IR text. Local reassignment
is supported with the fixed-type rules below.

[calls.mc](calls.mc) calls externally supplied `Numbers.left()` and
`Numbers.right()`, returning `49` with the helper definitions in
[the call experiment](../../docs/investigations/compiler-calls.md). It emits
symbolic assembly:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/calls.mc scratch/compiler-example/calls.mcasm
```

Raw CODE additionally needs `--symbols <symbol-source.mcasm>` from that
scaffold. The frontend does not generate the module or its method metadata yet.

[parameters.mc](parameters.mc) defines a two-parameter `difference` function
with a local after its parameters. [arguments.mc](arguments.mc) calls it with
nested argument expressions. Compile each independently:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/parameters.mc scratch/compiler-example/parameters.mcasm
cargo run -p ciq-compiler -- examples/compiler-probe/arguments.mc scratch/compiler-example/arguments.mcasm
```

[The parameter/argument experiment][ref-the-parameter-argument-experiment]
integrates both generated bodies into the supplied module metadata, returning
`48`. Calls still require explicit symbol bindings and a matching scaffold
signature.

[strings.mc](strings.mc) prints generated UTF-8 strings and `40 + 2`, then
returns `43`. Lowering emits named literal requirements with no DATA placement:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/strings.mc scratch/compiler-example/strings.mcasm
```

Raw CODE uses both `--symbols` and `--data` from an explicit scaffold.
[The string/print experiment](../../docs/investigations/compiler-strings.md)
shows literal allocation, native/text parity, signing, and simulator output.
String concatenation/returns/parameters remain unsupported.

[control-flow.mc](control-flow.mc) is an SDK research input for conditional
returns, a backward loop, and an explicit caught exception. Our compiler does
not accept it yet. The [control-flow investigation][control-flow] explains
scaffold integration and simulator-verified binary edits. Explicit exceptions
remain unsupported by our frontend.

[ref-complete-stop-resume-commands]: ../../docs/specs/build/compiler-cli.md#compiler-cli-and-saved-stages

[ref-the-arithmetic-experiment]: ../../docs/investigations/compiler-arithmetic.md
[ref-the-parameter-argument-experiment]: ../../docs/investigations/compiler-arguments.md
[control-flow]: ../../docs/investigations/control-flow.md

[conditions.mc](conditions.mc) initializes a common local and returns `47` or
`49` from a terminal `if/else`. Compile it with:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/conditions.mc scratch/compiler-example/conditions.mcasm
```

`--emit ir` produces IR text with explicit blocks and a Boolean branch.
Both `==` and `>` have [simulator evidence][conditions].

[conditions]: ../../docs/investigations/compiler-conditions.md

[nested-conditions.mc](nested-conditions.mc) adds arm-local declarations, prints,
nested comparisons, early returns, and a continuing path to a shared return.
Sibling arms can reuse names; their locals do not escape the arm. Compile with:

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/nested-conditions.mc scratch/compiler-example/nested-conditions.mcasm
```

[The nested-condition experiment][nested] verifies both nested outcomes and
continuing arms, with displays `53` and `46`. Braces remain required; `else` is
now optional.

[nested]: ../../docs/investigations/compiler-nested-conditions.md

[assignments.mc](assignments.mc) updates a common integer and string local in
both condition arms, prints their selected values, then increments the integer.
Both locals reuse their original frame slots; type changes are rejected.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/assignments.mc scratch/compiler-example/assignments.mcasm
```

Raw CODE needs explicit symbol and DATA bindings for the generated literals.
[The assignment experiment][assignments] records native/text parity, integration,
and simulator returns `44`/`48` with display `54`. Parameters remain read-only;
compound assignment remains unsupported.

[assignments]: ../../docs/investigations/compiler-assignments.md

[loops.mc](loops.mc) decrements a stored counter in `while`, reinitializes a
body-local value each iteration, and accumulates a total through nested branches.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/loops.mc scratch/compiler-example/loops.mcasm
```

Its [runtime experiment][loops] verifies three/four iterations returning `49`/`53`
and a zero/one-iteration variant returning `40`/`41`. Shared scaffold displays
are `59` and `47`. Conditions are reevaluated each visit; there is no generated
iteration cap. Body locals do not escape.

[loops]: ../../docs/investigations/compiler-loops.md

## Loop control

[loop-control.mc](loop-control.mc) uses optional `else`, outer and inner
`continue`, and inner and outer `break`. The innermost loop supplies each
control's target. Arguments `0` and `1` produce totals `4` and `6`, returning
`44` and `46`. The [simulator experiment][loop-control] confirms display `52`
and a zero-iteration variant displaying `48`.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/loop-control.mc scratch/compiler-example/loop-control.mcasm
```

[loop-control]: ../../docs/investigations/compiler-loop-control.md
