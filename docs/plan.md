# Plan and current status

## Objective

Build an independent, open-source Connect IQ toolchain in Rust. Monkey C remains the intended source language for any future compiler; this is not a Rust-to-Connect-IQ compiler.

Use Garmin's compiler to generate reference specimens and Garmin's simulator as the compatibility oracle. The expected initial device is the Forerunner 265 (`fr265`), subject to verification against installed SDK device definitions.

## Current status

- Original design discussion is preserved in the [session handoff](../connect-iq-rust-toolchain-codex-handoff.md).
- Repository documentation and working conventions are established.
- SDK 9.2.0 is downloaded under `connectiq/`; [local inventory](investigations/sdk-inventory.md) records launcher details and missing native simulator dependencies.
- [Hello World](../examples/hello-world/README.md) compiles and runs successfully for `fr265` using the project-local device data, with [artifact hashes and build details](investigations/hello-world-build.md) recorded. Console output and the captured simulator screen both confirm the greeting and `41`.
- [Simulator launch](investigations/simulator-container.md) is verified in the host's Ubuntu/Xvfb container. The runner preserves console logs and captures the screen while the client is active, then ends the session.
- The same launcher now provides `--interactive` for a persistent X11/Xwayland desktop window. Local lifecycle and launch-argument checks passed; the actual host desktop connection remains to be verified.
- [SDK manager inspection](investigations/sdk-manager-device-endpoint.md) identified authenticated device download endpoints. The user has now supplied the `fr265` package and fonts under `scratch/garmin-data/`.
- A [standalone browser-login device downloader](../scripts/README.md) is available, with six passing offline tests. The actual package is present; the agent has not observed the user's live sign-in flow.
- External implementations have not yet been inspected in this repository.
- No PRG format, opcode, or integrity-field claims have been verified here.
- The original Garmin-generated PRG has a verified simulator baseline. No Rust implementation, PRG round trip, or modified-bytecode acceptance experiment has been completed.

## Milestones

1. Survey existing research: inspect the MonkeyC Ghidra plugin and monkeyc-optimizer. Document PRG layout, instruction encodings, references, available serializers, and integrity/version handling. Identify differences and unknowns.
2. Read a minimal Garmin PRG: implement the smallest useful `ciq-prg` and `ciq-mc` crates based on the observed format.
3. Round trip: preserve all bytes, including unknown sections, and test the emitted artifact in the official simulator.
4. Modify one instruction: change a visible result such as `41` to `42`, keeping instruction length unchanged initially, and verify execution.
5. Replace a function: emit a complete instruction sequence while preserving the surrounding known-good container.
6. Construct a minimal independent PRG, then extend toward a textual assembler with symbolic references and fixups.

Do not begin a Monkey C parser, barrel linker, LLVM backend, or independent VM before the target format and bytecode are sufficiently understood.

## Next investigation

Inspect these primary implementation sources, pinning the exact revision in the investigation notes:

- [Gualdimar/MonkeyC-Ghidra-Plugin](https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin)
- [markw65/monkeyc-optimizer](https://github.com/markw65/monkeyc-optimizer)

Produce a local technical reconnaissance covering:

- PRG headers, sections, endianness, lengths, and version fields.
- Opcode definitions, operand widths, and instruction boundaries.
- Function, class, module, symbol, constant, and string representations.
- Read/write support and treatment of unknown data.
- Integrity, checksum, or signature behavior after modifications.
- A minimum evidence-based Rust API and the smallest observable reference application.

Cross-check source-derived findings against tiny Garmin-generated specimens when the SDK is available. Simulator acceptance remains an experiment to perform, not an inference from another project's serializer.
