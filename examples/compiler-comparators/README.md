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
API levels; see [API contracts](../../docs/specs/sdk/README.md#declaration-model).
`InterfaceCalls` forwards Comparable and Comparator parameters/results through
locals, calls their declared methods, and sorts through a Comparator value.
Receiver/argument markers check evaluation order. See the
[interface profile](../../docs/specs/frontend/README.md#structural-interfaces)
for remaining limits, including mixed scalar-token key constants and general Object Arrays.
`NativeComparables` checks all seven supported scalar witnesses, typed returns,
scalar assignments, mixed interface/scalar conditional arms, and a Number guard.
`NullableInterfaces` checks guarded calls, early returns, loops, nullable sorting,
and direct source-object assignments to nullable and non-null interface locals.
`InterfaceHolder` checks nullable/non-null interface fields, replacement, null
defaults, stored comparators, instance isolation, and a local snapshot retained
after clearing its field.
`InterfaceArrays` checks typed literals, indexed writes, native collection
methods, nullable elements, scalar runtime tags, and Comparator reads used to
sort a scalar Array.
`InterfaceSorting` checks default `compareTo` dispatch on ranked source objects,
explicit reverse ordering, aliases, nullable Comparator forwarding, homogeneous
native witnesses, and Comparator-element Arrays with an explicit callback.
`InterfaceDictionaries` checks nullable/non-null interface values, source writes,
missing-key guards, shared mutation, enumeration snapshots, scalar constant tags,
and retrieved Comparators used for sorting.
`InterfaceKeys` checks hashing/equality overrides, collisions, equivalent source
keys, inherited identity, native witnesses inserted at runtime, key enumeration,
and removal. Comparison counters verify that hashing does not use the ordering
methods. Mixed scalar-token key constants remain deferred; see the
[key contract](../../docs/specs/frontend/README.md#interface-dictionary-keys).
`InterfaceConstants` checks all scalar tags, Number/Char distinctions, independent
constant loads versus shared aliases, nullable Comparable contents, and a
Null-only Comparator constant used for default sorting.

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
[sorting contract](../../docs/specs/frontend/README.md#source-comparators).
