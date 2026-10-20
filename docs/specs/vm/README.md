# VM bytecode

- [Encoding](encoding.md): opcode bytes, widths, byte order, and decoding errors.
- [Instruction reference](instructions.md): operands, stack effects, evidence,
  and unresolved behavior.

[Assembly](../assembly/README.md) resolves symbolic operands;
[compiler lowering](../lowering/README.md) selects instructions. Recognizing an
opcode does not imply that the frontend can generate every use of it.
