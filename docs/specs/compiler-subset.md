# First Monkey C compiler subset

Initial restricted frontend, 2026-10-10. Design: [small semantic IR](../design/compiler-ir.md). Runtime evidence: [constant returns](../investigations/first-compiler.md), [arithmetic](../investigations/compiler-arithmetic.md), and [locals](../investigations/compiler-locals.md).

Input is exactly one top-level function:

```text
function <identifier>() { <local-declaration>* return <expression>; }
local-declaration = "var" <identifier> "=" <expression> ";"
expression = primary (("+" | "-") primary)*
primary = <decimal-integer> | <local-identifier> | "(" expression ")"
```

Identifiers use ASCII letters/underscore followed by ASCII letters/digits/underscore. `function`, `return`, and `var` are reserved. Whitespace, `//` comments, and non-nested `/* ... */` comments are supported. The accepted literal domain is `0..=127`; computed results may be negative or exceed 127. Unary signs, larger literals, other literal forms, parameters, types, modules/classes, other statements/additional functions, and other operators are rejected. This is a frontend limit, not a Monkey C language or VM integer limit. The domain deliberately avoids unresolved high-bit one-byte immediate behavior.

Diagnostics identify the input byte offset and one-based line/column (columns count Unicode scalar values). The caller supplies the filename. Unsupported input never silently generates a partial function. Name resolution currently covers earlier initialized function locals only. There is no general type inference, global/member resolver, whole-program analysis, or optimizer yet.

The IR contains integer constants, binary add/subtract definitions, local initialization/read operations, and a final return using a value ID, with optional source spans. Source compilation supplies them; IR text may omit them. Without locals, lowering emits a label from the function name, `argc 0x01`, `ipushz` for zero or `ipush1` for a positive supported literal, left/right operands followed by `addv` or `subv` for binary definitions, then `return`. The frame count follows the observed parameterless/global-function context slot; omitting SDK global-resolution instructions is limited to this context-independent body. It does not establish the frame layout or valid setup for arbitrary functions.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/probe.mcasm
```

CLI output must be a new file; existing output is never overwritten. Output is ordinary assembly, not a complete PRG. Library `compile(source)` returns an inspectable immutable `Function` IR; `Function::assembly()` lowers/prints it. Public IR types are preliminary. `Function::new` validates explicitly constructed operations, `parse_ir` resumes [IR text](compiler-ir-text.md), and `Function::lower()` constructs the [shared assembly model](assembly-model.md) directly. Existing assembler/signing commands consume the output after explicitly integrating it into an application section project.

The compiler CLI now supports `--from source|ir|assembly` and `--emit ir|assembly|code`. The default two-path invocation still emits assembly from source. CODE output is a raw payload, not a PRG. See [pipeline commands and limits](compiler-ir-text.md#command-line-stages).

## Arithmetic and lowering contract

The source subset supports binary `+` and `-` with equal precedence and left associativity, and parentheses for grouping. Literals remain decimal `0..=127`; subtraction results may be negative. Unary signs, other operators, reassignment, calls, and other statement forms remain unsupported. Parse nesting is bounded at 64; the semantic function is limited to 1024 operations including return. These are implementation limits.

Semantic IR contains pure integer add/subtract definitions with explicit left/right value IDs. Each initializer/return consumes one expression tree: definitions are unique, operands refer to earlier definitions, every defined value has exactly one use, and initialization statements or the final return consume expression results. General shared-value graphs and unused definitions remain rejected until their scheduling policy is implemented. Definition order need not be VM stack order: the backend traverses the return expression and emits left operand, right operand, then `addv`/`subv`, preserving subtraction order. No constant folding is performed. VM selection lives in `ciq_compiler::backend::lower`; `Function::lower()` delegates to it. Source parsing, IR validation/text I/O, and assembly encoding share the existing stage boundaries.

## Local declarations and reads

The frontend accepts zero or more `var <identifier> = <expression>;` statements before the final return. Expressions can read earlier initialized locals repeatedly; duplicate declarations, self/forward/unknown reads, uninitialized declarations, reassignment, nested scopes, and calls are rejected. Locals are function-scoped and retain their initializer value. This restricted subset does not imply Monkey C `var` is immutable generally.

IR distinguishes semantic `LocalId` from `ValueId` and physical VM slots. `LocalInitialize` consumes an expression value and declares a unique local; `LocalRead` defines a fresh value from a previously initialized local. Every expression value still has exactly one use; repeated source reads create separate read definitions. Initializations execute in source/IR order, including unused locals. The backend assigns slots 1 onward in initialization order, preserving slot 0, uses `argcincsp 0x01 <local-count>`, and emits `lputv`/`lgetv`. Without locals it retains the existing `argc 0x01` path. Functions are limited to 255 locals and 1024 total operations. [Controlled SDK output and simulator results](../investigations/compiler-locals.md) confirm two/three-local bodies; the 255-local limit is checked structurally in tests, not established as a device runtime limit.

Provisional version-1 text gains `.local $<decimal-u32-id> %<value-id>` and `.read %<result-id> $<local-id>`, each with the existing optional span. Semantic IDs need not be contiguous; their numeric values never select physical slots.
