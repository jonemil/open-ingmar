# First Monkey C compiler subset

Initial restricted frontend, 2026-10-10. Design: [small semantic IR](../design/compiler-ir.md). Runtime evidence: [constant returns](../investigations/first-compiler.md) and [arithmetic](../investigations/compiler-arithmetic.md).

Input is exactly one top-level function:

```text
function <identifier>() { return <expression>; }
expression = primary (("+" | "-") primary)*
primary = <decimal-integer> | "(" expression ")"
```

Identifiers use ASCII letters/underscore followed by ASCII letters/digits/underscore. `function` and `return` are reserved. Whitespace, `//` comments, and non-nested `/* ... */` comments are supported. The accepted literal domain is `0..=127`; computed results may be negative or exceed 127. Unary signs, larger literals, other literal forms, parameters, types, modules/classes, additional statements/functions, and other operators are rejected. This is a frontend limit, not a Monkey C language or VM integer limit. The domain deliberately avoids unresolved high-bit one-byte immediate behavior.

Diagnostics identify the input byte offset and one-based line/column (columns count Unicode scalar values). The caller supplies the filename. Unsupported input never silently generates a partial function. There is no type inference, symbol resolver, whole-program analysis, or optimizer yet.

The IR contains integer constants and binary add/subtract definitions, followed by a return using a value ID, with optional source spans. Source compilation supplies them; IR text may omit them. Lowering emits a label from the function name, `argc 0x01`, `ipushz` for zero or `ipush1` for a positive supported literal, left/right operands followed by `addv` or `subv` for binary definitions, then `return`. The frame count follows the observed parameterless/global-function context slot; omitting SDK global-resolution instructions is limited to this context-independent body. It does not establish the frame layout or valid setup for arbitrary functions.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/probe.mcasm
```

CLI output must be a new file; existing output is never overwritten. Output is ordinary assembly, not a complete PRG. Library `compile(source)` returns an inspectable immutable `Function` IR; `Function::assembly()` lowers/prints it. Public IR types are preliminary. `Function::new` validates explicitly constructed operations, `parse_ir` resumes [IR text](compiler-ir-text.md), and `Function::lower()` constructs the [shared assembly model](assembly-model.md) directly. Existing assembler/signing commands consume the output after explicitly integrating it into an application section project.

The compiler CLI now supports `--from source|ir|assembly` and `--emit ir|assembly|code`. The default two-path invocation still emits assembly from source. CODE output is a raw payload, not a PRG. See [pipeline commands and limits](compiler-ir-text.md#command-line-stages).

## Arithmetic and lowering contract

The source subset supports binary `+` and `-` with equal precedence and left associativity, and parentheses for grouping. Literals remain decimal `0..=127`; subtraction results may be negative. Unary signs, other operators, locals, calls, and multiple statements remain unsupported. Parse nesting is bounded at 64; the semantic function is limited to 1024 operations including return. These are implementation limits.

Semantic IR contains pure integer add/subtract definitions with explicit left/right value IDs. For this step it represents one expression tree: definitions are unique, operands refer to earlier definitions, every defined value has exactly one use, and one final return consumes the expression result. General shared-value graphs and unused definitions remain rejected until their scheduling policy is implemented. Definition order need not be VM stack order: the backend traverses the return expression and emits left operand, right operand, then `addv`/`subv`, preserving subtraction order. No constant folding is performed. VM selection lives in `ciq_compiler::backend::lower`; `Function::lower()` delegates to it. Source parsing, IR validation/text I/O, and assembly encoding share the existing stage boundaries.
