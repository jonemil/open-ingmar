# Compiler goals and acceptance

Compile the supported Monkey C subset independently through semantic IR and
symbolic VM assembly. Support function CODE for explicit scaffold integration
and complete PRGs through the [application builder](console-application.md).
The [source specification](../specs/compiler-subset.md) defines accepted syntax;
[the plan](../plan.md) defines current priorities.

## Boundaries

Parsing and name resolution construct validated semantic IR. The VM backend
selects instructions and frame storage; assembly handles layout and binding.
Application generation supplies metadata and lifecycle code. Signing, packaging,
and project configuration remain separate. See [representation
design](compiler-ir.md).

Keep source spans for diagnostics and readable assembly for low-level debugging.
Unsupported input must produce an explicit diagnostic. Implement independently
of external research repositories; SDK tools provide reference output and
runtime checks, not compiler implementation code.

## Acceptance

Require intended behavior and valid own-key signing, not byte equality with
Garmin's compiler. For scaffold integration, verify preservation of unrelated
sections. Source, typed IR, saved IR, and saved assembly must agree under the
same bindings. Every operation extension updates validation, lowering, text I/O,
and parity checks together.

Compiler tests run without the SDK. Controlled SDK builds and simulator runs
provide separate compatibility evidence; encoding success does not establish
execution. Record specimens, versions, observations, and uncertainty in
[investigations](../investigations/README.md).

The representations remain provisional. Extract crates or add optimizer/linker
infrastructure when actual consumers or representation needs justify them.
