# Nullable user-function contracts

Experiment 2026-10-11, SDK/compiler/simulator 9.2.0, fr265, optimization disabled.
Question: can nullable values cross user-call boundaries while source, typed IR,
and saved text retain the same validated contracts?

## Contract and specimen

Garmin's [Object API][api] uses `or Null` in public type declarations. Our
[example][example] independently exercises that annotation syntax on user
Number/String parameters and results. The SDK accepted these sources. This
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
console logs before the next run overwrites them. Observe for 15 seconds and
compare actual lines with the example's expected output.

## Observations and limits

Both applications produced the same ten lines. Native `toNumber()` results
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
Nullable Boolean, general unions, and nullable automatic console entries remain
unsupported. No compiler
or SDK executable was disassembled or decompiled for this experiment.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html#equals-instance_function
[example]: ../../examples/compiler-nullable-contracts/README.md
