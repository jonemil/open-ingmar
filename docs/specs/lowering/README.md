# VM lowering

`ciq-compiler::backend::lower` consumes validated semantic IR and constructs a
`ciq-assembly::Program` directly. `Function::lower` delegates to it. Printing
and reparsing IR or assembly is optional and must preserve the same result.

- [Values and storage](values.md): immediates, frame slots, containers, ownership.
- [Calls and objects](calls.md): receiver/member selection and invocation.
- [Control flow](control-flow.md): branches, joins, and loops.
- [Unit layout](unit-layout.md): owner-qualified CODE labels and body order.

The frontend establishes source meaning; IR validation checks types, ownership,
block-local single uses, and an effect schedule that the backend can honor.
Lowering assigns VM storage and selects instructions without constant folding,
extra runtime guards, or substitutions for unsupported source features. Assembly
then resolves identities and placement; it does not select operations.

Slot zero is the context/receiver; parameters begin at one. Ordinary locals
follow parameters in initialization order, including locals on mutually
exclusive paths. Private builders and hidden temporaries count against the same
255-slot budget. Slots are not reused. The prologue uses `argc` without extra
locals or `argcincsp` to allocate additional frame storage. These are structural
encoding bounds, not demonstrated maximum device capacities.

Hidden reference temporaries must release their ownership on their final use. A
`LocalTake` loads the value, pushes Null, and clears its frame slot while
retaining the value on the evaluation stack. Source locals keep ordinary
assignment/frame lifetimes; there is no general last-use destruction analysis.
Ordinary CFG edges have empty stacks; value edges carry their one join argument.

[VM encodings](../vm/encoding.md) and [instruction semantics](../vm/instructions.md)
are distinct from this compiler's instruction choices. Native API declarations
supply identities and availability, not VM slots, imports, or implementations.
[Application generation](../build/application.md) materializes DATA, symbols,
imports, and container sections from the lowered unit.
