# Multi-file Monkey C example

A watch app with three independently authored source files:

- `source/App.mc`: lifecycle and console output.
- `source/Numbers.mc`: `Numbers.answer()`, computing `40 + 6`.
- `source/View.mc`: display, calling the same module function.

Both the console and watch screen show `46`. No ordering between these files is
specified; the Jungle source directory supplies all of them to Garmin's
compiler.

```sh
sh scripts/build-example.sh examples/multi-file
sh scripts/run-simulator-container.sh --prg examples/multi-file/build/fr265/multi-file.prg
```

Export and reconstruct with the Rust tools, using fresh output paths:

```sh
mkdir -p scratch/multi-file
cargo run -p ciq-dis -- all examples/multi-file/build/fr265/multi-file.prg scratch/multi-file/project
cargo run -p ciq-asm -- project scratch/multi-file/project scratch/multi-file/reassembled.prg -y keys/developer_key.der
```

The reconstruction is byte identical and runs successfully. Changing only
`ipush1 0x06` to `ipush1 0x07` in the exported CODE source, then
assembling/signing to a new PRG, makes both callers observe `47`. See [the
investigation][ref-the-investigation] for artifacts and evidence. Generated
files stay under ignored `build/` and `scratch/`.

[ref-the-investigation]: ../../docs/investigations/multi-file-and-barrels.md
