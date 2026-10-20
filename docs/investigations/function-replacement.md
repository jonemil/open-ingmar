# Same-size arithmetic function replacement

Date: 2026-10-08. SDK/simulator target: 9.2.0 / `fr265`. Status: assembled,
cryptographically verified, and successfully executed in the host simulator.

## Question and design

After [the successful one-byte edit](developer-signing.md), can our assembler
replace the complete `probe()` function with a different sequence that computes
the same result? This experiment uses the original SDK Hello World specimen,
whose source is `examples/hello-world/source/HelloWorldApp.mc`. No
compiler/frontend or relocation implementation is added.

Original `probe()` occupies CODE `[0x30, 0x3d)`, 13 bytes:

```text
argc 0x01
spush @globals
getm
lputv 0x00
ipush1 0x29
return
```

Replacement also occupies 13 bytes, ending at the same return offset:

```text
argcincsp 0x01 0x01 # CODE+0x30: one context slot, one extra local
ipush1 0x28        # CODE+0x33: 40
ipush1 0x02        # CODE+0x35: 2
addv              # CODE+0x37
lputv 0x01        # CODE+0x38
lgetv 0x01        # CODE+0x3a
return            # CODE+0x3c
```

**Runtime-supported interpretation:** `argcincsp` reserves local slot 1 after
the single hidden context slot, allowing a store/load of the integer sum. The
original globals-resolution sequence is unnecessary for this arithmetic-only
body. These choices follow
[our instruction reference](../specs/vm/instructions.md), particularly the
compiler-generated `onUpdate` local-frame pattern. The successful host run
supports this combination of frame setup, integer addition, local slot 1
store/load, and return. It does not establish the full calling convention or
exact physical frame layout.

This gives the same result as the previously validated `42` variant through
different instructions. The temporary local makes the function exactly fit
without adding unreachable padding or changing subsequent entry points. Function
references, DATA, exceptions, resources, and debug/line payloads are preserved;
debug mappings within the rewritten function may no longer describe its new
instructions accurately.

## Reproduction

Use fresh output directories/files:

```sh
mkdir -p scratch/function-replacement
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/function-replacement/hello-arithmetic-project
```

In `003-code-c0debabe.mcasm`, replace the instructions starting at CODE `0x30`
through `0x3c` with the replacement above. Keep the next function's `argc` at
`0x3d` and all other declarations unchanged. Then:

```sh
cargo run -p ciq-asm -- project scratch/function-replacement/hello-arithmetic-project scratch/function-replacement/hello-arithmetic.prg -y keys/developer_key.der
cargo run -p ciq-asm -- verify scratch/function-replacement/hello-arithmetic.prg -y keys/developer_key.der
cargo run -p ciq-dis -- code scratch/function-replacement/hello-arithmetic.prg > scratch/function-replacement/hello-arithmetic.disassembly.mcasm
cargo run -p ciq-asm -- scratch/function-replacement/hello-arithmetic.prg scratch/function-replacement/hello-arithmetic.disassembly.mcasm scratch/function-replacement/hello-arithmetic-roundtrip.prg
```

No keys or generated binaries are checked in. Encoded replacement:
`4a 01 01 38 28 38 02 03 13 01 12 01 16`.

## Observed checks

- Output length 122668 bytes; 13 section identifiers, order, and payload lengths
  match the original.
- CODE remains 744 bytes. Its prefix before `0x30` and suffix from `0x3d` onward
  are byte identical to the original.
- All other non-signature sections are byte identical. Signing public-key fields
  remain identical; signatures are regenerated for the changed body.
- Rust verification against the developer key passes for SHA-1 and SHA-256.
  Independent Python public-RSA checks described in
  [the signing investigation](developer-signing.md) also pass.
- Disassembling and reassembling the modified PRG without signing reproduces it
  byte for byte (verified with Python byte comparison).
- Output SHA-256:
  `bb11b043c2b5a6ddd88bb6f86e8191e7a47051d754b7e76af4270d54c1292474`.

No Rust implementation changed, so these artifact checks are the relevant
validation; the preceding 52-test workspace result remains the implementation
baseline.

## Host runtime experiment

Podman is unavailable in the agent container. The user ran this command on the
host and reported success:

```sh
sh scripts/run-simulator-container.sh --prg scratch/function-replacement/hello-arithmetic.prg
```

The agent inspected the updated captures: the console contains
`Hello from Monkey C!` and `42`, and the screen shows `Hello world!` and `42`.
Both `onStart` and `onUpdate` call the replaced function. Capture timestamps
advanced to 2026-10-08 23:16, with simulator layout messages at 23:16:09; the
arithmetic PRG still matches the SHA-256 hash above.

The console and screen hashes equal the preceding one-byte experiment because
visible behavior is intentionally equivalent: console SHA-256
`b02f243853f712d3baf359332103eb9536423921b300c61b9e33304bd55b674c`, screen
SHA-256 `bc99ef1b2602410cbb751aab2e263d36968ce3974d1c5f25e293099a0d5fa111`.
Artifact selection is supported by the user's confirmation of the requested
command and fresh capture timestamps; these captures alone do not encode which
PRG was loaded.

This completes the same-size function-replacement milestone. Next investigate
structured class/method declarations and code-reference relocation using
controlled SDK specimens. Length-changing assembly remains deferred until
affected metadata and references are identified.
