# Void functions and statement calls

2026-10-10. Complete application lifecycle methods need returns without integer
values. This step adds semantic void returns and effect-only user-module calls
before introducing class/receiver syntax. Reference environment: SDK 9.2.0,
Java 17, Forerunner 265, default memory, our developer key.

## Implementation and inputs

[Source rules][source] infer void when a function has no valued return. Bare
`return;` is explicit; reaching the final brace inserts a void terminator.
Integer-return functions retain their every-path requirement. Mixed returns are
rejected. Function IR records the validated common result type; saved text uses
[IR][ir] for void returns and statement calls. Unit validation checks
arity and rejects using a void callee as a value, including forward references.

The [two-file example][example] has a void global startup, three void module
functions, and three integer helpers. The existing application builder chooses
a startup wrapper from the entry's result contract: integer entries print their
result, while void entries discard the VM call result without a print.
Both profiles use independent metadata and a native blank View.

The backend emits `return` without a preceding value for void terminators.
Statement calls use the existing getmv/context/invoke sequence, followed by
`popv`, for both void and integer callees. Argument values execute left to right.
This discard convention is runtime-tested below; semantic void does not become
an expression value or an invented integer zero. Null/object/array values and
source classes are not added in this step.

## Non-returning result contracts

2026-10-20, SDK 9.2.0, `fr265`, `-O0`. Authored functions containing only
`for (;;) {}` compile with explicit Number and source-class results. Logs and
inputs are `scratch/compiler-unbraced/nonreturning-{number,source}.{mc,log}`.
Therefore absence of a value return must not erase a declared result to Void.

The `CoreNonreturning` group in
[source-types.mc](../../examples/compiler-object-tests/source-types.mc) retains
Number, String, and source-class result contracts for infinite-loop functions.
Callers select finite branches, producing `41`, `nonreturning-types`, and `1`
in both SDK/generated applications. Infinite loops are compiled and loaded but
not executed; this is a contract/loading check, not a termination experiment.
Source, saved IR, and assembly agree, and both PRGs pass structured
reconstruction without raw CODE/DATA fallback and own-key signing checks.

Reproduce with the shared [console checker](../../scripts/README.md#console-specimen-check)
and `compiler-object-tests`; artifacts are in `scratch/core-nonreturning/check/`.
Focused tests cover scalar/nullable/Array/source results, non-returning tuple
callbacks, unresolved source classes, inconsistent caller metadata, and reachable
void returns/fallthrough. Existing return/type/API checks remain required;
the [IR contract](../specs/ir/README.md#cyclic-control-flow) owns the rules.

## Simulator observations

The own-key signed application printed, in order:

```text
start
1
2
47
3
2
zero
99
done
```

This establishes execution of no-argument void calls, left-to-right integer
argument calls, subtraction order, explicit void return, early return from a
loop, zero-iteration fallthrough, and discarded integer-call results. Neither
`Numbers.answer()`'s returned `7` nor an automatic startup result is printed.
The `done` marker shows the caller continues after the discarded calls.

Changing only `Numbers.left()` from `return 50;` to `return 51;` prints `48`
instead of `47`; every other line is unchanged. Both eight-second observations
leave the blank View active with no error dialog. Captures are under ignored
`scratch/compiler-void/simulator-47/` and `simulator-48/`. Console traces establish
execution; the blank display alone does not. Other devices and real-watch
execution remain unverified.

## Parity and reproduction

Source and saved-unit-IR generation produce identical unsigned PRGs. Export and
reassembly preserve the signed PRG byte for byte; its developer signatures
verify with our key. Both signed variants are 2388 bytes, with DATA 300 bytes,
CODE 363 bytes, and RESOURCES 12 bytes. The source edit changes exactly one
unsigned CODE byte, offset `321` (`0x141`), from `50` to `51`; all other unsigned
section payloads are identical. Signatures change and verify independently.

Signed SHA-256:

```text
47 4057dd9c9e0d91d589ca81c754b687986df98b73665382a9d04aac456038107c
48 cb69433c24c03c7cf9e94331f2939c925f534d56ed7519c3c864283528fa1aa2
```

Use the [example commands][example] for generation/signing/running. Save and
resume a unit with fresh outputs:

```sh
target/debug/ciq-compiler --unit --emit ir \
  examples/compiler-void/main.mc examples/compiler-void/workers.mc \
  scratch/compiler-void/unit.ciqir
target/debug/ciq-compiler --unit --from ir --emit prg \
  --app-id 3c94c9e5ba2c43fa849d64ecff33264e \
  scratch/compiler-void/unit.ciqir scratch/compiler-void/from-ir.prg
cmp scratch/compiler-void/unsigned.prg scratch/compiler-void/from-ir.prg
target/debug/ciq-dis all scratch/compiler-void/generated.prg \
  scratch/compiler-void/roundtrip-project
target/debug/ciq-asm project scratch/compiler-void/roundtrip-project \
  scratch/compiler-void/roundtrip.prg
cmp scratch/compiler-void/generated.prg scratch/compiler-void/roundtrip.prg
target/debug/ciq-asm verify scratch/compiler-void/roundtrip.prg \
  -y keys/developer_key.der
```

Six new tests cover void inference/terminators, typed and text validation,
control-flow/effect ordering, unit result/arity checks, VM call/result-discard
sequences, complete-PRG parity, and CLI rejection before output creation.
Existing rejection cases now valid as void functions or statement calls move
to positive coverage; integer-return missing-path checks remain. Malformed IR
headers are rejected.

Next establish source class ownership and receiver/context semantics, then the
null/object/array and native-call operations required by AppBase/View lifecycle
bodies. Void return support does not itself provide class syntax or project
configuration.

[source]: ../specs/frontend/README.md#void-functions-and-statement-calls
[ir]: ../specs/ir/README.md#void-returns-and-statement-calls
[example]: ../../examples/compiler-void/README.md
