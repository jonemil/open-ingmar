# Array add

The SDK and generated fr265 applications agree on `Array<Char>.add(Char)`:
`add` mutates the receiver, returns a usable `Array<Char>`, and preserves call
operand order. The SDK method requires API 1.3.0.

## Evidence

**Officially documented:** [`Toybox.Lang.Array.add`][array] appends one Object,
increases array size, and returns `self`. Its generic signature accepts
Object-or-Null and returns `Array<Object or Null>`.

**Experimentally observed:** SDK compilation accepts a Char argument and the
result where a user function requires `Array<Char>`. The compiled CODE symbol
record names `add` at `0x0080014e`. The simulator output was:

```text
[a, b, c]
[a, b, c]
[a, b, c]
3
4
[a, b, c, d]
receiver
value
[x, y, z]
```

The first three lines show the original local, a copied reference, and the
method result observing the appended element. A chained `add().size()` sees the
next mutation. The final lines show that a side-effecting receiver and argument
run once each, in receiver-then-argument order.

SDK and generated applications match line for line. The SDK PRG, exported
symbol table, and both simulator logs are retained under
`scratch/compiler-array-add/`.

## Compiler boundary

This runtime probe covers Char-array append and alias visibility. Generic element
substitution/type checks are specified in [API declarations](../specs/sdk/declarations.md).
[Bulk append/removal](compiler-array-mutation.md) has separate runtime evidence.

[array]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Array.html
