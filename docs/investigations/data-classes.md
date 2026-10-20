# DATA class/module fields and method bindings

Date: 2026-10-08. Follow-up to [DATA strings](data-and-resources.md). Question:
which class/module fields connect the source-visible names to method addresses,
and can we inspect them without changing export semantics?

## Sources and evidence

Reviewed the existing pinned research checkouts as format evidence, then
implemented independently in Rust. No external code or dependency was
incorporated.

- **Source:** [MonkeyC Ghidra Plugin][ref-monkeyc-ghidra-plugin].
  **Revision/license:** `12ae55010721165b4ba98254d5e77db22eaa1b07`, Apache-2.0.
  **Relevant locations:** `doc/prg.md`, ClassDef v1/v2 tables and detailed field
  descriptions.

- **Source:** [monkeyc-optimizer][ref-monkeyc-optimizer]. **Revision/license:**
  `2920dc88c23caadded9f66ee0613a7de6ab2d1ea`, MIT. **Relevant locations:**
  `src/readprg/data.ts`, class framing and method binding.

- **Source:** Local specimens. **Revision/license:** SDK 9.2.0, `fr265`, `-O 0`,
  debug builds. **Relevant locations:** Existing Minimal Console / Hello World
  PRGs and debug XML; hashes below.

**Source-derived:** header presence bits select extends/statics/parent/module
words; field keys encode a 24-bit symbol ID; type 6 is a method; v1 types are
embedded in the key and v2 types are explicit. The reference overview's
incorrect field widths were already documented: detailed descriptions and
optimizer traversal imply eight/nine bytes, matching actual v2 records. V1
support remains synthetic-test validation only; neither specimen has v1 classes.

**Experimentally observed:** the independent walker consumes both DATA payloads
completely and retains the previous string counts. Minimal Console has 128 v2
class/module records and 52 strings; Hello World has 129 classes and 53 strings.

| Type byte | Minimal Console fields | Hello World fields |
|---|---:|---:|
| `00` | 27 | 27 |
| `01` | 72 | 72 |
| `02` | 1 | 1 |
| `03` | 216 | 216 |
| `06` | 131 | 134 |
| `07` | 2 | 4 |
| `09` | 16 | 16 |
| `0a` | 251 | 251 |

Counts establish occurrences, not complete value semantics for each type.

## Observed bindings

Offsets are DATA-relative unless explicitly CODE-relative. Names resolve from
each PRG's SYMBOLS section.

- **Specimen:** Minimal Console. **Class record:** `0x47b`. **Field offset:**
  `0x491`. **Symbol/name:** `0x80` / `hello`. **Type/value:** `06` /
  `0x10000024`. **Interpretation:** method at CODE `0x24`.

- **Specimen:** Minimal Console. **Class record:** `0x47b`. **Field offset:**
  `0x488`. **Symbol/name:** `0x7f` / `MinimalConsoleApp`. **Type/value:** `07` /
  `0x4b5`. **Interpretation:** matches a DATA class record.

- **Specimen:** Minimal Console. **Class record:** `0x4b5`. **Field offset:**
  `0x4e1`. **Symbol/name:** `0x80029f` / `onStart`. **Type/value:** `06` /
  `0x10000041`. **Interpretation:** method at CODE `0x41`.

- **Specimen:** Hello World. **Class record:** `0x47b`. **Field offset:**
  `0x49a`. **Symbol/name:** `0x81` / `probe`. **Type/value:** `06` /
  `0x10000030`. **Interpretation:** method at CODE `0x30`.

- **Specimen:** Hello World. **Class record:** `0x47b`. **Field offset:**
  `0x488`. **Symbol/name:** `0x7f` / `HelloWorldApp`. **Type/value:** `07` /
  `0x528`. **Interpretation:** matches a DATA class record.

- **Specimen:** Hello World. **Class record:** `0x528`. **Field offset:**
  `0x554`. **Symbol/name:** `0x80029f` / `onStart`. **Type/value:** `06` /
  `0x100000e3`. **Interpretation:** method at CODE `0xe3`.

The `hello`, `probe`, and `onStart` targets agree with SDK debug metadata. Every
ordinary CODE method reference in both listings points to a decoded instruction
boundary. This does not establish general function extent or calling convention.

**Correction to a source assumption:** the plugin labels header
extends/statics/parent/module uniformly as symbol IDs. Minimal Console's app
class has `extends=0x40000119`, while its `AppBase` field has type 7 and the
same value. This does not resolve in SYMBOLS. Parent `0x8002e6` does resolve to
`globals`. We preserve raw header references and avoid assigning them one
universal address kind. Internal API-class mapping remains unverified.

**Unknown:** field flags/app flags, module-value namespace, unsupported address
spaces, native bindings, mutation/serialization, and relocation. Numeric values
are preserved; matching known record boundaries provides bounded inspection
annotations only.

## Implementation and validation

[Local specification](../specs/formats/data-classes.md) defines the independent parser
and listing. `ciq-prg::data::classes` supplies an ordered record model. The
existing DATA string walker shares its framing to avoid diverging header/count
calculations. `ciq-dis classes` supplies read-only output; complete export
includes a `.classes.txt` inspection sidecar without changing authoritative
assembly.

Five new tests cover all 16 v2 presence combinations and truncations, v1/v2
method equivalence, unknown types/magic/flags and unsupported address spaces,
typed annotations and invalid CODE boundaries, and ignoring edited sidecars on
project import. The complete workspace now has 47 passing Rust tests; Clippy
passes with warnings denied. No new runtime execution occurred: Podman is absent
in the current container, and simulator checks remain host-side work.

Reproduce with new export paths:

```sh
cargo run -p ciq-dis -- classes examples/minimal-console/build/fr265/minimal-console.prg
cargo run -p ciq-dis -- classes examples/hello-world/build/fr265/hello-world.prg
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/class-project-minimal
cargo run -p ciq-asm -- project scratch/class-project-minimal scratch/class-project-minimal.prg
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/class-project-hello
cargo run -p ciq-asm -- project scratch/class-project-hello scratch/class-project-hello.prg
cmp examples/minimal-console/build/fr265/minimal-console.prg scratch/class-project-minimal.prg
cmp examples/hello-world/build/fr265/hello-world.prg scratch/class-project-hello.prg
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Both complete 13-section real projects reconstruct byte for byte:

- Minimal Console: SHA-256
  `cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7`.
- Hello World: SHA-256
  `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7`.

Local experiment artifacts are under ignored `scratch/classes/`; the temporary
`inspect.py` independently tabulates specimen records. [ref-monkeyc-ghidra-plugin]: https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin/tree/12ae55010721165b4ba98254d5e77db22eaa1b07
[ref-monkeyc-optimizer]: https://github.com/markw65/monkeyc-optimizer/tree/2920dc88c23caadded9f66ee0613a7de6ab2d1ea
