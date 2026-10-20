# Native and textual compiler pipeline checkpoint

2026-10-10. SDK/simulator 9.2.0, `fr265`. Question: do native, saved-IR,
and saved-assembly paths produce equivalent CODE and permit an edited function
to execute? The experiment used a constant-return function appended to an SDK
scaffold; it did not test independent application generation.

All 128 accepted literal inputs agreed across native/text models and CODE.
Alternate value IDs/spans, malformed IR/assembly, external bindings, CLI stage
resumption, and output preservation were checked. Shared-model responsibilities
and current invariants are specified in [assembly](../specs/assembly/README.md#model-and-layout) and
[IR](../specs/ir/README.md#model-and-validation).

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

## Artifacts and limits

This runtime check establishes the recorded scaffold/function path. It does not
establish general relocation, all VM stack behavior, or compatibility beyond the
stated SDK/device. Current language and format support belong in the specs.

- **Artifact:** Native / IR-resumed / assembly-resumed `41` CODE. **SHA-256:**
  `cf2fd8b7314d28594165c1ca7b7f56207ac2c911abbd86632dc91b5757923680`.

- **Artifact:** Saved IR with original source spans. **SHA-256:**
  `814a64efc63f2bcd89e15c2a343a51f08492a6f2f0ce26dd6355d8166dde87fd`.

- **Artifact:** Edited `43` CODE. **SHA-256:**
  `f6f6df45c153e7695fa1ab3ef8dd7b462df16268b18983db801dbf2de94a41f2`.

- **Artifact:** Signed `43` PRG. **SHA-256:**
  `f11a38bbd04bb46e57775a873a475ad2f485052b51ac51fd8a8101d04fdddbf8`.

## Representation terminology references

External comparisons consulted on 2026-10-10:
[LLVM code generation](https://llvm.org/docs/CodeGenerator.html),
[Clang toolchains](https://clang.llvm.org/docs/Toolchain.html), [LLVM
IR][ref-llvm-ir], [LLVM MC][ref-llvm-mc],
[llvm-mc](https://llvm.org/docs/CommandGuide/llvm-mc.html),
[GIMPLE](https://gcc.gnu.org/onlinedocs/gccint/GIMPLE.html),
[RTL](https://gcc.gnu.org/onlinedocs/gccint/RTL.html),
[GNU as](https://sourceware.org/binutils/docs/as/), and [Cargo target
names][ref-cargo-target-names]. These informed terminology; they are not
compatibility requirements.

Related evidence:
[initial replacement](function-replacement.md),
[instruction semantics](instruction-semantics.md),
[MIR/module observations](module-method-names.md),
[locals](compiler-locals.md),
[module calls](compiler-calls.md), and
[parameters/arguments](compiler-arguments.md).

[ref-llvm-ir]: https://github.com/llvm/llvm-project/blob/main/llvm/lib/IR/CMakeLists.txt
[ref-llvm-mc]: https://github.com/llvm/llvm-project/blob/main/llvm/lib/MC/CMakeLists.txt
[ref-cargo-target-names]: https://doc.rust-lang.org/cargo/reference/cargo-targets.html
