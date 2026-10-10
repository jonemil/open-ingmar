# Repeated method names in modules

2026-10-10. Question: can one member symbol name multiple methods, and how should a section export distinguish them? Scope: two modules in one SDK-generated PRG, with a same-size edit to one method. No new linking or relocation behavior.

## Inputs and procedure

Independently authored local source: [module-names](../../examples/module-names/README.md), derived from our multi-file scaffold. `Left.answer()` returns `21`; `Right.answer()` returns `22`. App lifecycle code prints both values and their sum, and the view displays the sum. SDK/compiler/simulator 9.2.0, `fr265`, `-O 0`, project-local device data and external `developer_key.der`. No external implementation was incorporated for this change.

```sh
sh scripts/build-example.sh examples/module-names fr265
cargo build -p ciq-dis -p ciq-asm
# All output paths must be new.
target/debug/ciq-dis all examples/module-names/build/fr265/module-names.prg scratch/module-names/qualified-project
target/debug/ciq-asm project scratch/module-names/qualified-project scratch/module-names/roundtrip.prg
cmp examples/module-names/build/fr265/module-names.prg scratch/module-names/roundtrip.prg
cp -R scratch/module-names/qualified-project scratch/module-names/edited-project
```

In `edited-project/003-code-c0debabe.mcasm`, change only `ipush1 0x15` in `method_sym_00000002__answer` to `ipush1 0x14`. Keep every other instruction and all placement constraints.

```sh
target/debug/ciq-asm project scratch/module-names/edited-project scratch/module-names/left-20.prg -y developer_key.der
target/debug/ciq-asm verify scratch/module-names/left-20.prg -y developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/module-names/roundtrip.prg
sh scripts/run-simulator-container.sh --prg scratch/module-names/left-20.prg
```

Actual runtime validation used the existing Garmin container through container-broker, running `xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg <path> --device fr265 --observe-seconds 8`. Retained logs/screens are under ignored `scratch/module-names/simulator-43/` and `simulator-42/`; both screens were inspected.

## Experimentally observed

| Property | Left | Right |
|---|---|---|
| Module symbol | `0x00000002`, `globals/Left` | `0x00000080`, `globals/Right` |
| Member symbol | `0x00000085`, `answer` | `0x00000085`, `answer` |
| DATA class record | `0x000004d9` | `0x00000566` |
| DATA method field | `0x000004ea` | `0x00000577` |
| CODE method target | `0x00000030` | `0x000000f2` |
| Exported label | `method_sym_00000002__answer` | `method_sym_00000080__answer` |

Calls use `getmv @sym_00000002 @answer` or `getmv @sym_00000080 @answer`, followed by `invokemz`. They carry different module IDs and the same member ID. Each method prologue uses `argc 0x01`, `spush @<module>`, `getm`, and `lputv 0x00` before its literal-return body. This module example does not establish a universal function prologue.

The previous exporter used offset labels for the repeated name. The updated exporter retains a short label for a unique member; repeated names gain an owner-qualified alias. The shared method target at CODE `0x03` remains numeric. DATA and line metadata use the same labels as CODE. Unknown or ambiguous ownership is retained conservatively. Full [naming rules](../specs/code-labels.md) include concatenation collision handling and reserved generated prefixes.

Unchanged full export/import is byte identical. The signed edit changes exactly one ordinary CODE byte, at offset `0x3b` (`0x15` to `0x14`), and regenerates developer signatures; all other section payloads are identical. Both signatures verify with the supplied key. The reconstructed original prints the greeting, `21`, `22`, `43` and displays `43`. The edited PRG prints the greeting, `20`, `22`, `42` and displays `42`.

| Artifact | SHA-256 |
|---|---|
| SDK original / unchanged roundtrip | `b2d8997dc89ac581dab64c09acaa6ea44cb86ba5aad332fda147ec8bf8b062d2` |
| Own-key signed Left-only edit | `a403baaf0e9ab555e89ed9bf430fb61bb16d9eb96028db59b7220191ad6b30ca` |

## Interpretation and limits

A member ID alone is not a globally unique executable method identity in this specimen. Separate module records bind that member to separate CODE targets. Owner-qualified assembly labels improve editability without changing VM symbols, adding include scopes, or requiring a separate linker.

The exported owner remains a DATA alias. Mapping path-like aliases to pleasant source module names, nested module lookup, inheritance overrides, overloaded method behavior, and separate compilation remain unverified. Shared stubs must not inherit whichever binding appeared first. Four new Rust tests cover qualification, shared-target ordering, generated-label collisions, and full-project reconstruction with a shared member ID. All 111 workspace tests pass with Rust 1.99; formatting and Clippy with warnings denied pass.

Next start the restricted constant-return Monkey C compiler milestone; investigate branch/exception fixups alongside its later extensions. Full relocation is not required for the first straight-line body.

## MIR follow-up

Local SDK 9.2.0 build output also contains `build/fr265/mir/source/Left.mir`. Observed: it retains the `module Left` and `function answer()` structure, source-location annotations and generated labels, but expresses the return through a temporary assignment (`%tmp.1 = 21;`) followed by `ret %tmp.1;`. The corresponding ordinary CODE contains the module prologue described above followed by `ipush1 0x15` and `return`. MIR is therefore at a higher abstraction level than our bytecode assembly; this comparison does not specify its full grammar or lowering rules.

Other observed build paths include `internal-mir/Rez.mir` (generated resource declarations), and `external-mir/` for barrel-consumer inputs. The installed `connectiq/bin/api.mir` contains Toybox API declarations, types, documentation, and empty function bodies. The same extension therefore covers declaration-oriented content as well as generated function bodies. Neither our documented barrel nor IQ archive specimens contains a MIR member. These observations do not rule out other package profiles carrying it.

Primary corroboration: [Garmin Connect IQ team's compiler-debugging response](https://forums.garmin.com/developer/connect-iq/f/discussion/396955/odd-compiler-errors-with-sdk-7-4-1/1868511) distinguishes generated MIR in intermediate logs from original Monkey C files in verbose logs. Read locally on 2026-10-10 alongside the SDK 9.2.0 artifacts.

Interpretation for our toolchain: a future source compiler may use its own intermediate representation before emitting `.mcasm`; our assembler currently consumes bytecode assembly, not Garmin MIR. There is no evidence here that preserving or implementing Garmin MIR is required for PRG reassembly or the first constant-return compiler milestone. MIR could be a useful source-to-bytecode research aid or an eventual compatibility input, separately scoped from package preservation.
