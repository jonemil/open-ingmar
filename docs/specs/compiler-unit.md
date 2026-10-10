# Compilation units

This profile collects multiple restricted functions before lowering CODE. It is
independent of application profiles; the
[console application builder](console-application.md) supplies a complete PRG
from this unit, while Jungle resolution remains future work. The [function subset](compiler-subset.md)
and [function IR](compiler-ir-text.md) remain the per-function contracts.

## Source declarations

```text
unit = declaration+
declaration = function | "module" identifier "{" function+ "}"
```

Inputs are one or more source files in caller-supplied order. Global functions
and flat module members are collected in file/declaration order. Modules may be
reopened within or across files; each `(module, function-name)` identity must
be unique. Global functions use a separate owner. Reject duplicate globals and
a global function whose name matches a declared module. `System` is reserved
as an intrinsic receiver and cannot be declared as a module.

All function bodies use the existing grammar and limits. Qualified user calls
must resolve to a module member in this unit, regardless of declaration order.
Call argument counts must exactly match the callee's parameter count. All
parameters, user-call arguments/results, and returns are integers; string and
Boolean arguments are rejected. These are the current subset's fixed type
rules, not general Monkey C type inference.

Forward and recursive calls are accepted; termination is not proven. Calls to
undeclared external modules/members are rejected. `System.println` remains the
separate intrinsic. Global functions can serve as entry methods but cannot yet
be called by bare source names. Imports, classes, nested/empty modules, typed
signatures, optional/default parameters, overloading, and arbitrary SDK calls
remain unsupported.

The unit contains 1..=256 functions total across all files. Function operation,
nesting, parameter/local, literal, and control-flow limits remain unchanged.
No total CODE or device-memory budget is inferred from this declaration limit.
Source failures identify the input-file position and a location within it.

## Identity and CODE layout

Semantic function names remain unqualified; declaration ownership is separate.
Lowering emits all bodies into one assembly model in declaration order, using:

```text
global: __ciq_fn_g_<name-byte-length>_<name>
member: __ciq_fn_m_<module-byte-length>_<module>_<name-byte-length>_<name>
```

Lengths are decimal; source names are ASCII. For example, global `probe` maps
to `__ciq_fn_g_5_probe`, and `Numbers.difference` maps to
`__ciq_fn_m_7_Numbers_10_difference`. Internal block and string aliases use that
label as their prefix, preventing collisions across owners. Method metadata
must bind to the declaration's CODE label, not just its source name.

Calls still select symbolic module/member identities through `getmv` and the
existing context/argument convention. CODE labels identify bodies; they do not
replace module DATA method records. Symbol IDs and DATA literal placements
remain explicit encoder inputs. Strings are deduplicated within each function;
unit lowering does not merge literals across functions.

## Unit IR text

Canonical text wraps existing complete function IR documents:

```text
ciq-unit 1
.owner global
ciq-ir 1
.function probe
.call %0 Numbers answer
.return %0
.end
.owner module Numbers
ciq-ir 1
.function answer
.integer %0 41
.return %0
.end
```

The first line is the unit header. Each `.owner global` or
`.owner module <identifier>` applies to exactly one following function document,
including its own `ciq-ir 1` or `ciq-ir 2` header and `.end`. There is no unit end
directive. Blank lines and `#` comments between records are accepted, as are
LF/CRLF line endings; function text retains its existing quoting/comment rules.
Each inner function is validated before checking ownership and calls together.
Malformed framing, unsupported versions, duplicates, unresolved calls, and
signature mismatches are errors.

Source spans remain optional per-function provenance. Filenames and source-file
indexes are not serialized. Saved unit IR produces the same assembly and CODE
as native source compilation with identical bindings. Single-function IR text
and its parser remain independently supported.

## CLI

`--unit` selects compilation-unit source or IR mode. With source input it accepts
multiple input paths, with the final path always the new output:

```sh
cargo run -p ciq-compiler -- --unit examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/unit.mcasm
cargo run -p ciq-compiler -- --unit --emit ir examples/compiler-unit/entry.mc examples/compiler-unit/numbers.mc scratch/unit.ciqir
cargo run -p ciq-compiler -- --unit --from ir scratch/unit.ciqir scratch/resumed.mcasm
```

`--from ir` and `--from assembly` require one input. Assembly is the existing
shared format, so `--unit` is unnecessary when resuming it. `--emit code` accepts
the existing explicit `--symbols`/`--data` bindings and emits raw CODE, not a PRG.
All outputs must be new files. Without `--unit`, the original single-function
CLI behavior is unchanged.

See [the example](../../examples/compiler-unit/README.md) and
[simulator evidence](../investigations/compiler-unit.md).

For complete unsigned application output, use `--emit prg --app-id <id>`
with source or saved unit IR. The [console application profile](console-application.md)
specifies the generated lifecycle, metadata, entry selection, and restrictions.
