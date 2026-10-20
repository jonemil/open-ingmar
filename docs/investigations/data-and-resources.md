# DATA strings and initial resource inspection

Date: 2026-10-08. Questions: how do string records differ from resource strings,
can they become assembly input, and what remains opaque?

## Sources and scope

This work independently implements the repository's observed format. No external
code was copied or added as a dependency.

- **Source:** [MonkeyC Ghidra Plugin][ref-monkeyc-ghidra-plugin].
  **Revision/license:** `12ae55010721165b4ba98254d5e77db22eaa1b07`, Apache-2.0.
  **Relevant locations:** `doc/prg.md`: DATA string/class records and RESOURCES
  category/language/resource tables.

- **Source:** [monkeyc-optimizer][ref-monkeyc-optimizer]. **Revision/license:**
  `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT. **Relevant locations:**
  `src/readprg/data.ts`: record traversal; `src/readprg/symbols.ts:parseSymbol`:
  length/content/terminator traversal.

- **Source:** Local specimens. **Revision/license:** SDK 9.2.0, `fr265`, `-O 0`,
  debug builds. **Relevant locations:** Minimal Console and Hello World;
  compiler inputs/build hashes in [the bootstrap](rust-bootstrap.md).

**Source-derived:** string records have a tag and u16 length. Class framing uses
eight-byte fields in v1 and nine-byte fields in v2. The plugin's v1 overview
says N×4 bytes, but its detailed key/value fields and the optimizer imply eight
bytes per field. Its v2 overview says N×5 while its detailed key/type/value
fields imply nine. Our actual v2 specimens traverse exactly to EOF with
nine-byte fields; overview sizes are not adopted.

Address-profile clarification, 2026-10-10: the same pinned plugin's
`doc/prg.md`, section 1.3, lists prefixed addresses such as `0x00xxxxxx` for
DATA and `0x10xxxxxx` for CODE. Our zero-prefix, 24-bit DATA offset policy was
adopted from these patterns. However, the text describes the selector as the
"high nibble," leaving the remaining nibble's role unclear. The patterns do
not by themselves establish an eight-bit selector or a 16 MiB DATA maximum.
Our specimens confirm small zero-prefix string addresses and `0x10`-prefixed
method addresses, not behavior at offsets beyond `0x00ffffff`. The current
limit is a conservative implementation profile; larger offsets and the exact
selector/offset boundary remain unverified.

A subsequent [boundary experiment](string-address-boundary.md) on the same date
loaded a DATA string at `0x01000000` in the simulator with an increased memory
allowance. This disproves the universal 24-bit offset ceiling; the full
selector/offset rule remains unresolved. Current authoring restrictions must
not be presented as established VM limits.

**Experimentally observed:** a sequential walk of Minimal Console DATA finds 128
c2 class/module records and 52 string records. Hello World finds 129 c2 records
and 53 strings. No c1 or JSON records occur. Class records are retained as
opaque bytes despite known boundaries. String names include generated SVG
filesystem paths and option values, not just source literals. This clarifies
that actual path strings can occur in DATA, independently of path-like module
names in SYMBOLS. Their presence does not establish runtime filesystem loading
behavior.

No scan for arbitrary `01` bytes is used. When known record traversal fails,
validated `news` references can still anchor string extraction. Unsupported
content and address spaces remain raw/numeric. Repeated references retain one
record, and overlaps reject structured export.

## Literal and resource observations

All offsets below are relative to their own section payload unless labeled
CODE/file.

| Specimen | CODE news offset | DATA record offset | Content |
|---|---|---|---|
| Minimal Console | `0x38` | `0x3279` | `Hello!` |
| Hello World | `0xa2` | `0x32cf` | `Hello world!` |
| Hello World | `0xef` | `0x32b7` | `Hello from Monkey C!` |

Minimal Console literal bytes: `01 00 06 48 65 6c 6c 6f 21 00`. DATA payload
starts at file `0x5f`; its literal's tag therefore starts at file `0x32d8`.

**Experimentally observed:** both RESOURCES payloads have top-level entries for
Strings (`0x008000a2` → `0x1c`) and Drawables (`0x008000a3` → `0x12`). The
drawable index points to a container at `0x26`; its contents remain opaque. The
language table at `0x1c` contains `0x008002e7` → `0x1e3c`. That final table
references the app-name string at `0x1e46`:

| Specimen | Resource ID | Resource content | Source input |
|---|---|---|---|
| Minimal Console | `0x83` | `Minimal Console` | `resources/strings.xml`, `AppName` |
| Hello World | `0x84` | `Hello World` | `resources/strings.xml`, `AppName` |

The app-name resource IDs resolve to `AppName` in each PRG's local symbol table.
These IDs differ from DATA addresses. The observed language ID resolves to
`<valyrian>` in SYMBOLS; `0x008002eb` resolves to `<eng>`. We do not infer
language selection/fallback behavior from those names. This is a reproducible
difference from the plugin's English example, not evidence that the application
runs in a particular human language.

## Assembly design and reproduction

Implemented [DATA source](../specs/formats/data-strings.md) combines `.string` records
with pinned `.bytes` opaque spans. Manifest `data` encoding chooses
this source as authoritative. Code resolves `news &alias`; symbol references
remain `@alias`. Strings in authored standalone DATA can omit placement for
source-order allocation. Exports pin layout. Template assembly supports inline
pinned strings and same-length edits. RESOURCES remains raw;
[resource strings](../specs/formats/resources.md) have read-only inspection.

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/string-project-minimal-v2
cargo run -p ciq-asm -- project scratch/string-project-minimal-v2 scratch/string-project-minimal-v2.prg
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/string-project-hello-v2
cargo run -p ciq-asm -- project scratch/string-project-hello-v2 scratch/string-project-hello-v2.prg
cmp examples/minimal-console/build/fr265/minimal-console.prg scratch/string-project-minimal-v2.prg
cmp examples/hello-world/build/fr265/hello-world.prg scratch/string-project-hello-v2.prg
```

Both 13-section projects reconstruct byte for byte, including DATA, RESOURCES,
symbols, and signing material. SHA-256 remains
`cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7` for Minimal
Console (122188 bytes) and
`8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` for Hello
World (122668 bytes). DATA payload lengths are 12974 and 13066.

Additional commands exercised:

```sh
cargo run -p ciq-dis -- data-source examples/minimal-console/build/fr265/minimal-console.prg > scratch/minimal-data.mcasm
cargo run -p ciq-asm -- data scratch/minimal-data.mcasm scratch/minimal-data.bin
cargo run -p ciq-dis -- code-symbolic examples/minimal-console/build/fr265/minimal-console.prg > scratch/minimal-with-strings.mcasm
cargo run -p ciq-asm -- examples/minimal-console/build/fr265/minimal-console.prg scratch/minimal-with-strings.mcasm scratch/minimal-with-strings.prg
cargo run -p ciq-dis -- resource-strings examples/minimal-console/build/fr265/minimal-console.prg
```

The standalone DATA equals the original payload, and symbolic template assembly
equals the original complete PRG. Replacing the inline declaration's `Hello!`
with `Hallo!` and assembling a separate output changes exactly one file byte,
`0x32dc` (`65` → `61`). CODE, record length/placement, opaque DATA, resources,
and all other sections remain identical. This output still contains the old
signature; no runtime acceptance is claimed.

## Validation and remaining work

Forty-two workspace tests and Clippy with `-D warnings` pass. New coverage
includes record-boundary walking, c1 synthetic framing, opaque spans, repeated
and overlapping references, UTF-8 byte lengths and escapes, pinned placement,
deterministic allocation, truncation and malformed data, typed references,
authoritative project string edits, template edits, duplicate DATA/raw fallback,
manifest version compatibility, resource index traversal and malformed/cyclic
indexes.

UTF-8 serialization is based on the local format and parser checks; controlled
non-ASCII Garmin compiler specimens are still desirable. The allocator preserves
declaration order and rejects gaps. No length-changing relocation, signatures,
resource assembly, class-field decoding, or simulator execution was added.

Next targets: decode class/module fields and their reference kinds; validate
resource language/default rules and indexed resource containers with controlled
examples; implement lossless resource assembly; separately complete own-key
signing and observable simulator edits. JSON/container boundaries differ in the
current reference descriptions and are intentionally left unsupported until
tested.

[ref-monkeyc-ghidra-plugin]: https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin/tree/12ae55010721165b4ba98254d5e77db22eaa1b07
[ref-monkeyc-optimizer]: https://github.com/markw65/monkeyc-optimizer/tree/2920dc88c23caadded9f66ee0613a7de6ab2d1ea
