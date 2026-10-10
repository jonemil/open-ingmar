# Compiler representation design

Use a small value-based semantic IR between source parsing and VM assembly. This
separates language meaning from frame/stack choices and keeps each stage
inspectable. The format and APIs remain provisional; this is not Garmin MIR
compatibility or a commitment to LLVM/SSA infrastructure.

## Representation and invariants

A function has a name, parameter count, and ordered operations. Operations
represent integer/string constants, addition/subtraction, positional parameter
reads, local initialization/reads, user-module calls, print statements, and a
final integer return. Source spans are optional diagnostic provenance.

Value IDs name semantic results. Local IDs name stored semantic locals. Neither
chooses a VM slot, symbol ID, or CODE/DATA offset. Parameter indexes name
positions; the backend accounts for the VM context separately.

Each expression value has one use and references earlier definitions. Local
initializations execute in order, including unused locals; reads require an
initialized local. Repeated source reads create fresh values from stored locals
rather than recomputing initializers.

Pure-only expression definitions may be scheduled by dependencies. Functions
containing calls or prints must have IR operation order matching left-to-right
expression evaluation and statement order. Validation rejects schedules the
current backend cannot preserve. Shared-value graphs, reassignment, object
calls, and control flow require further scheduling/validation work.

The IR is immutable after validation. Source parsing, typed construction, and
[IR text](../specs/compiler-ir-text.md) use the same validator, so no producer
can bypass backend assumptions.

## Layer responsibilities and naming

- **Frontend (`ciq-compiler`)**: source syntax, name resolution, diagnostics,
  and semantic IR construction. Parsing has no machine opcodes.
- **Semantic IR (`ciq-compiler`)**: operations, invariants, and text I/O. A
  future `ciq-ir` may expose this independently of VM lowering/assembly.
- **VM backend (`ciq-compiler::backend`)**: instruction selection and evaluation
  stack/frame/local choices. `Function::lower()` delegates to
  `backend::lower()`. A separate `ciq-codegen` remains optional.
- **Assembly (`ciq-assembly`)**: selected instructions, symbolic operands,
  labels, syntax, layout, binding, and CODE encoding. See
  [the assembly model](assembly-model.md).
- **Instructions (`ciq-mc`)**: concrete opcodes and operand byte encodings.
- **PRG tools (`ciq-asm` / `ciq-dis`)**: reconstruction and inspection/export.
  Keep these paired names and the shared `ciq-assembly` library name.

Each shared library owns a representation and its rules, not miscellaneous
utilities. Extract crates when dependency boundaries or independent consumers
justify it. If IR is extracted, move VM-specific methods out of its types; the
backend consumes IR and produces assembly, never the reverse dependency.

Source project configuration, DATA/resource/module generation, signing, and
packaging remain separate concerns. Current function integration uses existing
application metadata. Symbol binding resolves module/member names against an
explicit symbol source without generating method records.

## Native and textual pipelines

Source → semantic IR → assembly model → CODE is the native path. IR text and
`.mcasm` are optional serialization boundaries, not mandatory temporary files.
Handwritten, generated, and disassembled assembly use the same language.

Text resumption must reestablish the native model's invariants and use the same
lowering/encoder. Every operation extension updates validation, native lowering,
IR parsing/printing, and parity tests together. Preserve this contract without
freezing the schema or backend algorithm.

The current tree-based backend needs no optimizer or general control-flow
framework. Add blocks, terminators, result joins, and stack validation when
conditions require them. Keep readable assembly available to diagnose VM-level
issues throughout that evolution.

## Rationale and evidence

Direct AST emission is smaller for a literal return, but ties source traversal
to VM stack/frame choices. A semantic boundary makes dependencies, local
identity, and effect order inspectable before selecting instructions. This
supports both native composition and editable stage outputs.

LLVM's frontend → semantic IR → machine representation → MC emission flow
motivated the separation. Its textual assembly is optional. This analogy does
not establish a need for register allocation, a relocatable object format, or an
independent linker for this VM toolchain.

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

Local evidence:
[initial replacement](../investigations/function-replacement.md),
[instruction semantics](../investigations/instruction-semantics.md),
[MIR/module observations](../investigations/module-method-names.md),
[pipeline parity](../investigations/compiler-pipeline.md),
[locals](../investigations/compiler-locals.md),
[module calls](../investigations/compiler-calls.md), and
[parameters/arguments](../investigations/compiler-arguments.md).

[ref-llvm-ir]: https://github.com/llvm/llvm-project/blob/main/llvm/lib/IR/CMakeLists.txt
[ref-llvm-mc]: https://github.com/llvm/llvm-project/blob/main/llvm/lib/MC/CMakeLists.txt
[ref-cargo-target-names]: https://doc.rust-lang.org/cargo/reference/cargo-targets.html

## Literal content and placement

Semantic string values retain UTF-8 content rather than DATA addresses. Local
reads retain their initializer's integer/string type. The current validator
restricts arithmetic, parameters, user-call arguments/results, and returns to
integers; print statements accept either type and define no result.

Lowering deduplicates content within a function and emits unplaced assembly
string declarations plus symbolic `news` operands. An explicit DATA source
supplies placement at encoding, keeping semantic IR independent of application
layout. DATA layout can be validated before its method references are resolved;
PRG reconstruction completes those bindings. This supports both native and saved
textual stages without allocating application metadata in the frontend.

`System.println` is an explicit intrinsic. Its dedicated SDK alias and discarded
VM result follow observed SDK output; arbitrary SDK names and return types are
not inferred. [The experiment](../investigations/compiler-strings.md) documents
this boundary and generated UTF-8 output in the simulator.
