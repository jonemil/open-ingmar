# Char operator contracts

Question: how do equality, ordering, and arithmetic treat Char operands?
Experiments on 2026-10-12: SDK/compiler/simulator 9.2.0, fr265, optimization
disabled. Public background: the [Monkey C reference][reference] lists arithmetic
and relational operators and identifies Char as UTF-32. It does not establish
all operand-pair rules used below; those come from our specimens.

## Probe results

An ignored SDK probe prints Char/Char and Char/Number equality/order results,
then Char + Number, Number + Char, and Char + Char results. A first version also
included all three subtraction pairs involving Char. Default SDK checking
rejected `Char - Number`, `Number - Char`, and `Char - Char`. Removing these
allowed compilation and execution; subtraction runtime behavior was not tested.

| Expression | Observed value | Effective result |
|---|---|---|
| `'A' == 'A'` | true | Boolean |
| `'A' == 'B'` | false | Boolean |
| `'A' == 65` | true | Boolean |
| `65 == 'A'` | true | Boolean |
| `'B' > 'A'` | true | Boolean |
| `'B' > 65` | true | Boolean |
| `66 > 'A'` | true | Boolean |
| `'A' + 1` | B, numeric conversion 66 | Char |
| `1 + 'A'` | B, numeric conversion 66 | Char |
| `'A' + 'B'` | AB | String |

Use a local AppBase wrapper whose `onStart(state)` calls `start()` and whose
initial View is `[new Toybox.WatchUi.View()]`, with fr265 manifest/resources:

```sh
sh scripts/build-example.sh scratch/compiler-char-operators
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-char-operators/build/fr265/compiler-char-operators.prg
```

**Observed:** operator equality compares Char/Number numerically. It differs
from `Object.equals`: `'A'.equals(65)` is false in the complete example.
**Design decision:** support only the tested Char/Number comparison pairs,
mixed addition returning Char, and Char/Char addition returning String.
Boolean/String/Null mixed operators and Char subtraction stay outside this
profile. The public native `compareTo` contract does not justify widening
operator types without separate evidence.

## Complete comparison and lowering

Create ignored `group.jungle` selecting the [complete example][example] and the
same wrapper/manifest/resources. Build with ordinary SDK type checking:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-char-operators/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-char-operators
```

Run its PRG, then build/sign/run our output with the example README. Both
programs produced the same 24 console lines. The typed Char and String helpers
compiled under SDK checking and ran successfully. Negative Number offsets,
BMP code points, local assignment, null guards, and left/right call markers
exercise result use and evaluation order. SDK CODE export fell back to raw at
unsupported offset `0x23b`; no full SDK instruction comparison is claimed.

Our generated programs use `addv`, `eq`, and `gt` directly. Simulator execution
establishes these instructions for the tested pairs without explicit conversion
calls. Semantic IR recomputes the result from operands; `.add` has no trusted
frontend result annotation. Source/resumed-IR PRGs and symbolic assembly agree.
Contract checks reject changed operand types, changed operators, incompatible
result consumers, and unchecked nullable operands. Earlier negative IR checks
now require a Number-only consumer because Char + Number is supported.

Overflow, raw invalid Char ranges, supplementary results, other numeric types,
other firmware/devices, and execution on an API 1.3.0 runtime remain unverified.
The compiler's minimum check follows known Char introductions; it is distinct
from proving instruction support on older VMs.

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[example]: ../../examples/compiler-char-operators/README.md
