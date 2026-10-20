# Char literal encoding and source limits

Question: which instruction creates a Char literal, and which source forms can
we reproduce? Experiments on 2026-10-12: SDK/compiler/simulator 9.2.0, fr265,
optimization disabled. Char's public API 1.3.0 introduction is documented in
the [Char method review](compiler-char-methods.md).

## Isolated SDK specimen

An ignored probe prints `.toNumber()` results for `'A'`, `'é'`, `'猫'`, `'😀'`,
`'\n'`, `'\r'`, `'\t'`, `'\''`, and `'\\'`. It uses the local fr265
manifest/resources and an AppBase wrapper whose `onStart(state)` calls `start()`
and whose initial View is `[new Toybox.WatchUi.View()]`.

```sh
sh scripts/build-example.sh scratch/compiler-char-literals
cargo run -p ciq-dis -- all \
  scratch/compiler-char-literals/build/fr265/compiler-char-literals.prg \
  scratch/compiler-char-literals/probe-export
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-char-literals/build/fr265/compiler-char-literals.prg
```

**Observed:** CODE initially stopped at offset `0x38`, before the first literal.
The bytes there are `34 00 00 00 41`. Each subsequent literal uses opcode `34`
and a four-byte big-endian operand. Following bytes resolve `toNumber` and
invoke it. Adding this encoding permits complete CODE decoding in the specimen.
Our mnemonic is `cpush`; it names the observed Char push instruction.

| Source | CODE offset | Operand | Console Number |
|---|---|---|---|
| `'A'` | `0x38` | `00000041` | 65 |
| `'é'` | `0x50` | `000000e9` | 233 |
| `'猫'` | `0x68` | `0000732b` | 29483 |
| `'😀'` | `0x80` | `0000d83d` | 55357 |
| `'\n'` | `0x98` | `0000000a` | 10 |
| `'\r'` | `0xb0` | `0000000d` | 13 |
| `'\t'` | `0xc8` | `00000009` | 9 |
| `'\''` | `0xe0` | `00000027` | 39 |
| `'\\'` | `0xf8` | `0000005c` | 92 |

The supplementary character becomes a UTF-16 high surrogate in this SDK build,
whereas Number-to-Char conversion preserves 128512 in the earlier probe.
**Design decision:** raw source Chars require one BMP Unicode scalar until this
discrepancy is resolved. Do not truncate a supplementary source character.
Combining sequences containing multiple scalars are also rejected.

## Source escape probes

Isolated probes on 2026-10-20 use SDK/simulator 9.2.0, fr265, `-O 0`.
Create an ignored project with the ordinary console manifest/resources and an
AppBase wrapper whose `onStart(state)` executes one statement and calls
`System.exit()`. Compile with `scripts/build-example.sh`. For Char probes,
print the literal's `.toNumber()`; for Strings, print or compare the value.

**Observed:** both quote forms accept `\b`, `\f`, `\n`, `\r`, `\t`, `\'`,
`\"`, `\\`, and `\uNNNN` with exactly four hexadecimal digits. Unicode Chars
preserve code units: `\u0000` → 0, `\u00e9` → 233, `\u732B` → 29483,
`\uD83D` → 55357, `\ude00` → 56832, and `\uFFFF` → 65535. Unlike a raw
supplementary Char, these surrogate values are explicit source code units.

String `"\uD83D\uDE00"` exports as UTF-8 `😀`. An isolated `"\uD83D"`
compiles but exports as `"?"` in DATA. **Design decision:** accept valid pairs
and reject unpaired String surrogates rather than silently replacing them.
Keep the raw supplementary Char restriction above. These are explicit subset
boundaries, not claims that the SDK rejects either form.

Isolated String probes reject `\x41`, `\0`, `\101`, `\u{41}`, `\q`, `\/`,
backslash-newline continuation, and short `\u041`. Char probes reject
`\x41`, `\0`, `\q`, short `\u041`, and two-unit `'\uD83D\uDE00'`.
Several invalid forms produce misleading parser/symbol
diagnostics; the isolated compile failure establishes rejection, not a useful
error taxonomy.

The [shared object specimen][escape-example] groups the accepted forms,
surrogate Char Array storage, escaped versus raw Unicode String content,
embedded NUL length, and non-recursive backslash decoding. String content checks
use `.equals()`: `==` can distinguish identical contents stored at different
addresses. On this simulator, `"a\u0000b".length()` reports 1 despite the
three-byte stored value; preserving the value does not imply that native String
methods process bytes after NUL. Use the specimen's
console checker for SDK/generated runtime comparison and structured CODE/DATA
reconstruction. Both applications match the expected output, and both PRGs
reconstruct without raw CODE/DATA fallback. See the
[current literal contract][literal-contract].

## Raw IR bits and complete application

A separate generated app edits five Char constants in saved IR before lowering.
Each constant feeds `toNumber()` and `println`:

| `.char` bits | Observed Number |
|---|---|
| `0x00000000` | 0 |
| `0x0000ffff` | 65535 |
| `0x0000d83d` | 55357 |
| `0x0001f600` | 128512 |
| `0xffffffff` | -1 |

Compile a unit containing five `System.println('A'.toNumber())` statements to
IR, replace its five `.char` operands in order with the values above, and use
`--from ir --unit --emit prg` with API 1.3.0 or later. Sign/run with the ordinary
example workflow. These observations support raw u32 preservation in IR and
assembly, without requiring every value to be a valid Unicode scalar. String
conversion and rendering for those raw values remain unverified here.

For the [complete source example][example], create an ignored `group.jungle`
selecting its sources and the same wrapper/manifest/resources. Build with:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-char-literals/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-char-literals
```

SDK and generated applications produced the same 18 console lines. They cover
BMP literals and escapes, case conversion, equality with Number-created Chars,
hash consistency, typed user calls, local replacement, and comparison.
Source/resumed-IR PRGs and assembly round trips agree. Focused checks cover
source byte spans, malformed literals/escapes, raw operand widths, truncated
instructions, edited consumers, and API introduction diagnostics.
The SDK example's complete section export and reconstruction were byte identical,
including the Char push instructions and preserved signing material.

[example]: ../../examples/compiler-char-literals/README.md
[escape-example]: ../../examples/compiler-object-tests/README.md
[literal-contract]: ../specs/frontend/grammar.md#lexical-rules-and-literals
