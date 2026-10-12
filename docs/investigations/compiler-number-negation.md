# Number negation and postfix behavior

Question: how should unary minus lower, and what happens at the i32 minimum?
Experiments on 2026-10-12: SDK/compiler/simulator 9.2.0, fr265, optimization
disabled. Inputs are our sources, with default SDK type checking.

## Probe evidence

An ignored probe includes a Number-parameter helper returning `-value`, grouped
constants, minimum values, postfix calls, and an effectful Number-returning call.
Use a local AppBase wrapper whose `onStart(state)` calls `start()` and whose
initial View is `[new Toybox.WatchUi.View()]`, with fr265 manifest/resources:

```sh
sh scripts/build-example.sh scratch/compiler-number-negation
cargo run -p ciq-dis -- all \
  scratch/compiler-number-negation/build/fr265/compiler-number-negation.prg \
  scratch/compiler-number-negation/probe-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-number-negation/build/fr265/compiler-number-negation.prg
```

**Observed:** negating 41 gives -41, negating zero gives zero, and negating
-2147483648 gives -2147483648. `-(-2147483648)` has the same minimum result.
SDK CODE pushes zero, evaluates the Number operand, then uses `subv`. An
effectful operand prints once before its negated result. This supports the
same lowering for our semantic negation operation and the observed minimum
wraparound; it does not establish all arithmetic overflow rules.

## Literal postfix discrepancy

The SDK probe prints:

| Source | Number result |
|---|---|
| `-1.abs()` | -1 |
| `(-1).abs()` | 1 |
| `-1.toNumber()` | 1 |
| `-(1.toNumber())` | -1 |
| negated Number-returning call followed by `.toNumber()` | expected negative result |

In the first specimen, `-1.abs()` emits zero, the *negative* literal -1,
`abs`, then `subv`. The later `toNumber` result demonstrates that a simple
"postfix before unary minus" explanation is insufficient for bare signed
literal receivers in this SDK build. The mechanism inside the SDK is unknown.

**Design decision:** keep those bare literal forms unsupported. Use
`(-1).toNumber()` for a negative receiver and `-(1.toNumber())` for negating a
positive receiver's result. Parameters, locals, module-call results, and grouped
expressions support unary minus with ordinary postfix precedence. SDK checking
also rejects spaced repeated prefixes (`- -41`); use `-(-41)` instead. Do not
turn these observations into a claim about all SDK versions.

## Complete comparison and IR contract

Create ignored `group.jungle` selecting the [complete example][example], the
same wrapper, and local manifest/resources. Build with:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-negation/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-negation
```

SDK and generated programs produced the same 19 console lines. They cover
parameter/local/call negation, grouped arithmetic, postfix precedence, guarded
nullable Numbers, both i32 extremes, mutation, conditions, and a single effect
marker. Source/resumed-IR PRGs and symbolic assembly round trips agree.

`.negate` requires a non-null Number and defines Number. Source and resumed IR
share that requirement; edited nullable branch targets do not bypass it.
Effect scheduling traverses its operand and retains call order. A test moves
an unrelated call ahead of negation while preserving valid dependencies and
confirms rejection for evaluation-order mismatch. Unary nesting counts toward
the existing depth bound. General positive signs, Boolean negation, non-Number
operands, and other firmware/device profiles remain outside this evidence.

[example]: ../../examples/compiler-number-negation/README.md
