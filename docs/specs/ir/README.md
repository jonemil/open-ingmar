# Semantic IR

Semantic IR retains computations, storage identities, types, and effect order
before VM instruction selection. Native construction and text resumption pass
through the same validation.

- [Text grammar](grammar.md): function, operation, unit, and field syntax.
- [Model and validation](model.md): shared invariants and API boundaries.
- [Values and storage](values.md): constants, locals, types, tests, and casts.
- [Containers](containers.md): allocation, constants, and private builders.
- [Control flow](control-flow.md): blocks, edges, reachability, and loop proofs.
- [Calls and objects](calls.md): user/native dispatch and receiver contracts.
- [Units and classes](unit.md): ownership and cross-body validation.

[Source semantics](../frontend/README.md) define the shared type and behavior
contracts; [lowering](../lowering/README.md) owns their VM implementation.
