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
- [Initial research and Rust implementation](investigations/rust-bootstrap.md) now cover pinned Ghidra plugin and optimizer sources, observed PRG framing, and the opcode subset present in two compiled specimens.
- The Cargo workspace contains working `ciq-prg`, `ciq-mc`, `ciq-dis`, and `ciq-asm` crates. Ten Rust tests and Clippy pass; there are no external Rust dependencies.
- [Minimal Console](../examples/minimal-console/README.md) compiles for `fr265`; its target `hello()` function contains only `System.println("Hello!")`, within the required watch-app scaffolding.
- Both reference PRGs have completed byte-identical full-code disassembly/reassembly and container reconstruction. The original Hello World has verified simulator behavior; newly emitted PRGs still need a host-side simulator run. No changed-bytecode acceptance experiment has been completed.

## Milestones

1. Survey existing research: inspect the MonkeyC Ghidra plugin and monkeyc-optimizer. Document PRG layout, instruction encodings, references, available serializers, and integrity/version handling. Identify differences and unknowns.
2. Read a minimal Garmin PRG: implement the smallest useful `ciq-prg` and `ciq-mc` crates based on the observed format.
3. Round trip: reconstruct a complete runnable PRG, keeping undecoded sections raw, and test it in the official simulator. Byte-identical preservation is one test mode; successful assembly may emit different bytes and use our own signing key.
4. Modify one instruction: change a visible result such as `41` to `42`, keeping instruction length unchanged initially, and verify execution.
5. Replace a function: emit a complete instruction sequence while preserving the surrounding known-good container.
6. Construct a minimal independent PRG, then extend toward a textual assembler with symbolic references and fixups.

Do not begin a Monkey C parser, barrel linker, LLVM backend, or independent VM before the target format and bytecode are sufficiently understood.

## Next investigation

The initial survey of these primary sources is pinned in [the bootstrap investigation](investigations/rust-bootstrap.md):

- [Gualdimar/MonkeyC-Ghidra-Plugin](https://github.com/Gualdimar/MonkeyC-Ghidra-Plugin)
- [markw65/monkeyc-optimizer](https://github.com/markw65/monkeyc-optimizer)

Next, run the reassembled Minimal Console PRG in the simulator and then test a same-size `41` to `42` edit in Hello World.

Also implement complete section export/import: assembly for decoded ordinary code, raw binary for undecoded sections, and a manifest preserving identifiers and order. Plan an assembler `-y <developer-key>` option that signs reconstructed output with our own key and replaces the original developer signature. Matching the original signature or obtaining its private key is not part of this objective. The signing option is not implemented yet.

Extend the reconnaissance to cover:

- PRG headers, sections, endianness, lengths, and version fields.
- Opcode definitions, operand widths, and instruction boundaries.
- Function, class, module, symbol, constant, and string representations.
- Read/write support and treatment of unknown data.
- Integrity, checksum, or signature behavior after modifications.
- A minimum evidence-based Rust API and the smallest observable reference application.

Continue cross-checking source-derived findings against tiny Garmin-generated specimens. Equal-length replacement avoids relocating offsets but does not make an old signature valid for changed bytes. Verify changed-bytecode acceptance and behavior experimentally before implementing a different function body or length-changing assembly. The signing boundary and current SDK CLI limitation are recorded in [the architecture](architecture.md#complete-section-export-and-signing-boundary).
