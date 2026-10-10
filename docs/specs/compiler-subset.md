# First Monkey C compiler subset

Initial restricted frontend, 2026-10-10. Design: [small semantic IR](../design/compiler-ir.md). Runtime evidence is recorded in [the compiler investigation](../investigations/first-compiler.md).

Input is exactly one top-level function:

```text
function <identifier>() { return <decimal-integer>; }
```

Identifiers use ASCII letters/underscore followed by ASCII letters/digits/underscore. `function` and `return` are reserved. Whitespace, `//` comments, and non-nested `/* ... */` comments are supported. The accepted integer domain is `0..=127`; signs, larger values, other literal forms, parameters, types, modules/classes, additional statements/functions, and expressions are rejected. This is a frontend limit, not a Monkey C language or VM integer limit. The domain deliberately avoids unresolved high-bit one-byte immediate behavior.

Diagnostics identify the input byte offset and one-based line/column (columns count Unicode scalar values). The caller supplies the filename. Unsupported input never silently generates a partial function. There is no type inference, symbol resolver, whole-program analysis, or optimizer yet.

The IR contains an integer constant definition and a return using its value ID, with source spans. Lowering emits a label from the function name, `argc 0x01`, `ipushz` for zero or `ipush1` for a positive supported literal, then `return`. The frame count follows the observed parameterless/global-function context slot; omitting SDK global-resolution instructions is limited to this context-independent body. It does not establish the frame layout or valid setup for arbitrary functions.

```sh
cargo run -p ciq-compiler -- examples/compiler-probe/probe.mc scratch/probe.mcasm
```

CLI output must be a new file; existing output is never overwritten. Output is ordinary assembly, not a complete PRG. Library `compile(source)` returns an inspectable immutable `Function` IR; `Function::assembly()` lowers/prints it. Public IR types are preliminary, and external IR construction is not supported yet. Existing assembler/signing commands consume the output after explicitly integrating it into an application section project.
