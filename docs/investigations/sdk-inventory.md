# Local SDK inventory

Date: 2026-10-07. Scope: inspect the downloaded SDK and readiness for reference
compilation and simulator execution. No proprietary compiler implementation was
inspected.

Follow-up: a compatible local Java runtime and the downloaded `fr265` device
package are now available, and
[Hello World compiles successfully](hello-world-build.md). The initial readiness
observations below are historical; native simulator dependencies and simulator
execution remain pending.

## Observed installation

- SDK root: `connectiq/` (Git-ignored).
- `connectiq/bin/version.txt` reports `9.2.0`.
- Download archive: `connectiq-sdk-lin-9.2.0-2026-06-09-92a1605b2.zip`. The
  filename is recorded as found; it is not independent verification of the
  release date or provenance.
- Launchers include `monkeyc`, `monkeydo`, `barrelbuild`, and `barreltest`.
- Native simulator: `connectiq/bin/simulator`.
- Samples and project templates are available under `connectiq/samples/` and
  `connectiq/bin/templates/`.
- Device reference pages include
  `connectiq/doc/docs/Device_Reference/fr265.html` and `fr265s.html`.
  Documentation presence does not establish that the corresponding
  build/simulator device package is installed.

## Locally observed command interface

The `monkeyc` launcher invokes Java with `bin/monkeybrains.jar`; it requires a
usable `java` command. Compiler flags and Java version compatibility have not
yet been checked by running the compiler.

The `monkeydo` launcher's usage text specifies:

```text
monkeydo executable device_id [-n] [-a additional_file] [-t | -t test_name]
```

It states that the simulator must already be running. The intended baseline
invocation, after a successful build and device-package verification, is:

```sh
connectiq/bin/monkeydo scratch/reference.prg fr265
```

This invocation has not been tested.

## Execution-environment readiness

Commands used:

```sh
cat connectiq/bin/version.txt
java -version
ldd connectiq/bin/simulator
```

Observed in the agent's current execution environment:

- `java` is not available on `PATH`.
- `ldd` reports unresolved dependencies for the simulator, including GTK 3,
  WebKitGTK 4.0, Soup 2.4, JavaScriptCoreGTK 4.0, libsecret, libusb, and
  graphics/font libraries.
- `file` is also unavailable; this is an inspection-tool limitation, not an SDK
  requirement.
- No compiler device profile was located in the SDK scan. Some searched
  home-directory locations were absent or inaccessible, so this does not
  establish that no device package exists elsewhere.

These observations describe the agent's execution environment, not necessarily
the user's desktop installation. Compilation and simulator execution have not
yet been attempted successfully.

Current setup commands are in [SDK support scripts](../../scripts/README.md).
[The plan](../plan.md) owns remaining toolchain work.
