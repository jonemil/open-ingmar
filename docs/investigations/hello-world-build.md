# Hello World reference build

Date: 2026-10-07. Question: can the local Garmin SDK compile a minimal
independently written reference application for the Forerunner 265?

## Successful follow-up build

The user supplied a downloaded device package under
`scratch/garmin-data/Devices/fr265/` and fonts under
`scratch/garmin-data/Fonts/`. A subsequent build succeeded with exit code `0`
and output `BUILD SUCCESSFUL`, with no warnings in the captured log. The
resulting PRG subsequently ran successfully in the host's Ubuntu/Xvfb simulator
container: console output and the inspected screenshot both confirm the greeting
and `41`. [The simulator record][ref-the-simulator-record] contains execution
evidence and capture hashes.

The build script now defaults `CIQ_DATA_HOME` to `scratch/garmin-data/` in the
repository. It creates
`examples/hello-world/build/sdk-user-home/.Garmin/ConnectIQ` as a symlink to
that data root, and invokes the official compiler's Java entry point with
`-Duser.home` pointing at the generated user-home directory. This empirically
verifies SDK 9.2.0 device discovery through Java's user-home property on Linux;
no compiler implementation inspection was needed. SDK launcher JVM options are
preserved.

Reproduction:

```sh
examples/hello-world/build.sh
```

For another device-data root:

```sh
CIQ_DATA_HOME=/absolute/path/to/garmin-data examples/hello-world/build.sh
```

The root contains `Devices/` and `Fonts/`. SDK and key paths remain
independently overridable through `CIQ_SDK_HOME` and `CIQ_DEVELOPER_KEY`.
Relative overrides are resolved before the script changes directories.

Verified inputs: SDK 9.2.0, Temurin 17.0.20.1+1, target `fr265`, hardware part
number `006-B4257-00`, optimization `-O 0`, user-provided signing key, and debug
information retained. The installed device compiler profile reports Connect IQ
`5.2.0` and firmware version `2502` for that part number; these are profile
metadata, not a reading of the physical watch's firmware.

Artifact SHA-256 values:

- **Artifact:** `examples/hello-world/build/fr265/hello-world.prg`. **SHA-256:**
  `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7`.

- **Artifact:** `scratch/garmin-data/Devices/fr265/compiler.json`. **SHA-256:**
  `e6b5de3f1a9090977f3bb41b7ae932c5c233215cd0fd560fc7856df403d70645`.

- **Artifact:** `scratch/garmin-data/Devices/fr265/simulator.json`. **SHA-256:**
  `8bf2ba93b5728054636533da002e705231cadc7047a1bf23934083b0abd1d29a`.

The build also emitted `hello-world.prg.debug.xml` and generated MIR/resource
files. These remain ignored alongside the PRG. The build log is retained locally
at `scratch/hello-world-build-success.log`. The earlier failed attempt below is
preserved as history.

## Inputs and toolchain

- SDK: local Garmin Connect IQ 9.2.0.
- Target: `fr265`.
- Project: [examples/hello-world](../../examples/hello-world/README.md).
- Signing key: user-provided root `developer_key.der`, ignored by Git along with
  `developer_key.pem`. Key contents were not printed or copied.
- Build flags:
  `-f monkey.jungle -d fr265 -o build/fr265/hello-world.prg -y <key-path> -O 0`,
  with no release flag.
- Independently generated 32x32 RGB PNG launcher icon, showing a white H on
  black.

The project has an `Application.AppBase` entry class and a `WatchUi.View`. A
separate `probe()` function returns `41`, displayed on screen and printed on
startup. This provides an observable reference for a later `41` to `42`
modification.

## Java setup

No Java runtime was initially available. A local Eclipse Temurin Java 17 JRE was
downloaded into ignored `scratch/tools/`, using the
[Adoptium API](https://adoptium.net/installation/ci-scripts/). The archive's
SHA-256 was verified against the checksum in API metadata before extraction.

Downloaded runtime: `OpenJDK17U-jre_x64_linux_hotspot_17.0.20.1_1.tar.gz`.
Runtime reports `Temurin-17.0.20.1+1`. Full download metadata is retained
locally in `scratch/tools/temurin-metadata.json`; this runtime is not a
checked-in dependency.

The local compiler's `--help` command succeeded and confirmed `-O 0` means no
optimization. This establishes launcher readiness with this runtime, not
successful source compilation.

## Build attempt

Reproduction from the repository root:

```sh
examples/hello-world/build.sh
```

Observed result:

```text
ERROR: Invalid device id specified: 'fr265'.
```

Compiler exit code: `101`. No PRG was produced. The compiler rejected the target
before source compilation, so there is no evidence yet that the source or
resources compile successfully.

The agent's home is `/home/codex`; `/home/codex/.Garmin/ConnectIQ` does not
exist. No compiler device profile was located in the downloaded SDK. A profile
installed in a different desktop user's home would need to be made available to
this execution environment.

## Earlier prerequisite (now resolved)

Install the Forerunner 265 device package using Garmin's SDK Manager, then make
it visible to the compiler here and rerun the build. Garmin documents the SDK
Manager as the tool for downloading the SDK and device library in [Getting
Started][ref-getting-started].

Successful compilation and artifact hashes are now recorded above. Baseline
simulator execution is also verified through the dedicated Ubuntu container;
native dependencies in the agent container remain a separate limitation
described in [the SDK inventory](sdk-inventory.md).

[ref-the-simulator-record]: simulator-container.md#successful-baseline-execution
[ref-getting-started]: https://developer.garmin.com/connect-iq/connect-iq-basics/getting-started/
