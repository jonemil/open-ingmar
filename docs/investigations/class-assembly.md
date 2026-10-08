# Lossless class assembly and relocation inventory

Date: 2026-10-08. Question: expose editable class/method fields without losing raw bits, and identify dependencies before allowing a function's size to change.

## Evidence and implementation boundary

Format evidence remains the pinned sources in [the class inspection investigation](data-classes.md): MonkeyC Ghidra Plugin revision `12ae55010721165b4ba98254d5e77db22eaa1b07` (Apache-2.0) and monkeyc-optimizer revision `2920dc88c23caadded9f66ee0613a7de6ab2d1ea` (MIT). Additional inspected optimizer locations: `src/readprg/data.ts` (`fixupData`), `header.ts`, `linenum.ts`, and `exceptions.ts`. Plugin `doc/prg.md` supplies the header, entry-point, and exception layouts. These remain research references, not runtime dependencies; all new format implementation is independent Rust against [our specification](../specs/data-classes.md).

Implemented `.class` / `.field` / `.endclass` blocks in the shared DATA source. They preserve complete keys, explicit/embedded types, ordered duplicate fields, presence versus zero, app flags, and raw relationship words. Supported full exports now choose this authoritative source; older opaque-span DATA sources still import unchanged. The inspection sidecar remains ignored on import. No new project encoding or manifest version is needed; this extends the version 3 DATA source grammar.

Placement and values remain explicit numeric constraints. This first step makes method targets editable without introducing unverified name allocation or code relocation. Class and method aliases/code labels will need typed resolution later.

## Artifact verification

Specimens: existing SDK 9.2.0 debug / `-O 0` builds for `fr265`. Exact inputs and hashes are in [the signing record](developer-signing.md).

```sh
mkdir -p scratch/class-assembly
cargo run -p ciq-dis -- all examples/minimal-console/build/fr265/minimal-console.prg scratch/class-assembly/minimal-project
cargo run -p ciq-asm -- project scratch/class-assembly/minimal-project scratch/class-assembly/minimal.prg
cargo run -p ciq-dis -- all examples/hello-world/build/fr265/hello-world.prg scratch/class-assembly/hello-project
cargo run -p ciq-asm -- project scratch/class-assembly/hello-project scratch/class-assembly/hello.prg
```

Python byte comparison confirms both entire output files equal their inputs. Minimal Console exports 128 classes and 52 strings; Hello World exports 129 classes and 53 strings. Neither specimen needs opaque class spans after a complete supported walk. Existing signing sections remain unchanged because reconstruction is byte identical.

Three added Rust tests cover all 16 v2 presence combinations and v1 round trips, repeated keys and unknown types, malformed block structure/widths, v1 type mismatch, count overflow, constrained layout conflicts, and a method-value edit that changes exactly the intended byte. V1 remains synthetic coverage; real specimens contain v2 only. The complete workspace passes 55 Rust tests and Clippy with warnings denied; formatting and diff checks pass. No new runtime experiment is claimed for edited class declarations.

## Code relocation inventory

This inventory separates source-derived candidate rules from measured specimen data. Ignored `scratch/class-assembly/relocation-inventory.json` records local payload observations.

| Location | Evidence | Implication |
|---|---|---|
| DATA method fields | Verified type 6 values `0x10xxxxxx`, instruction-boundary targets, previous SDK debug comparisons | Rebind each affected method to its new CODE offset; other raw values must not be treated as pointers |
| LINE_NUMBERS (`0xc0de7ab1`) | Source-derived u16 count and 16-byte entries: PC, file, symbol, line (u32 each). Both payload lengths exactly match this layout | Preserve address-space prefix; relocate PCs and determine a policy for mappings inside replaced code |
| Header background/glance CODE offsets | Source-derived u32 values at payload offsets 8 and 25 | Nonzero offsets need typed relocation; both specimens have zero values, so nonzero behavior is not experimentally established |
| EXCEPTIONS | Source-derived u16 count plus nine-byte entries: three u24 CODE offsets | Relocate try ranges and handlers; both specimens contain only `00 00`, so nonempty interval semantics remain unverified |
| CODE control flow | Branches/direct calls may embed code-relative references; current specimens use the supported branch-free subset and symbolic member calls | Decode supported relocation kinds before admitting programs with those instructions |
| ENTRY_POINTS | Source-derived records select module/class IDs, not direct CODE PCs | Do not blindly relocate every u32 that resembles a code offset; verify binding namespaces with controlled specimens |
| DEBUG and extended/background sections | Not fully specified; DEBUG is 225 bytes in both current examples | Investigate before claiming complete general relocation; opaque data may carry affected references |
| Signatures | Verified coverage of section wrappers/payloads | Regenerate after relocation, including changed lengths |

Measured line tables: Minimal Console has 34 entries (546-byte payload); Hello World has 45 (722 bytes). First Hello World PCs include `0x10000004`, `0x10000010`, `0x10000024`, `0x10000030`, `0x1000003a`, `0x1000003d`. These agree with known function/instruction offsets. Both 33-byte versioned headers begin `00 06 00 02` and contain zeros thereafter. Both ENTRY_POINTS payloads have count 1 and length 38, consistent with one 36-byte record; this is structural agreement, not full semantic validation.

## Next experiment

Compile controlled SDK variants where one small function grows and another follows it. Compare decoded class method targets, line PCs, exception/header fields, and debug payloads. Keep filenames/build options stable where possible so incidental debug differences do not masquerade as relocation evidence. Then implement typed code labels and explicit relocation rules for the verified subset; reject unknown affected reference formats. Current equal-length project/template restrictions remain in force.
