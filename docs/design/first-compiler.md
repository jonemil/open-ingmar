# First Monkey C compiler milestone

Status: planned after the initial packaging stages. The user approved starting a rudimentary compiler alongside the assembler naming/control-flow work. The initial PRG reconstruction, modification, signing, and simulator milestones have already succeeded; this plan turns the future compiler into a small executable milestone.

## First observable result

Compile this source into our existing assembly representation:

```monkeyc
function probe() {
    return 41;
}
```

Use the known-good Hello World section project to provide application lifecycle, resources, class/method bindings, and container metadata. Replace the selected function through the assembler's shared CODE label, assemble/sign with our key, and validate the resulting PRG in Garmin's simulator. The first compiler emits a function body; it does not independently generate an entire application.

Define and document the supported source grammar and calling-frame rules before implementing them. Establish the zero-argument function prologue, integer representation, return sequence, and integration point from our controlled SDK specimens and instruction specs. Reject unsupported syntax with source locations. Use independently implemented Rust parsing and lowering, with no Monkey C optimizer/Ghidra implementation dependency. Compiler output should use the same assembly language as handwritten and disassembled code; machine encoding and layout remain in the existing assembler/instruction layers. Decide the compiler crate name and APIs at implementation time.

## Ordered progress

1. Finish the supported own-key IQ writer, then return to repeated method names across modules. Begin branch/exception relocation research alongside the first compiler milestone. Do not require broad package compatibility or every relocation format before a straight-line function compiler can begin.
2. Implement the constant-return function above and verify source → assembly → signed PRG → simulator. Keep the reference scaffolding explicit and reproducible.
3. Extend to integer arithmetic and local variables, following the already observed arithmetic function patterns. Add function calls once resolution/calling conventions are sufficiently specified, then a string literal and `System.println` once string/member-call generation is supported.
4. Add conditions/branches and exceptions as the relevant assembler fixups become supported. Grow language coverage through small controlled examples rather than claiming full Monkey C compatibility.
5. Generate application scaffolding, classes/symbols, resources, and required metadata independently. Connect this to [Jungle-driven project builds](project-builds.md), including application and barrel workflows.

Acceptance is the expected function behavior and a valid signed program, not equality to Garmin's optimized bytecode. Preserve unrelated scaffold sections and test assembly output through the existing encoder. Compare small SDK examples to distinguish specified behavior from hypotheses. Compiler tests must not require the SDK; real SDK/simulator experiments provide separate compatibility evidence.
