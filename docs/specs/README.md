# Specifications

These pages describe current contracts and intended component boundaries.
Unsupported or unverified behavior remains explicit; the [plan](../plan.md) owns
upcoming work. [Architecture](../architecture.md) maps these boundaries to Rust
crates. Source, IR, and assembly each have one authoritative grammar.

- [Monkey C frontend](frontend/README.md): grammar, types, operators, classes,
  storage, and core Toybox.Lang behavior.
- [Semantic IR](ir/README.md): serialization, validation, and operation families.
- [VM lowering](lowering/README.md): instruction selection and storage assignment.
- [Assembly](assembly/README.md): grammar, layout, aliases, and binding.
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
