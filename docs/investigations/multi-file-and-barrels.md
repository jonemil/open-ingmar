# Multi-file source, Monkey Barrels, and IQ packages

Date: 2026-10-10. SDK 9.2.0, `fr265`, Java 17.0.20.1, `-O 0`. Independently authored examples; no external implementation code incorporated. No Rust crate changes were needed.

## Questions and sources

Do source-file boundaries survive as separate linkable objects in a PRG? Does a barrel contain compiled bytecode? Does an IQ package change the unit our assembler handles?

Primary references:

- Garmin [Shareable Libraries](https://developer.garmin.com/connect-iq/core-topics/shareable-libraries/), with complete SDK-local documentation at `connectiq/doc/docs/Core_Topics/Shareable_Libraries.html` (System 9 / API 9.2.0).
- Garmin [Jungle Reference](https://developer.garmin.com/connect-iq/reference-guides/jungle-reference/), SDK-local `connectiq/doc/docs/Reference_Guides/Jungle_Reference.html`.
- SDK barrel template and `barrelbuild -h` for manifest structure and compiler options. These are syntax references, not copied project implementation.

The web pages currently expose navigation without their full content in the browsing tool; the installed SDK HTML supplies the actual documentation. Garmin describes barrels as shareable source/resource libraries and supports archive or project consumption. The archive and PRG observations below are direct experiments.

## Multi-file specimen

[examples/multi-file](../../examples/multi-file/README.md) separates lifecycle, display, and a numeric module into `App.mc`, `View.mc`, and `Numbers.mc`. Both callers invoke `Numbers.answer()`, which computes `40 + 6`. A Jungle source-directory path selects all three files; no explicit topological order is provided.

```sh
sh scripts/build-example.sh examples/multi-file
mkdir -p scratch/multi-file
cargo run -p ciq-dis -- all examples/multi-file/build/fr265/multi-file.prg scratch/multi-file/project
cargo run -p ciq-asm -- project scratch/multi-file/project scratch/multi-file/reassembled.prg -y developer_key.der
```

Experimentally observed:

- One PRG contains 13 ordered sections, a single 755-byte CODE payload, 13118-byte DATA, 332-byte DEBUG, and 45 line entries (722 bytes).
- `answer` starts at CODE `0x30`. App and view use `getmv` with module symbol `globals/Numbers` and member `answer`, followed by invocation. The module identity is a symbol binding, not a source-file address.
- Its DATA method field uses `value=code:answer`; DEBUG distinguishes all source filenames while line mappings share CODE labels.
- All supported sections export structurally; opaque sections retain raw bytes. Own-key reassembly is byte identical to Garmin output.
- Changing only the unique `ipush1 0x06` to `ipush1 0x07` in a copied CODE source and signing a new output changes the result to 47. Only that CODE operand and the developer signature change; all other sections are identical.

The reconstructed baseline prints and displays **46**. The assembly-modified PRG prints and displays **47**, proving the shared module call works from both files. Sessions ran through the `container-broker` plugin in the existing Garmin simulator container using Xvfb and `scripts/simulator-session.py --observe-seconds 15`. An initial five-second capture had no program output; it was not accepted as execution evidence. Fresh console and screen captures were inspected and preserved under `scratch/multi-file/simulator-46/` and `simulator-47/`.

## Barrel specimen and build details

[examples/number-barrel](../../examples/number-barrel/README.md) packages the same `Numbers.mc` module. [examples/barrel-app](../../examples/barrel-app/README.md) has only app/view source and imports the module through the barrel configuration.

```sh
sh scripts/build-barrel.sh examples/number-barrel
sh scripts/build-example.sh examples/barrel-app
mkdir -p scratch/barrels
# Save the archive-input PRG before the source-input build overwrites it.
cp examples/barrel-app/build/fr265/barrel-app.prg scratch/barrels/archive-input.prg
CIQ_JUNGLE_FILES=monkey-source.jungle sh scripts/build-example.sh examples/barrel-app
cp examples/barrel-app/build/fr265/barrel-app.prg scratch/barrels/source-input.prg
```

The app manifest uses `<iq:barrels><iq:depends name="Numbers" version="1.0.0" /></iq:barrels>`. The archive Jungle sets `base.barrelPath = ../number-barrel/build/number-barrel.barrel`; the source variant uses `[../number-barrel/monkey.jungle]`. The barrel manifest identifies the `Numbers` module and version 1.0.0.

SDK-specific issues observed and resolved:

- Barrel device discovery needs the same generated Java `user.home` / ConnectIQ symlink as app compilation. Without it, `fr265` is unrecognized.
- Default device personality discovery introduces `Devices/fr265/personality.mss` outside the barrel root. Packaging rejects it, even with `--no-gen-styles`. The code-only specimen overrides `fr265.personality = resources`, an in-project directory with no styles. This workaround is deliberately scoped to this library, not a general resource rule.
- An initial app dependency spelled `iq:barrel` fails manifest validation; the application dependency element is `iq:depends`.
- `barrelbuild` can print errors and exit successfully. Our helper requires a fresh output path and a nonempty output archive, avoiding a stale archive being mistaken for success.

Direct archive inspection with Python's standard `zipfile` finds a ZIP archive with four members: `manifest.xml`, `content/manifest.xml`, `barrel.jungle`, and `content/source/Numbers.mc`. The source member is byte identical to our authored source. The generated Jungle rewrites paths into `content/`. There is no compiled PRG/object member in this barrel.

Archive and source consumption each produce 13-section PRGs. **CODE (780 bytes), DATA (13156 bytes), SYMBOLS, HEADER, ENTRY_POINTS, IMPORTS, PERMISSIONS, EXCEPTIONS, and SETTINGS are byte identical between them.** Differences are DEBUG source paths (341 versus 335 bytes), corresponding line filename/function offsets, six additional trailing resource zeros in the source build, and developer signatures. All line PCs and source line numbers are identical. The six-byte DEBUG difference is compensated by resource padding, retaining equal 122988-byte PRGs; this supports the earlier observed alignment pattern without proving a universal padding requirement.

The archive-input PRG reconstructs byte identically, including own-key signatures, and its reassembled form prints/displays **46** in the simulator. Source-input PRG export/import is also byte identical; it was not separately simulated. The barrel adds generated `Numbers/Rez` initialization even though this specimen has no authored barrel resources; source library boundaries do not become separate CODE sections here.

## IQ package specimen

An SDK export builds a single-device `.iq` package using the archive Jungle:

```sh
# Use a compatible java on PATH (the build helper finds our ignored local JRE).
java -Duser.home="$PWD/examples/barrel-app/build/sdk-user-home" \
    -cp connectiq/bin/monkeybrains.jar com.garmin.monkeybrains.Monkeybrains \
    -f examples/barrel-app/monkey.jungle -e \
    -o scratch/barrels/barrel-app.iq -y developer_key.der -O 0
```

Experimentally observed: this IQ file has the 7z signature `37 7a bc af 27 1c`, **not ZIP framing**. It was read with the installed `libarchive.so.13` through an ignored Python ctypes helper (`scratch/barrels/inspect-iq.py`). With a 7-Zip command installed, `7zz l` and `7zz x -o<scratch-directory>` provide an alternative inspection/extraction procedure.

Members:

- `manifest.xml`: app/product metadata, device PRG path, part number `006-B4257-00`, and signature attributes `sig` / `sig2`.
- `manifest.sig2` (512 bytes) and `dev_key.pub` (550 bytes); these are public signing metadata, not the developer private key.
- `006-B4257-00/barrel-app.prg` (25820 bytes).
- `006-B4257-00/debug.xml` (78674 bytes), retaining external source mappings.

The packaged device PRG has **11 sections**: DEBUG and SYMBOLS are absent, and LINE_NUMBERS is an empty two-byte count. CODE and DATA remain identical to the debug barrel-app PRG. This is a directly observed export mode, not an assumption that every IQ package has one device or strips the same metadata.

After extraction to `scratch/barrels/bundle-device.prg`, our existing tools reconstruct it exactly and verify its developer signature. Its reconstruction runs and prints/displays **46**. Captures are preserved in `scratch/barrels/simulator-bundle-46/`.

We have not implemented IQ package writing, manifest signature regeneration, or archive extraction in the Rust CLI. Re-signing an extracted PRG alone does not update package-level signing metadata. The SDK owns package creation in this experiment; the PRG remains our assembler's unit of input/output.

## Artifacts

Hashes describe this build/key and may vary with SDK temp paths or archive timestamps. Byte-preservation checks compare each actual input against its reconstruction rather than requiring these hashes in future builds.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Multi-file original / reassembled | 122876 | `643ecadc2ffa7f92c0aae22767f08ec9c3c2029368a140085392dc5562110a88` |
| Multi-file modified to 47 | 122876 | `5a90a7908f99975acb2a1fe4bc7c1d28668822a66eff43675b0e8d4b629f2be4` |
| Barrel archive-input PRG / reassembled | 122988 | `7da6c191dc78b9a97b5a730ffaeb5ec5c2edd91aa3baf2e71526de3becc771fb` |
| Barrel source-input PRG / reconstructed | 122988 | `1b95e6359a7ec29723216b36b37def375764db5865e6137e94ddd83cd9afc316` |
| `number-barrel.barrel` | 1271 | `cd5cf17363b59566a6896a8dc063e0dfb748da490ad697e08d774876bc22a4df` |
| SDK IQ package | 13895 | `7213919d07f6c93af812a0ee369cd76f3c7bcd7c6e9dd8b028b5ad2cd9a18746` |
| Extracted device PRG / reconstructed | 25820 | `3dbbeb65176f10d89ac943eed889730f7112e6ed1af823e730963f85f482414a` |

Console capture hashes: multi-file 46 `dc46072dc3632c17b50f7f113f8fc49c52db54b50289fe3a3a1d9d829a0bcf20`; multi-file 47 `e8444a21c34165e1461e1b66eb6bf2aa28c1d8683df81e8495a28c710f9d74be`; both barrel runs `c940c1625b26f71ee873e8ac65455f482a0e2adfc7c0bd4dc028a16f4d9b926b`. Screen hashes: multi-file 46 `b26f567d1fba25ce0a60bc6a4ec315501ad16f0724ead4f903db239e5af58cff`; multi-file 47 `7e860f5f17748a0724eba1ef35600240a9d4b416f7fefb18e39b91c2f32e4fac`; both barrel runs `21b021639f51cae7cf11ad828f36c238f3f993921af36cf216176f4622ed8a64`.

## Consequences and next work

Interpretation: these simple source libraries are resolved into one program layout by Garmin's compiler. A source barrel is a build-time library package; an IQ file is a distribution archive holding device programs. Neither experiment demonstrates independently linked VM object files. Our assembler can continue using a complete PRG section project and shared references; no separate linker or relaxed placement behavior is justified yet.

Keep `at=` constraints. A future source include mechanism can collect declarations and resolve them together before layout. Investigate repeated function names in different modules before choosing label scope/qualification; current export aliases may fall back to offset names for ambiguity. Then investigate branches and exception ranges with deliberately small inputs before expanding automatic relocation. Multi-device IQ packaging, barrel annotations/resources/dependencies, and package signing remain open.

Validation used the existing built Rust CLI binaries because Cargo is unavailable on PATH in the current agent environment. No Rust source changed; the earlier 67-test/Clippy result was not rerun or newly claimed here. Shell syntax checks pass, actual SDK builds pass, signatures verify, and fresh simulator evidence covers each runtime result above. Downloads, packages, keys, and captures remain ignored; only independently authored examples, scripts, and documentation are intended for Git.
