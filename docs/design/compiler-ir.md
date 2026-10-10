# Compiler representation decision

2026-10-10. Decision for the first frontend: a small value-based semantic IR between parsing and VM assembly. This is our design, not Garmin MIR compatibility or an LLVM dependency. Evidence informing it: [constant/arithmetic replacement](../investigations/function-replacement.md), [instruction semantics](../specs/instructions.md), and [module/MIR observations](../investigations/module-method-names.md).

## Comparison

| Example | Direct AST-to-assembly lowering | Value-based IR |
|---|---|---|
| Literal return | Very small recursive emitter | Constant definition and return reference; a little more structure |
| Arithmetic/local variable | Source traversal directly manages stack order and local slots | Semantic value dependencies precede VM stack/local selection |
| Calls with side effects | Emitter must maintain language evaluation order while arranging call frames | Ordered operations make evaluation order explicit; backend handles verified call-frame conventions |
| Conditional expression | Source emitter manages labels, joins, and compatible stack state | Future blocks can express successors and result merging before stack lowering |

Only literal return is implemented initially. Arithmetic, calls, and blocks in this comparison are design considerations, not claims of implemented features or proven Monkey C semantics. A source AST alone is sufficient for the first function, but binding its traversal directly to frame/stack choices would make the next steps harder to inspect independently. Choose the small IR now without adding optimization passes or a general control-flow framework.

## Initial boundary

`ciq-compiler` owns source parsing, diagnostics, the semantic IR, and lowering to assembly. A function initially has a name and ordered operations: define an integer value; return that value. Value IDs identify semantic results, not VM symbol IDs, CODE offsets, or physical local slots. The representation is in-memory Rust data and has no stable serialized format. It is not a full SSA/control-flow design.

Parsing produces a private source representation with byte spans; lowering builds the IR without machine opcodes. The VM backend selects `argc`, integer-push, and `return` instructions and prints `.mcasm`. `ciq-mc` supplies opcode names; `ciq-asm` performs layout and encoding. No PRG scaffolding, signing, packaging, or Jungle handling belongs in the frontend.

For future arithmetic, retain explicit value dependencies and operation ordering. Introduce blocks/terminators and a validation/stack-layout policy when conditions are implemented; do not rely on an assumed physical-stack model. Calls require separately verified receiver/frame handling and source evaluation order. Preserve source spans on operations for diagnostics and later debug mapping. These are extension directions, not promised APIs.

Readable `.mcasm` is a required inspection option for generated code. It uses the same grammar as handwritten assembly and disassembly. Tests can check semantic IR separately and then assemble the emitted output. The first simulator experiment integrates an appended generated function into an explicit known-good section project; it does not introduce automatic function replacement or general relocation.
