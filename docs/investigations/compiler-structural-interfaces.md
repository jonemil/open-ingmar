# Source structural interfaces

Environment: SDK/compiler/simulator 9.2.0, fr265, minimum API 3.0.0 and target
5.2.0. Inputs are independently authored Monkey C programs, not SDK internals.
Garmin's [Monkey Types guide][types] describes lexical interfaces with fields
and functions, anonymous annotations, and typedef naming.

The [application specimen](../../examples/compiler-interfaces/README.md) checks
two unrelated source implementations, an anonymous native `toString` contract,
nullable interfaces, mutable fields, snapshots, and ordered compound updates.
SDK and generated applications match all
[26 console lines](../../examples/compiler-interfaces/expected.txt). Both PRGs
support structured CODE/DATA export/reassembly and identical repeat exports,
without required raw fallback. No interface runtime class or new opcode is
needed: ordinary symbol lookup/calls and property writes execute the program.

## Signature and declaration probes

The following additions were compiled against the same AppBase scaffold:

- A required `answer() as Object` rejects an implementation returning Number.
- A required `answer(x as Number) as Number` rejects an implementation accepting
  Object, despite Number being assignable to Object.
- A required mutable `value as Object` rejects an implementation field declared
  Number. The complete specimen also needed explicitly typed Number fields;
  SDK fields declared only as `var count = 1` did not establish its witness.
- An unused `typedef Recursive as interface { function next() as Recursive; };`
  compiles. This establishes recursive declaration acceptance, not operations
  on recursively typed values.
- The complete specimen's `TokenBox` interface accepts a nullable Token field,
  an `echo(Token) as Token` method, and a method returning Token or SpareToken.
  Reads survive later field mutation; returned source objects support ordinary
  member calls. These paths compile and execute in both toolchains.

These observations support exact method signatures and invariant mutable fields
on this SDK. Recursive contracts remain outside the current finite-member
implementation. Its existing inferred Number field
profile provides a stronger contract than an SDK unannotated field; tracked
runtime comparison uses explicit declarations.

## Reproduction

Create an ignored SDK project using the minimal-console manifest/resources and
`../../examples/compiler-interfaces` as its source path. Set application ID
`937df7506e9b4ed7ae2a8f0196dded52`, build with `scripts/build-example.sh`, then use
`scripts/check-example.py` for preparation and SDK/generated simulation.
The reusable SDK PRG is indexed in `scratch/sdk-cache/sdk-index.json`; per-change
builds, captures, and logs are disposable after review. Current contracts belong
in the [frontend](../specs/frontend/README.md#source-interface-types),
[IR](../specs/ir/README.md#source-interface-contracts), and
[lowering](../specs/lowering/README.md#structural-members) references.

[types]: https://developer.garmin.com/connect-iq/monkey-c/monkey-types/
