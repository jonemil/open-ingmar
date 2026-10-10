# Barrel consumer

The app and view call `Numbers.answer()` from the sibling [number-barrel](../number-barrel/README.md). Both console and screen show `46`.

Build using the archive:

```sh
sh scripts/build-barrel.sh examples/number-barrel
sh scripts/build-example.sh examples/barrel-app
```

If the barrel archive already exists, reuse it or move it aside before rebuilding it. App output is `build/fr265/barrel-app.prg`.

Build using the barrel source project instead:

```sh
CIQ_JUNGLE_FILES=monkey-source.jungle sh scripts/build-example.sh examples/barrel-app
```

Both builds write the same output filename; save a copy before comparing them. The app manifest declares `Numbers` as an `iq:depends` inside `iq:barrels`. The two Jungle files select either the archive or a grouped barrel-project Jungle path. `Numbers.mc` is not included in the app's source directory.

```sh
sh scripts/run-simulator-container.sh --prg examples/barrel-app/build/fr265/barrel-app.prg
```

Both forms produce identical CODE and DATA. Archive-input reconstruction with our Rust assembler runs successfully. A separately exported `.iq` package also contains a PRG our tools round-trip and run. See [the investigation](../../docs/investigations/multi-file-and-barrels.md) for packaging commands, hashes, and limits.
