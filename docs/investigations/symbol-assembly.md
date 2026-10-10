# Symbol assembly and named-code round trips

Date: 2026-10-08. Question: can symbols become authoritative assembly input,
with readable code references and optional layout constraints?

## Evidence and implementation

The binary format and pinned external-source attribution remain in
[the symbol decoding investigation](symbols.md) and
[initial bootstrap](rust-bootstrap.md). This work used our documented format and
independent Rust implementation; no additional external code or dependencies
were introduced.

Implemented the shared `ciq_prg::symbols::source` declaration model, parser,
printer, and serializer. The decoder now retains table entry indexes in addition
to IDs and name offsets. `ciq-asm` resolves typed `@alias` operands, including
forward references and declarations in another manifest section. `ciq-dis`
exports pinned symbol declarations and named code only after independently
serializing the table and verifying exact equality with its payload.
Unrepresentable or ambiguous tables retain raw payloads.

This implementation follows
[the symbol assembly specification](../specs/symbol-assembly.md). The allocation
policy is a local design choice, not a claim about Garmin's compiler. Numeric
IDs stay mandatory; API binding and allocation of application-local IDs are not
inferred from placement freedom. The version 2 manifest introduces authoritative
`symbols` encoding while retaining version 1 import support.

## Reproduction and observations

Inputs are the existing SDK 9.2.0, `fr265`, `-O 0`, debug Minimal Console and
Hello World builds. Build settings and key handling are recorded in
[the bootstrap](rust-bootstrap.md). The experiment introduced no changes to
their Monkey C sources.

```sh
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/symbol-assembly-minimal-v2
cargo run -p ciq-asm -- project scratch/symbol-assembly-minimal-v2 scratch/symbol-assembly-minimal-v2.prg
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/symbol-assembly-hello-v2
cargo run -p ciq-asm -- project scratch/symbol-assembly-hello-v2 scratch/symbol-assembly-hello-v2.prg
cmp examples/minimal-console/build/fr265/minimal-console.prg scratch/symbol-assembly-minimal-v2.prg
cmp examples/hello-world/build/fr265/hello-world.prg scratch/symbol-assembly-hello-v2.prg
```

**Experimentally observed:** each export has 13 ordered sections and a version 2
manifest. The symbol section is selected as
`symbols 010-symbols-5717b015.mcasm`. Minimal Console's 3,224 entries serialize
to 95,991 bytes; Hello World's 3,225 entries serialize to 96,013 bytes. Each
reconstructed PRG is byte-identical to its original:

| Specimen | PRG bytes | Reconstructed SHA-256 |
|---|---|---|
| Minimal Console | 122188 | `cd361b76dc29e828d43f0511df3bebd96dc63ce6b7be61b70e2be8e1092d95e7` |
| Hello World | 122668 | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |

Minimal Console's instruction at CODE+`0x2e` now exports as:

```text
getmv @Toybox_System @println # CODE+0x00002e; module="Toybox_System"; member="println"
```

The next string load remains `news 0x00003279` at CODE+`0x38`; data pointers are
not symbol IDs.

Standalone commands were also exercised:

```sh
cargo run -p ciq-dis -- symbol-source examples/minimal-console/build/fr265/minimal-console.prg > scratch/minimal-symbols.mcasm
cargo run -p ciq-asm -- symbols scratch/minimal-symbols.mcasm scratch/minimal-symbols.bin
cargo run -p ciq-dis -- code-symbolic examples/minimal-console/build/fr265/minimal-console.prg > scratch/minimal-code-symbolic.mcasm
cargo run -p ciq-asm -- examples/minimal-console/build/fr265/minimal-console.prg scratch/minimal-code-symbolic.mcasm scratch/minimal-code-symbolic.prg
```

**Experimentally observed:** the standalone symbol payload equals the original
SYMBOLS bytes, and the symbolic-code/template output equals the original
complete PRG. These are parser/serializer and binding checks, not runtime
execution tests.

## Validation and limits

Thirty-one workspace tests and Clippy with `-D warnings` pass. New tests cover
deterministic allocation, independent table/name constraints, shared pinned
records, escaping including Unicode/control characters and literal `#`, layout
conflicts, duplicate aliases/IDs, required IDs, forward references, typed
operand rejection, version compatibility, authoritative symbol edits, alias
renaming, and raw fallback for orphan name records and duplicate symbol
sections. Existing raw section/path validation and round-trip tests continue to
pass.

Changing a fixture's alias and matching code reference preserves all bytes.
Changing one stored name to another of equal byte length changes the SYMBOLS
payload while preserving CODE. A pinned name edit that leaves an unsupported gap
is rejected. This establishes that the structured source is consumed, rather
than remaining an inspection listing.

No symbol ID allocation, general data relocation, include directive, signature
generation, or new simulator run was added. Greedy deterministic name placement
may reject a set of partial constraints even if some different packing
permutation could satisfy them. Unreferenced name records remain raw because the
current declaration language has no standalone name-record directive.

Next extend data/string references with the same content/placement distinction,
and perform the planned simulator and same-length bytecode-edit checks. Preserve
opaque records until their references can be relocated safely.
