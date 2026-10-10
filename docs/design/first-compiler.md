# First Monkey C compiler milestone

Compile a function independently, integrate it into known-good application
scaffolding, and verify its behavior in Garmin's simulator. This milestone is
complete; it established the pipeline used for later language extensions.

## Observable result and boundary

The first source was:

```monkeyc
function probe() {
    return 41;
}
```

Hello World supplied lifecycle methods, resources, method bindings, and other
container metadata. Shared CODE labels selected the generated function; the
assembler reconstructed and signed the PRG with our key. The simulator verified
`41` and a source edit to `42`. See
[evidence](../investigations/first-compiler.md).

The compiler emits a function, not a complete application. Source diagnostics
reject unsupported input. Independent Rust parsing and semantic IR construction
have no optimizer/Ghidra implementation dependency. VM lowering produces the
same assembly model used by handwritten code and disassembly; encoding, layout,
and signing remain separate.

[Compiler design](compiler-ir.md) explains the semantic/backend boundary.
[The source subset](../specs/compiler-subset.md) and
[IR text](../specs/compiler-ir-text.md) specify accepted inputs. The
[native/text checkpoint](../investigations/compiler-pipeline.md) verifies both
in-memory composition and editable stage resumption.

## Extension order

1. Completed: constant returns, integer addition/subtraction/grouping,
   initialized locals, user-module calls, integer parameters, and positional
   arguments. [The plan](../plan.md) links their runtime evidence.
2. Next: strings and member calls such as `System.println`, with controlled
   specimens before implementation.
3. Conditions/branches and exceptions after their assembler fixups and
   stack/frame requirements are understood.
4. Independent application DATA/classes/symbols/resources and other required
   metadata, then [Jungle-driven project builds](project-builds.md).

Define source grammar and calling conventions before adding syntax. Preserve
source spans for diagnostics. Assess evaluation/effect order, temporary/local
storage, receiver/context handling, and stack compatibility at control-flow
joins. Distinguish verified behavior from assumptions; broad package
compatibility and every relocation format are not prerequisites for a bounded
straight-line compiler.

## Acceptance

Require intended function behavior and valid own-key signing, rather than byte
equality with Garmin's compiler. Check unrelated scaffold preservation and
native/IR/assembly equivalence. Compiler tests must run without the SDK;
official SDK/simulator experiments provide separate compatibility evidence.

Every IR extension updates semantic validation, lowering, text I/O, and parity
checks together. Keep readable `.mcasm` for low-level debugging. Formats and
APIs remain provisional; no optimizer framework, new crate split, or Garmin MIR
compatibility is required by this milestone.
