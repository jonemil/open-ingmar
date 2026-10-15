# Dictionary construction and initial operations

Date: 2026-10-15. SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: how are Dictionaries created, accessed, and represented in bytecode?
Inputs are our authored Monkey C sources compiled with the SDK.

## Reference and broader probe

**Officially documented:** [Dictionary][api] is an Object subclass available at
API 1.0.0, with potentially heterogeneous Object keys/values. Missing lookups
return Null. `size`/`isEmpty` are API 1.0.0; the class's `toString` override is
1.0.1. Iteration/string order is not guaranteed.

**Experimentally observed:** `scratch/compiler-dictionary-probe/app.mc` creates
`{}` and `{"one" => 1, "two" => 2}`, then exercises indexed reads/writes,
`get`, `put`, `hasKey`, `remove`, `size`, and `isEmpty`. Its SDK simulator output
is `0, true, 1, null, 3, true, null, false, 2`, one value per line. Storing Null
retains the key; removing it makes `hasKey` false. This probe does not establish
all key equality/hashing rules or arbitrary heterogeneous value behavior.

The empty allocation has `ipushz` at CODE+`0x27`, byte `2f` at `0x28`, then
`lputv 2`. Treating `0x2f` as the operand-free `newd` permits decoding past this
point. An external corroboration is the Ghidra plugin's `newd` entry in
`doc/opcodes.md` and `data/languages/MonkeyC.sinc`, at the revision pinned in the
[instruction research](instruction-semantics.md). No implementation was copied.
The SDK SYMBOLS table identifies `isEmpty` as `0x008000b8`; `size` uses the
already supported `0x008000ae`.

The nonempty literal uses `42 00 00 32 ac` at CODE+`0x53`, referencing a
Dictionary record at DATA+`0x32ac`. The pinned Ghidra research names this opcode
`hpush`. The constant's local string pool uses offsets 0 and 6 for `one`/`two`;
the value block has root `0x0b` and two entries. Additional SDK specimens under
`scratch/dictionary-constants/` cover numeric-only entries, Char/Boolean/Number
mixing, shared string references, empty strings, and UTF-8. The mixed-key output
record reorders source entries; preserve the encoded order without inferring a
source iteration guarantee.

All five specimens, including the original probe, export structured CODE/DATA,
reconstruct byte-for-byte, and re-export identically. Verdicts/artifacts are under
`scratch/dictionary-constants/verified/`; the earlier opcode/DATA gaps are closed
for this flat scalar profile. Unsupported nested values and other scalar types
still stop the complete walk.

An edit of the original probe's Dictionary entry changes `one` from 1 to 41.
After project assembly and own-key signing, the simulator prints
`0, true, 41, null, 3, true, null, false, 2`, one per line. The other output is
unchanged. Artifacts are `scratch/dictionary-constants/edited*`. This establishes
editable constant contents, not fresh-versus-shared repeated-load behavior.
The [constant format](../specs/data-dictionaries.md) records the verified framing.

## Empty-container profile

The [tracked example](../../examples/compiler-dictionary/README.md) isolates
empty `{}` construction, local reference copies/reassignment, typed Dictionary
arguments/results, native observers, chained calls, and repeated loop allocation.
Both SDK and generated simulator runs match all ten expected lines. The launch
provides Null callback state, so its guarded observer is checked structurally,
not executed against a populated launch Dictionary. Unguarded access is rejected
by compiler tests.

The console checker confirms source/saved-IR parity, structured CODE/DATA export,
exact PRG reconstruction, and stable re-export for both programs. No DATA format
extension is needed for this profile. It uses `ipushz; newd`, plus existing local
and member-call instructions. The codec and assembler/disassembler now recognize
operand-free `newd`. Artifacts: `scratch/check-dictionary-empty/`.

**Interpretation:** keep the first compiler container opaque until key/value
contracts and constant encodings are established. Current contracts are in the
[source profile](../specs/compiler-subset.md#dictionary-values) and
[IR allocation format](../specs/compiler-ir-text.md#dictionary-allocation).

## Reproduction

Use the shared console scaffold with its Jungle source path set to
`../../examples/compiler-dictionary`. Build `scratch/compiler-dictionary`, then
prepare the [console checker](../../scripts/README.md#console-specimen-check)
with `--sdk-prg scratch/compiler-dictionary/build/fr265/compiler-dictionary.prg`
and a fresh output directory. Run simulation in the SDK environment. The
example's README has standalone compile/sign/run commands.

For the broader probe, use the operations described above in an SDK-only
`onStart` with `System.exit()`. Build and run it separately, then export all
sections with `ciq-dis all` and reconstruct with `ciq-asm project`. Both exports
should be structured for the tested scalar records.
Sources, build logs, console captures, and exports are retained under
`scratch/compiler-dictionary-probe/`.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Dictionary.html

To reproduce the edit, copy the original probe's exported project and change
`.entry string=&pool_00000000 number=0x00000001` to use `number=0x00000029`.
Assemble the copied project, sign the changed PRG with the developer key, and
run it in a fresh simulator capture directory. Compare its complete console log
against the nine values above; the process exit alone is insufficient.
