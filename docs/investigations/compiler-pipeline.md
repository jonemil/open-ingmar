# Native and textual compiler pipeline checkpoint

2026-10-10. The user requested exercising both paths before growing the
compiler, so IR design choices cannot silently make text resumption or native
composition awkward. Scope: the existing constant-return function subset and
full existing ordinary CODE assembly syntax; no new language expressions,
general relocation, or independent application construction.

## Implementation and observations

Introduced `ciq-assembly` as the shared model/parser/printer/encoder layer,
depending only on our standard-library-only container/instruction crates.
`ciq-asm` delegates its existing text APIs to it and keeps PRG/signing work.
`ciq-compiler` lowers directly to its typed `Program`, rather than printing a
string and reparsing to encode. Semantic IR now has a
[provisional versioned text format](../specs/compiler-ir-text.md), with the same
validator used by typed construction and source lowering.
[The assembly model](../design/assembly-model.md) shares structural checks and
encoding between native/text inputs. No external implementation was copied or
newly imported.

The body remains one integer definition and a return of that value. This
deliberately strict validator rejects IR that would require scheduling, locals,
or control flow the backend cannot yet handle. Optional source spans survive
serialization; they are metadata about the original source, not a requirement
that native and parsed assembly line provenance match.

Six new tests cover: all 128 accepted literal pipelines preserving
IR/items/labels/bytes; alternate value IDs and edited IR; invalid typed/text IR;
typed assembly references/declarations/layout round trips; invalid native
assembly and external binding resolution; CLI stop/resume, identical binaries,
invalid-stage/IR rejection, and output preservation. Existing
full-project/signing tests exercise the refactored assembly entry points. All
122 workspace tests pass with Rust 1.99; formatting and Clippy with warnings
denied pass.

## Reproduction and compatibility checks

```sh
mkdir -p scratch/pipeline
cargo build -p ciq-compiler -p ciq-asm -p ciq-dis
target/debug/ciq-compiler --emit ir examples/compiler-probe/probe.mc scratch/pipeline/probe.ciqir
target/debug/ciq-compiler --emit code examples/compiler-probe/probe.mc scratch/pipeline/direct.bin
target/debug/ciq-compiler --from ir --emit code scratch/pipeline/probe.ciqir scratch/pipeline/via-ir.bin
target/debug/ciq-compiler --from ir scratch/pipeline/probe.ciqir scratch/pipeline/probe.mcasm
target/debug/ciq-compiler --from assembly --emit code scratch/pipeline/probe.mcasm scratch/pipeline/via-assembly.bin
cmp scratch/pipeline/direct.bin scratch/pipeline/via-ir.bin
cmp scratch/pipeline/direct.bin scratch/pipeline/via-assembly.bin
```

Observed: all three CODE payloads are identical (`35 01 38 29 16`). Fresh
exports/reassemblies of the SDK Hello World and Module Names PRGs are byte
identical after the parser/encoder refactor. Those experiments used
`ciq-dis all`, `ciq-asm project`, and `cmp` with new
`scratch/pipeline/hello-project` and `module-project` directories.

To test editing and resuming IR, replace `.integer %0 41` with `.integer %0 43`,
preserving the return's `%0`, save as `edited.ciqir`, then:

```sh
target/debug/ciq-compiler --from ir scratch/pipeline/edited.ciqir scratch/pipeline/probe-43.mcasm
target/debug/ciq-compiler --from ir --emit code scratch/pipeline/edited.ciqir scratch/pipeline/probe-43.bin
```

For runtime reproduction, first prepare the 749-byte CODE/7804-byte RESOURCES
`scratch/compiler/hello-41-project` using
[the initial compiler procedure](first-compiler.md). Copy that project to a new
`scratch/pipeline/hello-43-project`. Replace only its appended `probe:` body
(starting CODE `0x2e8`) with the generated `probe-43.mcasm`; keep
`probe_original`, old line PCs, and manifest lengths unchanged. There are no
length changes relative to that prepared scaffold.

```sh
target/debug/ciq-asm project scratch/pipeline/hello-43-project scratch/pipeline/hello-43.prg -y keys/developer_key.der
target/debug/ciq-asm verify scratch/pipeline/hello-43.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh --prg scratch/pipeline/hello-43.prg
```

Actual execution: SDK/simulator 9.2.0 / `fr265`, existing Garmin container
through container-broker,
`xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py --prg scratch/pipeline/hello-43.prg --device fr265 --observe-seconds 8`.
Console prints greeting and `43`; inspected screen shows `Hello world!` and
`43`. The generated raw binary exactly matches the appended PRG CODE bytes.
Signatures verify against our external key. Logs/screens remain ignored in
`scratch/pipeline/simulator-43/`.

## Limits and next work

This establishes the representation boundaries for a complete function, not the
final IR design. `.ciqir` syntax, operation set, and APIs remain provisional.
Assembly comments/formatting and parsed line numbers are not serialized as
semantic content. The shared assembly model covers ordinary CODE and its
existing inline declarations; full DATA/class/resource/project models remain in
their current layers. Raw CODE output still needs the PRG scaffold, metadata,
and signing to run. Branches, exception handling, and generalized stack
scheduling remain unsupported.

Next extend the semantic IR and both readers/printers together when adding
arithmetic. Keep tests comparing native output with output resumed from each
text boundary. Resource/project generation can evolve independently.

- **Artifact:** Native / IR-resumed / assembly-resumed `41` CODE. **SHA-256:**
  `cf2fd8b7314d28594165c1ca7b7f56207ac2c911abbd86632dc91b5757923680`.

- **Artifact:** Saved IR with original source spans. **SHA-256:**
  `814a64efc63f2bcd89e15c2a343a51f08492a6f2f0ce26dd6355d8166dde87fd`.

- **Artifact:** Edited `43` CODE. **SHA-256:**
  `f6f6df45c153e7695fa1ab3ef8dd7b462df16268b18983db801dbf2de94a41f2`.

- **Artifact:** Signed `43` PRG. **SHA-256:**
  `f11a38bbd04bb46e57775a873a475ad2f485052b51ac51fd8a8101d04fdddbf8`.
