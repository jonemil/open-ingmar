# Nullable user-function contracts

SDK/compiler/simulator 9.2.0, fr265, optimization disabled. Number/String probes
were run 2026-10-11; Boolean/Char probes were run 2026-10-16.
Question: can nullable values cross user-call boundaries while source, typed IR,
and saved text retain the same validated contracts?

## Contract and specimen

Garmin's [Object API][api] uses `or Null` in public type declarations. Our
[example][example] independently exercises that annotation syntax on user
Number/String/Boolean/Char parameters and results. The SDK accepted these sources. This
establishes acceptance on the tested SDK, not general support for every union
syntax or inference rule in Monkey C.

For SDK compilation, use a local fr265 manifest/resources with
`MinimalConsoleApp` as entry, and this ignored wrapper:

```monkeyc
class MinimalConsoleApp extends NullableApp {
    function initialize() { NullableApp.initialize(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
```

Create `scratch/compiler-nullable-contracts/monkey.jungle` with the manifest,
example source directory, wrapper, and resource paths, then run:

```sh
sh scripts/build-example.sh scratch/compiler-nullable-contracts
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-nullable-contracts/build/fr265/compiler-nullable-contracts.prg
```

Build/sign/run the independent compiler output using the example README. Save
console logs before the next run overwrites them. Use the [console checker](../../scripts/README.md#console-specimen-check) to
compare both complete logs with the example's expected output.

## Observations and limits

Both applications match the example's complete [expected output](../../examples/compiler-nullable-contracts/expected.txt). Native `toNumber()` results
crossed module boundaries; String-or-Null values crossed instance boundaries.
Scalar and Null arguments were accepted by nullable parameters. Mixed String
and Null return paths, direct Number/String parameter guards, and a user-result
retry loop ran. The instance length helper also exercised a repeated guard.

Regression tests check equal PRGs from source and resumed unit IR, typed/text
validation, recursive/forward signatures, API revalidation, wrong alternatives,
unsafe consumers, and edited call result contracts. A declared union remains a
union even when all body returns are scalar or Null. The compiler checks this
in its shared validator rather than trusting source-only analysis.

Parameter facts are keyed separately from local facts. Tests verify that joins
and loops retain only proven facts, local writes preserve parameter facts, and
checks do not narrow copies or another parameter. Edited IR with swapped branch
outcomes, a changed checked parameter, or identical branch targets is rejected.
Boolean-or-Null calls preserve a present false value through guards and loops.
Char-or-Null calls preserve character values through module/instance signatures,
conditional returns, guarded arithmetic/conversion, and local Null assignments.
Both programs distinguish these values from missing values without coercion.
Char signatures retain their API 1.3.0 requirement. The SDK accepts these sources
with type checking; this does not establish general union inference.

General unions and nullable automatic console entries remain unsupported.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html#equals-instance_function
[example]: ../../examples/compiler-nullable-contracts/README.md
