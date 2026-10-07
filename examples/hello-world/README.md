# Hello World

A small independently written watch application targeting the Forerunner 265 (`fr265`). It displays `Hello world!` and `41`, and prints a greeting and `41` to the simulator console on startup. The `probe()` function returns the numeric result, providing a simple future bytecode-modification target.

## Build

From the repository root:

```sh
examples/hello-world/build.sh
```

Requires a compatible Java runtime, the Garmin SDK, a signing key, and the `fr265` device package installed through Garmin's SDK Manager or [our standalone downloader](../../scripts/README.md). The defaults are `connectiq/`, `developer_key.der`, and device data under `scratch/garmin-data/` at the repository root. The script uses Java on `PATH`, then an existing `JAVA_HOME`, then a locally downloaded runtime under `scratch/tools/java/` if available.

Override SDK and key locations with absolute paths:

```sh
CIQ_SDK_HOME=/path/to/sdk CIQ_DEVELOPER_KEY=/path/to/key.der \
    CIQ_DATA_HOME=/path/to/garmin-data \
    examples/hello-world/build.sh
```

The script disables optimization (`-O 0`) and retains debug information for inspection. Build artifacts go into the Git-ignored `build/fr265/` directory. The expected executable is `build/fr265/hello-world.prg`.

`CIQ_DATA_HOME` is the directory containing `Devices/` and `Fonts/`. The script creates a generated Java user-home directory under `build/sdk-user-home/` and links its `.Garmin/ConnectIQ` directory to that data root. It then invokes the SDK compiler with `-Duser.home` pointing there. This Linux setup was verified with SDK 9.2.0.

## Run

The verified container route, from the host with Podman installed:

```sh
sh scripts/run-simulator-container.sh
```

This saves a screenshot and console logs in `scratch/simulator/` before ending the session. See [the runner documentation](../../scripts/README.md) for observation options.

To open the simulator on your desktop and interact with it, run from a desktop terminal:

```sh
sh scripts/run-simulator-container.sh --interactive
```

This mode loads Hello World and stays open until you close the window or press Ctrl+C. It requires host `xauth` and a local X11/Xwayland display; see [interactive setup and verification status](../../scripts/README.md#use-the-simulator-interactively).

After a successful build, start the official simulator in a desktop environment:

```sh
connectiq/bin/simulator
```

Then, with Java available on `PATH`, run:

```sh
connectiq/bin/monkeydo examples/hello-world/build/fr265/hello-world.prg fr265
```

## Verification status

Hello World compiled successfully with SDK 9.2.0, the local Java runtime, and the downloaded `fr265` profile on 2026-10-07. On the same date, the host's Ubuntu/Xvfb simulator session printed the greeting and `41`; the captured screen visibly showed `Hello world!` and `41`. See [the build record](../../docs/investigations/hello-world-build.md) and [simulator evidence](../../docs/investigations/simulator-container.md).
