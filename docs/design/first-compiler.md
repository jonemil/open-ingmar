# First Monkey C compiler milestone

Status: first constant-return frontend implemented and simulator-validated; the initial packaging and module-naming stages are complete. The user approved starting a rudimentary compiler alongside the assembler naming/control-flow work. The initial PRG reconstruction, modification, signing, and simulator milestones have already succeeded; this plan turns the future compiler into a small executable milestone.

## First observable result

Compile this source into our existing assembly representation:

```monkeyc
function probe() {
    return 41;
}
```

Use the known-good Hello World section project to provide application lifecycle, resources, class/method bindings, and container metadata. Replace the selected function through the assembler's shared CODE label, assemble/sign with our key, and validate the resulting PRG in Garmin's simulator. The first compiler emits a function body; it does not independently generate an entire application.

Define and document the supported source grammar and calling-frame rules before implementing them. Establish the zero-argument function prologue, integer representation, return sequence, and integration point from our controlled SDK specimens and instruction specs. Reject unsupported syntax with source locations. Use independently implemented Rust parsing and lowering, with no Monkey C optimizer/Ghidra implementation dependency. Compiler output should use the same assembly language as handwritten and disassembled code; machine encoding and layout remain in the existing assembler/instruction layers. The initial crate is `ciq-compiler`; APIs remain preliminary.

## Initial architecture decision

The [initial representation decision](compiler-ir.md) selects a small value-based IR between parsing and VM lowering. The comparison and its limits are documented there. `ciq-compiler` now implements the [restricted constant-return subset](../specs/compiler-subset.md), with [simulator evidence](../investigations/first-compiler.md) for `41` and `42`. Readable `.mcasm` remains the low-level inspection boundary.

Use the constant-return milestone and small anticipated arithmetic/local/call/conditional examples to assess the boundary. In particular, consider evaluation order and side effects, temporary values and local slots, receiver/calling-frame rules, stack effects, and consistent stack state at control-flow joins. Separate verified VM behavior from assumptions. Assess how each design supports source diagnostics, inspection, and later language extensions without building a large optimization framework for the first function.

The chosen IR describes the semantic operations needed by our supported subset and lowers explicitly to VM instructions. Its initial form need not use SSA or have a serialized file format. Direct lowering was considered; the small IR was chosen to keep semantic values separate from VM stack/frame choices. Garmin MIR is useful comparative evidence, but compatibility with its syntax is not a requirement; see [local MIR observations](../investigations/module-method-names.md#mir-follow-up).

Readable `.mcasm` remains the low-level inspection boundary whichever internal design is selected. Compiler output should make generated instructions and references easy to inspect, assemble, and compare with disassembly when debugging bytecode generation. An internal IR complements that output rather than replacing it. Encoding, layout, and signatures remain responsibilities of the existing layers.

## Ordered progress

1. Completed: own-key IQ writing, repeated-module naming, and the initial compiler representation/lowering decision. Begin branch/exception relocation research alongside the first compiler milestone. Do not require broad package compatibility or every relocation format before a straight-line function compiler can begin.
2. Completed: constant-return compilation and verified source → assembly → signed PRG → simulator for `41` and `42`. Keep the reference scaffolding explicit and reproducible.
3. Integer addition/subtraction and grouping are complete, with [arithmetic runtime evidence](../investigations/compiler-arithmetic.md). Initialized local declarations/reads are also complete, with [local runtime evidence](../investigations/compiler-locals.md). Add function calls once resolution/calling conventions are sufficiently specified, then a string literal and `System.println` once string/member-call generation is supported.
4. Add conditions/branches and exceptions as the relevant assembler fixups become supported. Grow language coverage through small controlled examples rather than claiming full Monkey C compatibility.
5. Generate application scaffolding, classes/symbols, resources, and required metadata independently. Connect this to [Jungle-driven project builds](project-builds.md), including application and barrel workflows.

Acceptance is the expected function behavior and a valid signed program, not equality to Garmin's optimized bytecode. Preserve unrelated scaffold sections and test assembly output through the existing encoder. Compare small SDK examples to distinguish specified behavior from hypotheses. Compiler tests must not require the SDK; real SDK/simulator experiments provide separate compatibility evidence.

Before arithmetic work, the [native/text pipeline checkpoint](../investigations/compiler-pipeline.md) now proves both composition routes. IR text and assembly text are optional intermediate representations, each parsed into the same models used by the native route. Extend this pipeline and its round-trip/equivalence tests whenever the IR changes; the exact IR syntax remains provisional.

The [layer/naming clarification](compiler-ir.md#layer-responsibilities-and-naming) keeps `ciq-assembly` and the `ciq-asm` / `ciq-dis` pair. While extending arithmetic, maintain a clear semantic-IR/backend boundary; VM instruction selection is codegen, implemented in `backend::lower()` with a `Function::lower()` convenience wrapper. A future `ciq-ir` extraction must leave that VM-specific lowering in the backend. Crate splits are optional and should follow implementation needs; native/text pipeline compatibility remains part of acceptance.
