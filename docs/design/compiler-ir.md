# Compiler representation decision

2026-10-10. Decision for the first frontend: a small value-based semantic IR between parsing and VM assembly. This is our design, not Garmin MIR compatibility or an LLVM dependency. Evidence informing it: [constant/arithmetic replacement](../investigations/function-replacement.md), [instruction semantics](../specs/instructions.md), and [module/MIR observations](../investigations/module-method-names.md).

## Comparison

| Example | Direct AST-to-assembly lowering | Value-based IR |
|---|---|---|
| Literal return | Very small recursive emitter | Constant definition and return reference; a little more structure |
| Arithmetic/local variable | Source traversal directly manages stack order and local slots | Semantic value dependencies precede VM stack/local selection |
| Calls with side effects | Emitter must maintain language evaluation order while arranging call frames | Ordered operations make evaluation order explicit; backend handles verified call-frame conventions |
| Conditional expression | Source emitter manages labels, joins, and compatible stack state | Future blocks can express successors and result merging before stack lowering |

Literal return, grouped integer addition/subtraction, and initialized locals with repeated reads are implemented and simulator-verified. Top-level user module calls, integer parameters, and positional arguments are also implemented. Object calls and blocks remain design considerations. A source AST alone is sufficient for the first function, but binding its traversal directly to frame/stack choices would make the next steps harder to inspect independently. Choose the small IR now without adding optimization passes or a general control-flow framework.

## Initial boundary

`ciq-compiler` owns source parsing, diagnostics, the semantic IR, and lowering to assembly. A function has a name and ordered operations: define integer values, add/subtract earlier values, read positional parameters, call external user module members with integer arguments, and return the expression result. Local initialization/read operations add semantic local identities and ordered initialization sinks; each initializer/return consumes a pure expression tree with exactly one use per value definition. Value IDs identify semantic results, not VM symbol IDs, CODE offsets, or physical local slots. The representation is in-memory Rust data with a [provisional text format](../specs/compiler-ir-text.md); neither syntax nor API is stable yet. It is not a full SSA/control-flow design.

Parsing produces a private source representation with byte spans; lowering builds the IR without machine opcodes. The VM backend selects `argc`/`argcincsp`, integer-push, `addv`/`subv`, `lputv`/`lgetv`, `getmv`/`frpush`/`invokem`/`invokemz`, and `return` instructions and constructs a [shared assembly model](../specs/assembly-model.md) directly. That model can print `.mcasm` or encode CODE without a text round trip. `ciq-mc` supplies opcode encodings; `ciq-assembly` owns CODE parsing, layout, and binding resolution; `ciq-asm` reconstructs PRG projects. No PRG scaffolding, signing, packaging, or Jungle handling belongs in the frontend.

Arithmetic retains explicit value dependencies; the backend walks initializer/return expression trees rather than treating definition order as stack order. Initialization order is preserved. Local IDs stay independent of physical slots, assigned by the backend. Introduce blocks/terminators and a validation/stack-layout policy when conditions are implemented; do not rely on an assumed physical-stack model. Calls require separately verified receiver/frame handling and source evaluation order. Preserve source spans on operations for diagnostics and later debug mapping. These are extension directions, not promised APIs.

Readable `.mcasm` is a required inspection option for generated code. It uses the same grammar as handwritten assembly and disassembly. Tests can check semantic IR separately and then assemble the emitted output. The first simulator experiment integrates an appended generated function into an explicit known-good section project; it does not introduce automatic function replacement or general relocation.

## LLVM comparison

Consulted LLVM's [code-generator documentation](https://llvm.org/docs/CodeGenerator.html) and Clang's [toolchain documentation](https://clang.llvm.org/docs/Toolchain.html) on 2026-10-10. Their conventional ahead-of-time flow is frontend/AST → LLVM IR → target-specific machine representations → MC emission → object file → linker. MC emission can print textual assembly or write an object directly; an assembly text file is optional. An assembly parser provides another route into the MC layer. These are logical layers, not necessarily separate processes or on-disk files.

The useful design analogy here is semantic IR → VM instruction/reference model → either readable `.mcasm` or direct assembly/encoding through shared library APIs. The frontend now lowers into a shared typed assembly model; text printing and direct encoding are alternative consumers of that model. We do not need LLVM's full instruction-selection/register-allocation machinery for a stack VM, but must explicitly handle evaluation stack/local/frame choices. Nor does the analogy establish a need for a relocatable object format or separate linker for our whole-program PRG layout. Readable output remains available regardless of whether text becomes an optional serialization of a shared model.

## Pipeline checkpoint

The user requested proving native composition and text resumption before extending the language. Both routes now work for the constant-return subset, including edited IR and simulator execution. Source compilation, IR text parsing, and explicit IR construction use the same semantic validator; assembly text and native assembly construction use the same structural checks and encoder. See [the checkpoint](../investigations/compiler-pipeline.md). Future operation changes must update validation, native lowering, IR parsing/printing, and equivalence tests together. This requirement keeps the boundaries connected without freezing the operation set or serialization syntax.

## Layer responsibilities and naming

Discussion settled on keeping `ciq-assembly` for the shared assembly representation/infrastructure and `ciq-asm` / `ciq-dis` as the paired assembly/disassembly tools. Earlier rename suggestions are not the plan. Each shared library owns a coherent representation and its rules, rather than miscellaneous shared utilities. Cargo package and executable names can differ, but no name changes are needed ([Cargo target reference](https://doc.rust-lang.org/cargo/reference/cargo-targets.html)).

| Layer | Responsibility | Current location | Possible future boundary |
|---|---|---|---|
| Frontend | Source syntax, source-language checks, semantic IR construction | `ciq-compiler` source parser and `compile` | Frontend in `ciq-compiler` |
| Semantic IR | Values/operations, validation, IR text parsing/printing | `ciq-compiler` types, `Function::new`, `parse_ir`, `to_ir_text` | Shared `ciq-ir` library |
| Codegen / VM backend | Instruction selection, evaluation-stack/local/frame choices, later calls/control-flow lowering | `ciq-compiler::backend::lower()`; `Function::lower()` delegates | Possibly a `ciq-codegen` library |
| Assembly infrastructure | Selected VM instructions, labels, symbolic operands, `.mcasm` parsing/printing, layout/reference resolution and CODE encoding | `ciq-assembly` | Keep this responsibility/name |
| Instruction encoding | Opcode definitions, concrete operand bytes, instruction encoding/decoding | `ciq-mc` | Keep this responsibility/name |
| PRG tools | Complete project reconstruction or disassembly/export | `ciq-asm` / `ciq-dis` | Keep the paired names |

Codegen functionality already exists; a separate codegen crate does not. The IR library is a likely useful future extraction, not an implemented crate or a commitment to freeze the present operation set. If extracted, `ciq-ir` should own semantic types, validation, and text I/O and have no dependency on VM lowering or assembly. The frontend produces that representation; codegen consumes it and produces the assembly model. A compiler driver can orchestrate both. In particular, the existing VM-specific `Function::lower()` must stay with the backend rather than move into `ciq-ir` with the types.

As arithmetic/locals/calls grow, isolate VM lowering from semantic definitions, through the implemented backend module for now. Extract libraries when independent consumers or dependency boundaries warrant it; do not create packages only to match diagram boxes. No dedicated IR CLI is required: existing compiler commands can expose IR reading/writing/validation, and package names need not equal executable names.

Native model composition and textual stop/resume remain required capabilities. Keep shared validation and equivalence tests as representations change; the exact IR schema, serialization, APIs, backend algorithms, and future crate splits remain amendable. Assembly is a lower-level representation in the broad IR sense, but documentation should call it the assembly model to distinguish it from semantic compiler IR.

Naming comparisons consulted on 2026-10-10: LLVM separates [IR/Core](https://github.com/llvm/llvm-project/blob/main/llvm/lib/IR/CMakeLists.txt), [MC](https://github.com/llvm/llvm-project/blob/main/llvm/lib/MC/CMakeLists.txt), and [code-generation layers](https://llvm.org/docs/CodeGenerator.html), while [llvm-mc](https://llvm.org/docs/CommandGuide/llvm-mc.html) names a command using the MC infrastructure. GCC's internal representation names include [GIMPLE](https://gcc.gnu.org/onlinedocs/gccint/GIMPLE.html) and [RTL](https://gcc.gnu.org/onlinedocs/gccint/RTL.html); the GNU assembler command is [as](https://sourceware.org/binutils/docs/as/) in Binutils. These are examples of separating internal component names from user-facing tools, not a required crate naming scheme.

The [local-variable checkpoint](../investigations/compiler-locals.md) extends both pipeline paths without a crate split. Repeated source reads become distinct value definitions referencing one initialized semantic local; source binding is not implemented by sharing an expression value. This avoids recomputing initializer expressions. Current scheduling relies on pure expressions and no reassignment; mutable locals and effectful calls require explicit evaluation-order rules before implementation.

The [first call checkpoint](../investigations/compiler-calls.md) introduces effectful `ModuleCall` definitions. For call-containing functions, shared validation requires complete expression evaluation order in IR; lowering never silently changes that order. Pure-only functions keep their scheduling flexibility. This conservative restriction is provisional, not a general effect or exception model. Source/IR target names remain independent of numeric symbol identity. A compiler binding helper resolves them against an explicit existing symbol source; the shared assembly encoder still owns concrete reference encoding.

[Parameter/argument support](../investigations/compiler-arguments.md) adds function signature counts, semantic positional reads, and ordered call dependencies without a new crate. Backend parameter/local slot allocation and context-slot handling remain separate from the semantic representation. Both generated caller and callee now execute in an explicit scaffold, with native/text parity. Arity/type checking across functions and complete module metadata generation remain future work.
