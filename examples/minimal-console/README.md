# Minimal Console

The target function is deliberately small:

```monkeyc
function hello() {
    System.println("Hello!");
}
```

The app calls it from `onStart`. A watch app still needs initialization, a
manifest, resources, and a view; this specimen uses the base `WatchUi.View` with
no custom rendering. Its expected output is a single `Hello!` line in the
simulator console. Generated framework/resource code is also present in the PRG.

From the repository root:

```sh
examples/minimal-console/build.sh
```

SDK, Java, signing-key, and device-data requirements and overrides match
[Hello World](../hello-world/README.md#build). Both use
[the shared build helper](../../scripts/build-example.sh). This example's output
is `examples/minimal-console/build/fr265/minimal-console.prg`.

For the original compiled program, run on the host with Podman:

```sh
sh scripts/run-simulator-container.sh --prg examples/minimal-console/build/fr265/minimal-console.prg
```

Add `--interactive` first to open a persistent desktop window. The screenshot
mode saves console output in `scratch/simulator/`; use the console, rather than
a screen greeting, to validate this example.

Compilation succeeded with SDK 9.2.0 for `fr265`. Our Rust tools disassembled
and reassembled its entire ordinary code section and reconstructed a
byte-identical PRG. A host-side simulator launch of this specimen remains
pending. [The investigation](../../docs/investigations/rust-bootstrap.md)
records hashes, the `hello()` disassembly, and commands for the Rust round trip.
