# Class ownership and current-receiver calls

2026-10-10. Question: can the frontend preserve class/method ownership, emit
current-receiver calls, and run them on an independently generated application
instance? Environment: SDK 9.2.0, Java 17, Forerunner 265, default simulator
memory, our developer key. See [source/IR profile][profile].

## SDK reference

An ignored copy of the earlier SDK reference lives at
`scratch/compiler-class/sdk-reference/`. Its application class has:

```monkeyc
function onStart(state) { self.start(); }
function start() {
    System.println(self.answer(50, 3));
    self.announce();
}
function answer(left, right) { return left - right; }
function announce() { System.println("done"); }
```

It also includes ordinary AppBase initialization and a native blank View. Build
with `sh scripts/build-example.sh scratch/compiler-class/sdk-reference fr265`;
inspect with `ciq-dis code` at its generated build PRG. Optimization is zero.
No external implementation code was copied.

**Observed SDK CODE**:

- onStart at `0x24`: `argc 2; getselfv start; invokemz; popv; return`.
- start at `0x56`: the print's argument selects `answer` with `getselfv` at
  `0x62`, then `frpush`, `50`, `3`, `invokem 3`. No explicit receiver value is
  pushed. The subsequent print uses the existing counted-call convention.
- The no-argument announce statement uses `getselfv` at `0x71`, then
  `invokemz; popv`.
- answer begins `argc 3`; left/right parameters use slots 1 and 2 before `subv`.

This supports the existing hidden-context slot and a distinct self-call
selection operation. It does not establish arbitrary object representation,
field operations, inherited member lookup, or source superclass behavior.

## Independent generation and runtime

The [two-file example][example] declares an AppBase class with methods and a
module/global function with the same `answer` name. The compiler preserves class
ownership, resolves self calls within it, and emits owner-qualified labels.
DATA binds methods to the source class's generated application record. Generated
onStart invokes `Counter.start` with receiver lookup; base initialization and
the native View remain supplied by the bootstrap.

**Simulator observed**, after own-key signing:

```text
class startup
1
2
47
receiver call
3
2
9
```

The trace establishes nested left-to-right self calls, ordered subtraction,
void statement calls, a loop with early return, and a module call from a method.
Self selects the class answer body; the module's same-name method returns `9`.
The global same-name body returning `7` remains separate. Tests also verify
these methods share a member ID but have distinct DATA CODE targets.

Changing only `left()` from return `50` to `51` changes `47` to `48`; the rest of
the trace is unchanged. Selecting `Counter.left` as the integer startup entry
prints `1` followed by `50`, verifying the wrapper's automatic result print for
instance calls. All three eight-second sessions leave the blank View active
without an error dialog. Console traces establish execution; screen captures
alone do not. Captures are under ignored `scratch/compiler-class/simulator-47/`,
`simulator-48/`, and `simulator-integer-50/`.

Source and saved unit IR produce identical complete unsigned PRGs. Signed PRG
export/reassembly is byte-identical and its signatures verify. The `50` → `51`
edit changes exactly one unsigned CODE byte, offset `204` (`0xcc`); every other
unsigned payload is unchanged. Signing changes signatures as expected.

| Entry/input | DATA | CODE | RESOURCES | Signed PRG |
|---|---|---|---|---|
| Counter.start, 47/48 | 274 | 317 | 20 | 2324 |
| Counter.left, 50 | 274 | 329 | 8 | 2324 |

Sizes are bytes. Signed SHA-256:

```text
47 d713217dc74580e288aadd3bc07e95317e12fb62a3247866a8b1db6d83bcb62d
48 35421855e5a64c9b97e5469a5f52a706378b9aa2c525efea92f4d477456f59f7
50 0ed641d9574ecb9764284ab84ea6b87adf1c750306d2f95bd51bdaaebb3801af
```

## Reproduction and limits

Use the [example commands][example] to build, sign, and run fresh outputs. Save
and resume the same unit, keeping entry selection:

```sh
target/debug/ciq-compiler --unit --emit ir \
  examples/compiler-class/app.mc examples/compiler-class/numbers.mc \
  scratch/compiler-class/unit.ciqir
target/debug/ciq-compiler --unit --from ir --emit prg \
  --app-id 3c94c9e5ba2c43fa849d64ecff33264e --entry Counter.start \
  scratch/compiler-class/unit.ciqir scratch/compiler-class/from-ir.prg
cmp scratch/compiler-class/unsigned.prg scratch/compiler-class/from-ir.prg
target/debug/ciq-dis all scratch/compiler-class/generated.prg \
  scratch/compiler-class/roundtrip-project
target/debug/ciq-asm project scratch/compiler-class/roundtrip-project \
  scratch/compiler-class/roundtrip.prg
cmp scratch/compiler-class/generated.prg scratch/compiler-class/roundtrip.prg
target/debug/ciq-asm verify scratch/compiler-class/roundtrip.prg \
  -y keys/developer_key.der
```

Eight tests cover source/typed/text parity, receiver ownership, name collisions,
class framing/limits, exact arity and integer/void signatures, effect order,
method fields and distinct owner targets, entry selection, and CLI output
preservation. Tests use no SDK files or key.

Next add typed native-base calls and source initialization, then the value types
needed for source-authored callbacks and View creation. General objects,
fields, native inherited self calls, imports, and multiple application classes
are not established by this step. Other devices and real-watch execution remain
unverified.

[profile]: ../specs/compiler-classes.md
[example]: ../../examples/compiler-class/README.md
