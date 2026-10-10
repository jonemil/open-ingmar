# Branches and exception-bearing functions

2026-10-10. Question: what must assembly resolve before the compiler gains
conditions? SDK 9.2.0, Java 17, `fr265`, debug `-O 0`, our developer key.
All simulator runs use the original device profile and memory allowance.

## Sources and scope

- [MonkeyC Ghidra Plugin][ghidra], revision
  `12ae55010721165b4ba98254d5e77db22eaa1b07`, Apache-2.0:
  `doc/opcodes.md` and `doc/prg.md`, EXCEPTIONS table.
- [monkeyc-optimizer][optimizer], revision
  `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT:
  `src/readprg/opcodes.ts` (`parseCode`, `getOpEffects`),
  `exceptions.ts` (`parseExceptions`), and `emit.ts` (relative fixups).
- Our [control-flow source](../../examples/compiler-probe/control-flow.mc),
  inserted into the Hello World scaffold as described below.

External sources inform encoding interpretations; no implementation code was
copied. At this research checkpoint the new instructions required raw CODE
export. [Subsequent branch-label support](branch-labels.md) adds those encodings
and structured CODE; [exception assembly](exception-assembly.md) subsequently
adds structured EXCEPTIONS.

## SDK observations

`choose(0)` returns `41`, `choose(1)` returns `43`, and `countdown(3)` returns
`6`. `caught(false)` returns `41`; `caught(true)` throws `new Lang.Exception()`
and catches it, returning `43`. The scaffold's `probe()` returns
`choose(1) + countdown(3)`, displaying `49`.

The PRG's CODE payload starts at file offset `0x338a`, length 1021. SDK debug
XML gives inclusive function spans: `caught` `0x04..0x51`, `countdown`
`0x7e..0xa6`, `choose` `0xa7..0xc1`, and `probe` `0xc2..0xe1`.

**Experimentally observed:** previously unsupported encodings:

| Mnemonic | First CODE offset | Bytes | Interpretation |
|---|---|---|---|
| `ret` | `0x50` | `17` | local subroutine return |
| `goto` | `0x2b` | `19 0000` | unconditional relative jump |
| `eq` | `0xb4` | `1a` | equality comparison |
| `gt` | `0x8f` | `1d` | greater-than comparison |
| `jsr` | `0x30` | `23 001d` | local subroutine call |
| `bf` | `0x11` | `29 001a` | branch if false |
| `throw` | `0x2a` | `33` | throw the constructed exception |
| `btpush` | `0x206` | `3e` | push Boolean true |
| `bfpush` | `0x1f0` | `3f` | push Boolean false |

Names and abstract effects are **source-derived**, supported by the source and
execution observations. Exact exception-stack/frame behavior is unresolved.
In particular, SDK emits `jsr` to a shared `ret` even for this try/catch without
an explicit `finally`; do not treat `jsr` as an ordinary function invocation.

Relative target calculation agrees with instruction boundaries:

```text
target = instruction_offset + 3 + signed_i16_be(operand)

CODE+0xb5: 29 0006  -> 0xbe   # choose's else return
CODE+0xa1: 19 ffe8  -> 0x8c   # countdown's loop test (-24)
CODE+0x30: 23 001d  -> 0x50   # caught's shared ret
```

The EXCEPTIONS payload is 20 bytes:

```text
0002 00000f 000034 00003a 00003a 00004c 00004c
```

**Source-derived interpretation, matching the specimen:** a big-endian u16
count followed by ordered `(begin, end, handler)` triples of big-endian u24
CODE offsets. Entry 1 covers the try body and selects catch at `0x3a`; entry 2
covers the catch path and selects rethrow cleanup at `0x4c`. Both starts, ends,
and handlers lie on instruction boundaries. Half-open interval semantics are
consistent with this layout but were not isolated experimentally.

## Controlled edits and simulator results

Each variant starts from the original PRG and is signed/verified with our key.
The console prints six results after `Control flow probe`:

| Variant | Console results | Screen |
|---|---|---|
| Original | `41, 43, 6, 41, 43, 49` | `49` |
| Branch inverted | `43, 41, 6, 41, 43, 47` | `47` |
| Loop once | `41, 43, 2, 41, 43, 45` | `45` |
| Catch moved | `41, 43, 6, 41, 47, 49` | `49` |

- **Branch inverted:** change opcode at CODE+`0xb5` from `29` (`bf`) to `28`
  (`bt`), keeping the displacement. The observed swap supports their opposite
  condition sense for these Boolean comparison results.
- **Loop once:** replace `ffe8` at CODE+`0xa2` with `0000`, targeting the
  instruction immediately after the jump. This skips repeating the loop.
- **Catch moved:** append the 78-byte `caught` function at CODE+`0x3fd`, changing
  its catch constant from `43` to `47`. Point the method value at DATA+`0x49f`
  from `0x10000004` to `0x100003fd`. Keep old exception entries and append two
  translated entries: `(0x408, 0x42d, 0x433)` and
  `(0x433, 0x445, 0x445)`, raising the count to four. Internal relative operands
  remain unchanged because both instruction and target move by `0x3f9`.

**Experimentally observed:** the moved function handles its explicit exception
and returns the distinctive `47`. The original CODE prefix is byte-identical.
Only CODE/signature changes occur in the first two variants; the moved variant
also changes the method value and EXCEPTIONS. Every other section is identical.
No resource padding change was needed: appended CODE and exception entries add
96 bytes together. DEBUG/line metadata stays unchanged and has no mapping for
the appended function; accurate relocated debugging is not claimed.

Captures and logs are under ignored `scratch/control-flow/simulator-*`.
Signed SHA-256 values:

```text
original        ecfeda07cc22a07d048e934596b4cfca0868d6965b4bc3605849e807bc186285
branch-inverted b28b443d51db4933e1378a7a657ad424680ace2cecaaeab00da8fdf609e5ec8d
loop-once       fef9f470f9934b00d05b096a8ff64dea9bf40c4ed5eb8a7c3f0e722f5de09557
caught-moved    f4216015bcbda64f6c767863c77d805a2f4652d85742705a9baa4c090b70cd6d
```

## Reproduction

Create the SDK specimen from our checked-in source and scaffold:

```sh
python3 - <<'PY'
from pathlib import Path
import shutil
p = Path('scratch/control-flow/sdk-probe')
p.mkdir(parents=True, exist_ok=True)
for name in ('manifest.xml', 'monkey.jungle', 'resources'):
    src = Path('examples/hello-world') / name
    if src.is_dir():
        shutil.copytree(src, p / name, dirs_exist_ok=True)
    else:
        shutil.copy2(src, p / name)
s = Path('examples/hello-world/source/HelloWorld.mc').read_text()
s = s.replace('using Toybox.Graphics;',
              'using Toybox.Graphics;\nusing Toybox.Lang;')
s = s.replace('System.println("Hello from Monkey C!");',
              'System.println("Control flow probe");\n'
              '        System.println(choose(0));\n'
              '        System.println(choose(1));\n'
              '        System.println(countdown(3));\n'
              '        System.println(caught(false));\n'
              '        System.println(caught(true));')
s = s.replace('return 41;', 'return choose(1) + countdown(3);')
functions = Path('examples/compiler-probe/control-flow.mc').read_text()
s += '\n' + functions.split('\n\n', 1)[1]
(p / 'source').mkdir(exist_ok=True)
(p / 'source/HelloWorld.mc').write_text(s)
PY
sh scripts/build-example.sh scratch/control-flow/sdk-probe fr265
```

For binary experiments, read ordered PRG sections and apply exactly the edits
above to their payloads. Recompute section lengths, retain unrelated sections,
write the terminator, and sign each new output. Assert the original opcode,
operand, and method bytes before patching; offsets apply to this specimen.
These are controlled raw edits, not existing symbolic assembler commands.

```sh
cargo run -p ciq-asm -- sign scratch/control-flow/caught-moved-unsigned.prg scratch/control-flow/caught-moved.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/control-flow/caught-moved.prg -y keys/developer_key.der
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/control-flow/caught-moved.prg --device fr265 --observe-seconds 10
```

## Next step and limits

Implement [branch-label fixups](../design/control-flow.md), then structured
lossless exception sources. Validate through independent Rust tests and an
assembler-produced simulator edit before extending frontend conditions.
Displacement extremes, endpoint inclusivity, nested/typed catches, `finally`,
exception handler stack state, extended CODE, and other devices remain open.
The successful whole-function move does not validate arbitrary insertions or
deletions within protected ranges.

[ghidra]: https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin/tree/12ae55010721165b4ba98254d5e77db22eaa1b07
[optimizer]: https://github.com/markw65/monkeyc-optimizer/tree/2920dc88c23caadded9f66ee0613a7de6ab2d1ea
