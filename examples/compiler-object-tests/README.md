# Runtime object typing

[app.mc](app.mc) checks `instanceof` against supported concrete Lang classes,
`has` with literal and computed Symbols, source fields/methods, null handling,
short-circuiting, and operand evaluation order. See [expected output](expected.txt).
Guarded Object reads support Number arithmetic, String methods, nullable inputs,
conditional arms, loops, and numeric comparator sorting. Lexical casts cover
scalar/reference assertions, nullable targets, erasure/recovery, runtime identity,
and one-time operand effects. [source-types.mc](source-types.mc) checks cross-file
source targets, guarded calls, exact return contracts, and Object/View/delegate
projections. Its imported bare native names also cover signatures, casts,
runtime tests, and container arguments. Nullable source parameters/results retain
Object-, View-, and delegate-derived identities through calls and local copies;
null checks permit source dispatch, and nullable locals can be reset to Null.
Nullable source fields default to Null and retain snapshot identity after the
field is cleared, including View and delegate references.
Nullable source casts recover these exact identities from opaque references,
preserve Null and false assertions at runtime, and evaluate an effectful operand
once. Source conditional values retain one exact class with optional Null;
nested choices and effect markers check lazy arm evaluation.

`CoreLoops` groups `for` and `do/while`: initializer/update ordering, omitted
clauses, scoped locals, nested break/continue targets, early returns, and skipped
headers. Markers check effects as well as numeric results.

`CoreScopes` checks standalone blocks, local-name reuse, and `else if` selection
with effect markers, nullable guards, early returns, and enclosing loop controls.

`CoreHeaders` checks `for` declaration/effect lists with ordered call markers,
dependent initializers/updates, continue/break paths, and indexed writes.

`CoreNonreturning` keeps declared Number, String, and source-class results for
infinite-loop functions. Selected finite branches exercise caller contracts
without executing the infinite loops.

`CoreDeclarations` groups ordered local declarations, heterogeneous initializer
types, scope/name reuse, nullable copies, and repeated statement terminators.

`CoreSwitch` covers value/type cases, String content and custom `equals`,
ordered effectful tests, default placement, fall-through, scoped locals, and
switch/loop controls. Nullable value selectors are guarded before dispatch.

`CoreLiterals` checks Number/Long radix patterns and negation, Float suffix and
leading-dot forms, NaN type/comparisons/Array storage, and a radix field default.

`CoreEscapes` checks source control/quote escapes, Unicode code units, String
surrogate pairs and embedded NUL, and Char array storage.

`CoreLists` checks trailing commas in Array, ByteArray, and Dictionary literals,
including constant records and ordered effectful/branching construction.

`CoreCalls` and `OwnerToken` check unqualified owner calls, recursion, Void and
discarded results, contextual Array arguments, instance mutation, native Object
dispatch, and declared member priority over local/parameter names.

From the repository root:

```sh
mkdir -p scratch/compiler-object-tests
cargo run -p ciq-compiler -- --unit --emit prg --target-api 5.2.0 --min-api 5.0.0 --app-id 123456789abcdef0123456789abcdef0 --entry MinimalConsoleApp.onStart examples/compiler-object-tests/app.mc examples/compiler-object-tests/source-types.mc scratch/compiler-object-tests/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-object-tests/unsigned.prg scratch/compiler-object-tests/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/compiler-object-tests/generated.prg
```

See the [source contract](../../docs/specs/frontend/README.md#runtime-object-predicates)
and [runtime evidence](../../docs/investigations/compiler-object-tests.md).
