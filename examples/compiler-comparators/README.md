# Source comparison contracts

[ordering.mc](ordering.mc) supplies `compare(Object, Object) as Number` through
an existing source View class. It orders Object string conversions and retains
per-instance direction and call-count state. [app.mc](app.mc) checks callback
dispatch, typed source forwarding, shared mutation, receiver/argument order,
and empty/singleton inputs. The [expected output](expected.txt) matches
SDK/generated `fr265` programs.

The Object-rooted `TextOrder` class supplies `compareTo(Object) as Number`, the
Comparable signature. Direct calls check negative, equal, and positive results.
The library's `CompilationUnit::check_interface` checks this signature and its
API levels; see [API contracts](../../docs/specs/api-declarations.md#declaration-model).
`InterfaceCalls` forwards Comparable and Comparator parameters/results through
locals, calls their declared methods, and sorts through a Comparator value.
Receiver/argument markers check evaluation order. See the
[interface profile](../../docs/specs/compiler-subset.md#comparison-interface-values)
for remaining limits, including interface fields and object-array sorting.
`NativeComparables` checks all seven supported scalar witnesses, typed returns,
scalar assignments, mixed interface/scalar conditional arms, and a Number guard.
`NullableInterfaces` checks guarded calls, early returns, loops, nullable sorting,
and direct source-object assignments to nullable and non-null interface locals.

From the repository root:

```sh
mkdir -p scratch/compiler-comparators
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 949ff3758b28492cb54821459b9c90fa --entry MinimalConsoleApp.onStart examples/compiler-comparators/app.mc examples/compiler-comparators/ordering.mc scratch/compiler-comparators/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-comparators/unsigned.prg scratch/compiler-comparators/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-comparators/generated.prg
```

The structural contract requires exact Object inputs and a non-null Number
result. Source callbacks need no Comparator superclass. This profile retains
exact source-class identity or an explicit Comparator contract; erased Object
comparators remain unsupported. See the
[sorting contract](../../docs/specs/compiler-subset.md#source-comparators).
