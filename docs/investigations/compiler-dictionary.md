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

The nonempty literal still stops CODE decoding at `0x53`, with bytes
`42 00 00 32 ac`. DATA walking reports `unsupported constant-array framing`.
These are outstanding opcode/constant-format gaps, not accepted raw fallback for
an implemented language feature. Their record/operand semantics remain to be
established; do not infer later instruction boundaries by scanning byte values.

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
sections with `ciq-dis all`; expect the recorded CODE/DATA gaps until implemented.
Sources, build logs, console captures, and exports are retained under
`scratch/compiler-dictionary-probe/`.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Dictionary.html
