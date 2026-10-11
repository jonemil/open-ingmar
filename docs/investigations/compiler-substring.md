# String substring and nullable results

Reviewed 2026-10-11. Question: can the shared IR handle nullable String results
without a new operation, while retaining substring's versioned index behavior?

## Public contract

Garmin's [String API documentation][api] declares `substring(Number or Null,
Number or Null)` returning String or Null, since API 1.0.0. Indices are zero-based;
end is exclusive. From API 3.3.2, Null selects the corresponding string boundary
and negative indices count from the end. These are documented contracts, not
observations of older firmware. The compiler checks nullable index availability
when an API context is supplied. Numeric range analysis remains unsupported.

## Experiment

SDK/compiler/simulator 9.2.0, fr265, optimization disabled. Inputs are the
[substring example][example], plus an ignored SDK wrapper whose AppBase
`onStart(state)` calls `start()`, `initialize()` calls the base initializer, and
`getInitialView()` returns `[new Toybox.WatchUi.View()]`.

Create `scratch/compiler-substring/monkey.jungle` pointing to those two sources,
the wrapper, and a local fr265 manifest/resources, then:

```sh
sh scripts/build-example.sh scratch/compiler-substring
cargo run -p ciq-dis -- all \
  scratch/compiler-substring/build/fr265/compiler-substring.prg \
  scratch/compiler-substring/sdk-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-substring/build/fr265/compiler-substring.prg
```

Build/sign/run our version with the example README commands. Capture each run's
console before another run overwrites it. Observe for 15 seconds and compare the
actual lines with the example's expected output; exit status alone is insufficient.

**Observed:** SYMBOLS identifies `substring` as `0x008000bd`. Both compilers'
applications produced the same ten lines. Normal slicing produced `bananas`;
equal indices produced an empty String; `(4, 2)` on `hello` produced `ll`;
`(0, 99)` produced `hello`; `(-2, 5)` produced `lo`; `(null, null)` returned the
whole String. A Number-or-Null index from failed `toNumber()` also selected the
beginning. Explicit assignment to Null exercised the retry loop and write
invalidation. None of these calls returned Null; the fallback branch is covered
by validation tests rather than this runtime sample.

**Interpretation:** ordinary `getsv` and two-argument invocation suffice for our
validated profile. Source and resumed IR produce equal PRGs in tests, and the
independently generated application runs successfully. Range normalization is
an observation of this simulator/device profile; it is not a guarantee for
other APIs, firmware, or every numeric input. No older simulator was tested.
A [String group specimen](compiler-string-methods.md) records UTF-8 index
observations; broader Unicode behavior remains unverified. Full SDK CODE export encountered an
unsupported opcode; the symbol observation and runtime comparison do not depend
on decoding that instruction.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html#substring-instance_function
[example]: ../../examples/compiler-substring/README.md
