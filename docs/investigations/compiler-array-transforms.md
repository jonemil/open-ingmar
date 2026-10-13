# Array reverse and slice

The SDK and generated fr265 applications agree on element-preserving
`reverse`/`slice` results and the documented slice-bound rules. The bounded API
model now substitutes the receiver's element type through generic method
signatures. Runtime behavior was checked with SDK 9.2.0 and Forerunner 265.

## Evidence

**Officially documented:** [`Toybox.Lang.Array`][array] declares `reverse()` as
returning a new reversed Array and `slice(Number or Null, Number or Null)` as a
new Array. Slice includes the start, excludes the end, supports null and
negative bounds, and truncates out-of-range bounds. Both methods require API
1.3.0. The public signatures expose `Array<Object or Null>`.

**Experimentally observed:** SDK compilation accepted both results where our
source required `Array<Char>`, including `indexOf(Char)` and a call to a user
function taking `Array<Char>`. The SDK application printed:

```text
[a, c, b, a]
0
4
[a, b, c, a]
[b, c]
1
2
[a, b, c, a]
[c, a]
[a, b, c, a]
[]
```

This covers a reversed result, preserved source order, ordinary slice,
null-bound full slice, negative start, clipped bounds, and start-after-end. SDK
and generated application logs are saved under
`scratch/compiler-array-transforms/`.

**Interpretation:** the generic API result must be instantiated with the
receiver's element type for these methods. The source checker also specializes
`indexOf`'s element argument; the existing SDK probe rejected non-Char arguments
for `Array<Char>.indexOf`.

The SDK-produced CODE symbol table binds `reverse` to `0x008004c1` and `slice` to
`0x008004c2`. These IDs come from the compiled specimen. Generated and SDK
outputs match line for line in the official simulator.

## Reproduction

The source and expected output are in
[the example](../../examples/compiler-array-transforms/README.md). The SDK-only
wrapper and Jungle file are ignored local artifacts. Build with the repository's
SDK script and run each program in the simulator using the documented helper.

The API catalogue models generic element parameters and substitutes them from
the concrete receiver. Executable array values remain restricted to non-null
Char arrays. Mutation, other element types, comparator sorting, and all
non-Char generic runtime behavior remain unverified.

[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
