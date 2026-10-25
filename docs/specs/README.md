# Specifications

These pages describe current contracts and intended component boundaries.
Unsupported or unverified behavior remains explicit; the [plan](../plan.md) owns
upcoming work. [Architecture](../architecture.md) defines component boundaries;
its [implementation map](../architecture.md#implementation-map) points to source
owners. Source, IR, and assembly each have one authoritative grammar.

- [Monkey C frontend](frontend/README.md) and [grammar](frontend/grammar.md):
  types, operators, classes, storage, and core Toybox.Lang behavior.
- [Semantic IR](ir/README.md) and [grammar](ir/grammar.md): serialization,
  validation, and operation families.
- [VM lowering](lowering/README.md): instruction selection and storage assignment.
- [Assembly](assembly/README.md) and [grammar](assembly/grammar.md): layout,
  aliases, and binding.
- [VM bytecode](vm/README.md): encoding and instruction behavior.
- [SDK declarations and profiles](sdk/README.md): catalogue, availability,
  higher-level module calls, and application callbacks.
- [Build inputs and application generation](build/README.md): compiler commands,
  current application output, planned project resolution, and local device data.
- [Container and section formats](formats/README.md): PRG, DATA, metadata,
  signing, barrels, and IQ packages.

Grammar pages specify syntax. Semantic and format pages explain meaning,
validation, preservation, and limits. Examples there illustrate the grammar;
they do not define alternate accepted syntaxes. Investigation links retain
supporting evidence and uncertainty without making research notes necessary
implementation references.

Each category opens with its general reference; Monkey C, IR, and assembly keep
their grammars beside it. Headings divide related topics within those
references. Builds keep the independently used device-download protocol
separate. Formats separate DATA's shared record walk, metadata binding,
archive/package operations, and cryptographic signing from the general
PRG/project contract. These boundaries follow distinct consumers and formats
rather than individual features or helpers.
