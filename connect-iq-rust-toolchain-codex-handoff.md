# Codex Session Handoff: Open Connect IQ / Monkey C Toolchain in Rust

## Project goal

Explore and prototype an independent, open-source toolchain for Garmin
Connect IQ / Monkey C, implemented in Rust.

The immediate goal is **not** to replace Garmin's entire compiler or
simulator. Start at the bottom of the stack by understanding the `.prg`
container and Connect IQ VM bytecode well enough to build an
assembler/disassembler. The first decisive milestone is to produce or
modify a `.prg` that Garmin's official simulator accepts and executes.

Longer-term possibilities:

1.  Rust assembler/disassembler for Connect IQ VM bytecode.
2.  Independent `.prg` reader/writer.
3.  Open-source Monkey C compiler implemented in Rust.
4.  Eventually, an open-source Connect IQ VM/runtime/simulator for Linux
    and other platforms.

The source language for the compiler should remain **Monkey C**. Rust is
the implementation language for the toolchain; this is NOT currently a
Rust-to-Connect-IQ compiler project.

## User environment

-   Fedora 44.
-   Target watch: Garmin Forerunner 265.
-   Connect IQ device identifier believed to be `fr265`.
-   Official Garmin SDK/simulator can be used as a reference
    implementation/oracle.
-   Garmin's simulator is proprietary but available on Linux (official
    support is Ubuntu-oriented, so Fedora may need compatibility work).

## Legal/project framing

The project is intended as an interoperability/open-toolchain effort. EU
software interoperability provisions and the corresponding Norwegian
rules may be relevant to reverse engineering required for an
independently created compatible implementation.

Do not assume Garmin's SDK license alone settles the legal question. At
the same time, keep the implementation independent and avoid copying
Garmin implementation code. Prefer empirical format/behavior
documentation, black-box differential testing, and independently written
implementations.

This handoff is technical, not legal advice.

## Known ecosystem / prior art

Investigate these projects before reinventing format knowledge:

-   `Gualdimar/MonkeyC-Ghidra-Plugin`
    -   Ghidra loader/processor for Garmin Connect IQ `.prg` files.
    -   Parses parts of the binary format and disassembles Monkey C VM
        bytecode.
    -   Useful source for opcode encodings, PRG structures,
        symbols/classes/functions.
    -   At the time discussed, full Ghidra P-code/decompiler semantics
        were not complete.
-   `markw65/monkeyc-optimizer`
    -   Particularly important.
    -   Parses/analyzes Monkey C and can operate on Garmin-produced
        bytecode/PRGs.
    -   Its post-build functionality appears capable of reading,
        transforming, and writing executable output.
    -   Likely contains substantial knowledge of bytecode and PRG
        structures.
-   Anvil Secure has published reverse-engineering/security research on
    Garmin Connect IQ applications and VM behavior.

Garmin's compiler is proprietary. Monkey C compiles to Garmin-specific
VM bytecode contained in `.prg` applications. Garmin does not publish a
complete VM ISA specification.

## Important architectural conclusion

Model the project after **LLVM's separation of concerns**, but do NOT
initially depend on LLVM.

LLVM itself may eventually be useful, but an LLVM backend is likely
unnecessary overhead until the VM semantics and reference mechanisms are
understood. The useful idea to borrow is the architecture:

-   frontend
-   semantic IR
-   machine/target IR
-   MC-style symbolic instruction layer
-   assembler/disassembler
-   fixups/symbol resolution
-   executable/container emission

The most important initial layer is an LLVM-MC-like Connect IQ
representation.

Suggested architecture:

``` text
                         Monkey C
                            |
                     +------v------+
                     |   Frontend  |
                     | parser/types|
                     +------+------+
                            |
                            v
                     +-------------+
                     |     IR      |
                     +------+------+
                            |
                         lowering
                            |
                            v
                 +--------------------+
                 |   CIQ Machine IR   |
                 | symbolic operands  |
                 | functions/blocks   |
                 +---------+----------+
                           |
                           v
                 +--------------------+
                 |   CIQ MC layer     |
                 | instructions       |
                 | symbols            |
                 | labels             |
                 | expressions        |
                 | fixups             |
                 +---------+----------+
                           |
                    +------+------+
                    v             v
               assembler      disassembler
                    |             ^
                    v             |
                 PRG writer <-> PRG reader
```

However, **do not implement the frontend or compiler IR yet**. First
understand the target.

## Initial Rust workspace

A reasonable starting layout:

``` text
ciq-tools/
  Cargo.toml
  crates/
    ciq-prg/          # .prg container reader/writer
    ciq-mc/           # VM instructions, operands, symbols, encoding/decoding, fixups
    ciq-asm/          # textual assembler -> ciq-mc -> PRG
    ciq-dis/          # PRG -> ciq-mc -> textual disassembly
  samples/
  docs/
    prg-format.md
    bytecode.md
    symbols.md
    calling-convention.md
```

Later:

``` text
    monkeyc-lexer/
    monkeyc-parser/
    monkeyc-semantic/
    monkeyc-ir/
    monkeyc-codegen/
    ciq-vm/
```

Keep `ciq-prg` / `ciq-mc` usable independently of the future Monkey C
compiler.

## MC-style representation

Do not immediately encode source-level references into bytes. Preserve
symbolic operands.

Conceptually:

``` rust
pub enum McOperand {
    Int(i64),
    Symbol(SymbolId),
    Label(LabelId),
}

pub struct McInst {
    pub opcode: Opcode,
    pub operands: Vec<McOperand>,
}
```

Symbols might initially include:

``` rust
pub enum SymbolKind {
    Function,
    Class,
    Module,
    Global,
    External,
}
```

The exact model MUST follow observed Garmin bytecode rather than
assumptions.

## Fixups

Borrow LLVM MC's idea of fixups.

An encoder should be allowed to say, in effect:

``` text
"I can encode this instruction except for this symbolic reference."
```

Potential fixup/reference kinds we may discover include:

-   relative branch offset
-   absolute bytecode offset
-   function-table index
-   symbol ID
-   symbol hash
-   string-table index
-   class/module ID
-   Toybox/runtime API symbol
-   resource ID

Do not assume Connect IQ uses conventional addresses.

Example conceptual structure:

``` rust
struct Fixup {
    offset: usize,
    kind: FixupKind,
    target: SymbolId,
}
```

## Whole-program compilation and linking

Monkey C source files do not need to be treated like C/C++ translation
units.

The intended future compiler model is:

``` text
all applicable .mc files
        |
        v
parse all files
        |
        v
global semantic/name resolution
        |
        v
whole-program IR
        |
        v
symbolic CIQ machine instructions
        |
        v
final layout / fixups / external references
        |
        v
PRG
```

This means we may NOT need:

``` text
a.mc -> a.o
b.mc -> b.o
          |
          v
       linker
```

We still need linker-like mechanics---symbols, forward references,
layout, fixups---but they can happen inside one compiler/assembler
invocation.

Do not require topological source ordering. Forward references and
mutual recursion make that inappropriate.

Example:

``` monkeyc
function foo() {
    return bar();
}

function bar() {
    return foo();
}
```

Both functions should compile into symbolic references and be resolved
after definitions are known.

The assembler should likewise accept multiple input files in arbitrary
order:

``` text
ciq-as foo.asm bar.asm baz.asm -o app.prg
```

and resolve all symbols before final encoding.

Only introduce a standalone object format/linker if later evidence shows
it is useful.

## Monkey C source/module model

Important distinction:

-   `.mc` files are source organization.
-   Monkey C modules/namespaces are semantic organization.
-   `using` / `import` are about module/name visibility, not C-style
    textual inclusion.

A project may contain many `.mc` files that collectively form one
program.

Garmin's Jungle build configuration determines applicable source
directories/files/resources for a target.

Therefore, the future compiler can load all applicable source files and
perform whole-program analysis.

## Monkey Barrels

Monkey Barrels are Garmin's reusable library/package mechanism.

They should NOT be supported in v0.1.

An important research task is to inspect the actual contents and
behavior of `.barrel` files. Prior discussion suggested barrels may
package project/source/resource material rather than being ordinary
prelinked native-style object files, but this MUST be verified directly
against current SDK output.

Useful experiment:

1.  Create the smallest possible barrel with one exported function.
2.  Build/export it using Garmin command-line tooling (`barrelbuild` was
    identified as relevant; verify current SDK behavior/options).
3.  Run:
    -   `file foo.barrel`
    -   `unzip -l foo.barrel`
4.  Inspect its contents.
5.  Build an application that consumes it.
6.  Compare artifacts and determine whether bytecode is precompiled,
    regenerated for the target, or otherwise transformed.

This will clarify whether barrel support eventually requires a real
separately compiled-code linker.

## Official Garmin execution path

Garmin's official simulator can execute compiled `.prg` applications and
can serve as the compatibility oracle.

`monkeydo` is relevant for running an already-built `.prg` against a
simulator/device target. Verify exact current CLI syntax locally rather
than relying blindly on remembered examples.

Forerunner 265 Connect IQ device identifier: expected `fr265`; verify
from installed device definitions.

The development loop we want is approximately:

``` text
ciq-as test.asm -o test.prg
        |
        v
official Garmin simulator
        |
        v
accepted / rejected / behavior observed
```

No Garmin compiler should eventually be needed to assemble our test
program, though Garmin's compiler remains extremely useful for producing
reference specimens.

## Reverse-engineering strategy

Use Garmin's compiler and simulator as behavioral/reference oracles.

Start with tiny Monkey C programs where only one semantic feature
changes.

Examples:

``` monkeyc
function test() { return 0; }
function test() { return 1; }
function test() { return 2; }
function test() { return -1; }
```

Then:

``` monkeyc
function test() { return 1 + 2; }
```

Then locals, branches, calls, etc.

Compare resulting `.prg` files and disassembly.

Recommended progression:

1.  constants / returns
2.  local variables
3.  arithmetic
4.  conditional branches
5.  loops
6.  function calls
7.  forward function references
8.  strings
9.  arrays/dictionaries
10. modules
11. classes/objects
12. methods
13. inheritance
14. exceptions
15. Toybox calls
16. resources

Keep empirical notes in `docs/`.

## Critical first milestones

Do NOT begin by writing a Monkey C parser.

### Milestone 0 --- Survey existing RE

Inspect the two main open-source projects and document:

-   known PRG sections/headers
-   bytecode instruction representation
-   opcode table
-   operand encodings
-   symbol representation
-   string/constants tables
-   function/class/module representation
-   checksums/signatures/version fields
-   anything required for Garmin simulator acceptance
-   whether either project already contains a usable serializer

Cross-check claims between implementations and tiny Garmin-generated
specimens.

### Milestone 1 --- Read a Garmin PRG

Implement enough `ciq-prg` and `ciq-mc` to parse a trivial
Garmin-generated `.prg` and print meaningful structure/instructions.

### Milestone 2 --- Round trip

Try:

``` text
reference.prg
     |
     v
ciq-prg::read
     |
     v
internal representation
     |
     v
ciq-prg::write
     |
     v
roundtrip.prg
```

The strongest version is byte-identical output, but semantic/container
equivalence is sufficient if the simulator accepts and runs it.

### Milestone 3 --- Modify one instruction

Compile a trivial Garmin reference application, decode it, modify a
constant or similarly safe instruction, write it back, and run it.

Example conceptual change:

``` text
return 41
```

to:

``` text
return 42
```

If the official simulator executes the modified PRG with changed
behavior, we have proven independent bytecode modification/emission.

This is the first major success criterion.

### Milestone 4 --- Replace a whole function

Generate the complete instruction sequence for a trivial function
ourselves while retaining the surrounding known-good PRG structure.

### Milestone 5 --- Minimal independent PRG

Construct the necessary PRG structures without using a Garmin-produced
PRG as a template.

### Milestone 6 --- Textual assembler

Support something like:

``` asm
.function foo
    push 42
    ret
.end
```

The syntax is not important yet. Correct symbolic representation and
encoding are.

## Open-source VM / simulator

Longer-term, build an independent Connect IQ VM.

This should reuse `ciq-prg` and `ciq-mc` rather than creating another
bytecode model.

Initial VM need not simulate a watch GUI. A headless runtime capable of
executing ordinary VM instructions and a subset of Toybox APIs would
already be useful:

``` text
ciq-run tests.prg

test_distance .... ok
test_pace ........ ok
```

Likely VM work:

-   execution frames/calls
-   VM value representation
-   numbers/bools/null
-   strings
-   arrays/dictionaries
-   objects/classes/modules
-   exceptions
-   garbage collection/lifetimes as required by semantics
-   Toybox runtime shims

Later:

-   `Toybox.Graphics`
-   framebuffer/window
-   device profiles
-   simulated battery/HR/GPS/etc.
-   GPX-driven location
-   watchface/app GUI simulation

Example eventual CLI:

``` text
ciq-run --device fr265 --heart-rate 145 --battery 73 --gps trace.gpx app.prg
```

Do NOT attempt this before the bytecode semantics are sufficiently
understood.

## LLVM decision

Do not add LLVM initially.

The project should **model its architecture after LLVM**, particularly
LLVM MC, while remaining idiomatic Rust and small.

Reasons:

-   Connect IQ is a VM, not necessarily a conventional register-machine
    target.
-   We do not yet know whether function references are offsets, table
    indices, hashes, dynamic symbols, etc.
-   `.prg` is not ELF/COFF/Mach-O.
-   Implementing a proper LLVM target/backend may be more work than the
    assembler itself.
-   Whole-program Monkey C compilation may remove much of the
    traditional linker/object-file problem.

Revisit LLVM only after enough VM semantics are understood to know what
it would actually provide.

## Design principles

1.  Evidence before abstraction.
2.  Preserve symbolic information until final layout/encoding.
3.  Make encoding and decoding symmetrical.
4.  Keep PRG/container knowledge separate from Monkey C frontend
    semantics.
5.  Keep VM instruction definitions independent of the compiler.
6.  Avoid premature linker/object-format design.
7.  Do not require source dependency ordering.
8.  Document discoveries independently of Rust code.
9.  Make small differential programs the primary RE technique.
10. Test generated output against Garmin's official simulator
    frequently.

## Immediate task for Codex

Start by researching/inspecting the existing open-source
implementations, especially:

-   Gualdimar/MonkeyC-Ghidra-Plugin
-   markw65/monkeyc-optimizer

Do NOT start writing a large compiler.

Produce a technical reconnaissance of:

1.  `.prg` structure as currently understood.
2.  VM opcode/instruction definitions already known.
3.  How functions/classes/modules/symbols are represented.
4.  Whether PRG writing/serialization already exists in either project.
5.  What integrity/version/checksum fields must be updated after
    modifications.
6.  How realistic a Rust read -\> modify -\> write prototype is.
7.  The smallest Garmin-generated reference application we should create
    for the first experiment.

Then propose the minimum Rust structs/API for `ciq-prg` and `ciq-mc`,
based on observed data rather than guessed VM design.

The first implementation target should remain:

``` text
Garmin-generated trivial PRG
        |
        v
Rust parser
        |
        v
change one simple constant/instruction
        |
        v
Rust writer
        |
        v
modified PRG
        |
        v
Garmin official simulator
        |
        v
changed behavior observed
```

Once that works, proceed toward a standalone assembler.
