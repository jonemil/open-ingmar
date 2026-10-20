# ciq-dis

CLI for inspecting PRG sections, instructions, symbols, DATA, and resources,
and exporting an editable section project. Unsupported payloads remain raw.

From the workspace root (input paths are placeholders):

```sh
cargo run -p ciq-dis -- sections input.prg
cargo run -p ciq-dis -- code input.prg
cargo run -p ciq-dis -- all input.prg scratch/project
```

The export directory must be new. Reconstruct it with
[ciq-asm](../ciq-asm/README.md). See [section-project rules](../../docs/specs/formats/section-project.md)
and the [reference example](../../examples/minimal-console/README.md).
The [CLI source](src/main.rs) lists additional inspection modes.
