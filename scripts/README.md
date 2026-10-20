# SDK support scripts

## Workspace validation

Run focused tests while editing. Once implementation is stable, use the reusable
final-check runner in the Rust environment:

```sh
python3 scripts/check-workspace.py run --output scratch/workspace-checks/new-run \
  --audit --sdk-index scratch/example-audit/sdk-index.json --rustdoc --msrv
```

Output must be a new directory. Default checks are workspace tests, Clippy, and
formatting. Select `--audit` for language/binary changes, `--rustdoc` for public
API/docs changes, and `--msrv` when the declared minimum Rust version needs
checking. MSRV is read from the workspace manifest. The SDK index has the same
meaning as in the [example audit](#example-toolchain-audit); omit it to use the
audit's specimen discovery. The runner does not rebuild SDK specimens or run
simulators, and it is not added to CI.

With `--audit`, it builds and copies the required tools into the output's `bin/`
directory, then runs the audit alongside the serial Cargo checks. Copies keep
the audit executable paths stable while Cargo replaces build outputs.
This avoids concurrent Cargo commands contending for
the same build directory. Every selected lane must succeed; one failing lane
stops its remaining checks while independent lanes finish. Logs and an atomically
updated `report.json` retain each job's actual status and category. When the user
explicitly requests workflow measurement, add `--timings` to record timestamps
and monotonic durations. Timing is off by default. Timings overlap and must not
be added as sequential elapsed time. Start one final runner per checkout; do not
edit code while it checks it.

The command can run in the background using ordinary shell job control. While it
runs, prepare documentation and the final response. To wait without fixed sleeps,
use the same Linux container/PID namespace:

```sh
python3 scripts/check-workspace.py wait scratch/workspace-checks/new-run --timeout 45
```

Waiting returns immediately on runner exit, or after the requested timeout:
exit 0 means success, 1 means failure/incomplete reporting, and 2 means still
running. A completed report retains its nonzero failure code. Linux pidfd support
is required only for waiting; other environments can run checks in the foreground.
Reading a successful subprocess exit alone never substitutes for the final
selected-check verdict or simulator acceptance.

Use the same reporting for individual SDK, simulator, or other commands:

```sh
python3 scripts/check-workspace.py command --output scratch/checks/sdk-build \
  --category sdk.build -- sh scripts/build-example.sh scratch/compiler-dictionary
```

Arguments after `--` run directly, without an implicit shell. The command's exit
status and category are retained. Use the categories in the
[measurement protocol](../docs/workflow-measurement.md#activity-categories).

## Console specimen check

For examples with one documented complete-PRG compiler command and nonempty
expected console output, prepare the check in the Rust environment:

```sh
python3 scripts/check-example.py prepare compiler-increments \
  --sdk-prg scratch/compiler-increments/build/fr265/compiler-increments.prg \
  --output scratch/check-increments
```

Build the SDK PRG from the same source/configuration first. Preparation builds
our tools, audits this example's saved stages and SDK/generated structured
reconstruction, and signs generated output with `keys/developer_key.der`.
`--key`, `--device`, and `--expected` override defaults; expected text defaults
to the example's `expected.txt`. SDK input and expected text are snapshotted.
Preparation success does not establish runtime acceptance.

In the SDK/simulator environment, using the same shared project files:

```sh
python3 scripts/check-example.py simulate scratch/check-increments
```

The helper runs SDK/generated programs serially on Xvfb and compares each console
to expected text, preserving all lines and normalizing platform line endings.
It prints one verdict, saves captures/logs and `runtime/report.json`, and returns
nonzero for process failure, missing captures, or differing output. Use a new
preparation directory per change; `simulate --output <new-directory>` preserves
an additional runtime attempt. Do not run concurrent simulator sessions.

Use the existing simulator/interactive helpers for programs requiring visual or
input checks. `simulator-session.py --output-dir <new-directory>` also supports
isolated captures directly. The default shared capture location is unchanged.
Helper regression checks use the Python standard library:

```sh
python3 -m unittest discover -s scripts/tests
```

## Example toolchain audit

From the repository root, build the tools and choose a fresh ignored output
directory:

```sh
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm -p ciq-package
python3 scripts/audit-examples.py --output scratch/example-audit/new-run
```

The helper checks documented compiler source/IR/assembly/PRG paths, retained SDK
PRG export/reconstruction with identical repeat exports, and barrel member round
trips. It saves full reports
and logs, and returns nonzero for incomplete coverage, including raw CODE/DATA
fallback or missing SDK specimens. It does not run simulator sessions or rebuild
SDK inputs. Partial DATA sources containing opaque `.bytes` spans also fail the
coverage check. Use `--bin-dir` for another tool build and `--sdk-index <file.json>`
to select SDK PRGs explicitly. The index maps example directory names to one
path or a list of paths for multiple specimens. Use `--example <directory-name>`
to select an affected example during iteration; repeat it for several examples.
Omit that flag for the full final audit. Output is a summary by default;
`--verbose` adds per-example progress.

Unknown opcode discovery uses decoder walks from CODE start and recorded
source-line PCs, never guessed widths or arbitrary-byte resynchronization.
The inventory can grow after earlier unknowns are implemented. See the
[audit findings and limits](../docs/investigations/example-toolchain-audit.md).

## Input counter check

`check-input-counter.py` runs the independently authored
[input counter](../examples/compiler-input-counter/README.md) on Forerunner 265,
sends three taps using XTest, and checks selection/redraw values. Run it under
a fresh `xvfb-run -a -s '-screen 0 1280x1024x24'` display without a window manager:
`python3 scripts/check-input-counter.py --prg <program.prg> --output-dir <new-dir>`.
It retains console/session/simulator logs and a screenshot; review the visible
counter separately. This is a specimen-specific check, not general GUI automation.
The simulator's default skin/window placement and X11/Xtst libraries are required.

## Preview repository documentation

```sh
sh scripts/preview-docs.sh
```

Open `http://localhost:8000/README.md` in your browser. Follow the existing
Markdown links to design, specs, investigations, and examples. Refresh after
editing a page; Ctrl+C stops the server. Use `--port 8080` for another port.
Run on the host when using its browser, unless container port forwarding is
configured.

The first run needs Python 3.10 or newer with `venv`/pip and network access.
It installs pinned rendering dependencies and caches the stylesheet under
ignored `scratch/markdown-preview/`. Later runs can work offline. It serves
Git-tracked and non-ignored repository files on loopback; ignored SDK files,
keys, and scratch data are excluded.

Check local file links and Markdown heading anchors without starting a server:

```sh
sh scripts/preview-docs.sh --check
```

Rendering uses [markdown-it-py's GFM-style parser][markdown-it] and
[GitHub Markdown CSS][github-css], including tables, task lists, alerts,
strikethrough, and heading anchors. It approximates platform rendering; raw
HTML is escaped and platform-specific features such as Mermaid are not rendered.
This preview is separate from Cargo/Rustdoc and is not a publishing requirement.

[markdown-it]: https://markdown-it-py.readthedocs.io/en/latest/using.html
[github-css]: https://github.com/sindresorhus/github-markdown-css

## Build an example

Each example's `build.sh` delegates to the shared `scripts/build-example.sh`:

```sh
examples/hello-world/build.sh
examples/minimal-console/build.sh
```

The helper uses the repository-local SDK, signing key, and downloaded device
data by default, with `CIQ_SDK_HOME`, `CIQ_DEVELOPER_KEY`, and `CIQ_DATA_HOME`
overrides. It retains debug information and disables optimization. See
[example documentation](../examples/README.md) and
[the initial Rust experiment](../docs/investigations/rust-bootstrap.md) for the
disassembly/reassembly workflow.

The default signing key is `keys/developer_key.der`. Keep local developer keys
in the Git-ignored `keys/` directory; `CIQ_DEVELOPER_KEY` can select another
location.

## Use the simulator interactively

From a terminal on your Linux desktop, where Podman and `xauth` are installed:

```sh
sh scripts/run-simulator-container.sh --interactive
```

This reuses the existing Ubuntu image, opens the simulator on your desktop, and
loads Hello World for `fr265`. You can use the simulator's menus, mouse, and
keyboard. It stays open until you close the simulator window or press Ctrl+C in
the launching terminal. If the app or its console client exits, the simulator
window remains available. Logs go to `scratch/simulator-interactive/`; this mode
does not take a screenshot of your desktop.

Put `--interactive` first. You can choose another built example and device:

```sh
sh scripts/run-simulator-container.sh --interactive \
    --device fr265 --prg examples/hello-world/build/fr265/hello-world.prg
```

The launcher requires a local X11 `DISPLAY`, including Xwayland on a Wayland
desktop. It copies the current display's X authority entry into a temporary
file, mounts that file and the X11 socket, and removes the file after the
container stops. If no cookie is found, set `XAUTHORITY` to your desktop
session's authority file and retry. No `xhost` changes are needed.

The GUI container uses `--security-opt=label=disable` to access the existing
desktop socket and cookie without relabeling them. This disables SELinux
container separation for that invocation, while retaining the unprivileged user,
dropped capabilities, and `no-new-privileges`. The headless mode retains its
existing SELinux configuration. See [Podman's mount and security
options][ref-podman-s-mount-and-security-options].

Launcher configuration and session lifetime have passed local checks with
synthetic processes. The actual desktop connection still needs a host-side run;
Podman and a desktop display are unavailable in the agent container.

## Run the simulator with a virtual display

The simulator session helper clears its console log before startup and captures
the display after a runtime process exit, reporting the native exit code. A
startup failure does not establish that a PRG ran; do not interpret an older
screen capture as a new runtime result.

After building Hello World, run from the host where Podman is installed:

```sh
sh scripts/run-simulator-container.sh
```

This builds a dedicated Ubuntu simulator image if needed, runs the official
simulator under Xvfb, attempts to load the built PRG, and saves `screen.png`,
`simulator.log`, and `monkeydo.log` in ignored `scratch/simulator/`. Console
output is saved as it arrives. After 15 seconds the script captures the display
and ends the session, even if the console client remains active. Pass
`--observe-seconds 30` for a longer observation window. Inspect the screenshot
and logs to verify actual app execution.

The host container session has successfully run Hello World for `fr265`: console
output and the saved screenshot both show the greeting and `41`. In this
successful run, `monkeydo` remained active until the runner ended the session.
See [the simulator investigation][ref-the-simulator-investigation] for evidence
and the visible desktop alternative.

### Check deterministic console output

For a console-only experiment with an exact expected transcript, use `cmp`
instead of manually reading the entire log. For Hello World:

```sh
printf 'Hello from Monkey C!\n41\n' > scratch/hello-world.expected
cmp -s scratch/hello-world.expected scratch/simulator/monkeydo.log
```

Check the simulator runner's exit status first and use the log from that fresh
run. `cmp` returns 0 for equal files, 1 for a mismatch, and 2 for a file error.
On failure, inspect the difference and relevant simulator diagnostics. Compare
the whole transcript: extra output or runtime errors must not be discarded to
make it match. This verifies console output only; experiments involving the
display still require checking the captured screen.

For repeated checks that need parsing, add a focused helper that reports a
short result and returns nonzero on failed expectations or unreadable input.
Keep full logs under `scratch/` and inspect excerpts when a check fails or new
behavior needs investigation. Reuse ordinary commands for simple comparisons.

## Download a device package

Run on your desktop with Python 3.10 or newer and a browser on the same
computer:

```sh
python3 scripts/fetch-device.py fr265 --include-fonts
```

Open the printed localhost URL, sign in to your Garmin account in the browser,
and return to the terminal. The script exchanges the browser's service ticket
for an access token, finds `fr265` in the device catalogue, and downloads its
hardware-part-number ZIP. It uses only the Python standard library and does not
require the official SDK manager or the downloaded reference CLI.

By default, files are installed under:

```text
~/.Garmin/ConnectIQ/Devices/fr265/
~/.Garmin/ConnectIQ/Fonts/
```

These home-directory paths follow the Linux layout. To download to a
repository-local staging area instead:

```sh
python3 scripts/fetch-device.py fr265 --include-fonts --root scratch/garmin-data
```

The Hello World build script defaults to `scratch/garmin-data` and configures
the Java compiler's device discovery for that location. For a different download
root, set `CIQ_DATA_HOME` when building:

```sh
CIQ_DATA_HOME=/path/to/garmin-data examples/hello-world/build.sh
```

The directory must be visible to the environment running the compiler; a path
that exists only on the host must be mounted into a container before a container
build can use it.

Tokens and tickets stay in memory. The script never asks for your password, logs
callback URLs containing tickets, or saves tokens. Existing device installations
are preserved; `--include-fonts` can fetch missing fonts for a previously
installed device. If a font fails, the device files may already be installed.

The callback listens only on loopback. Run this on the host when using the
host's browser; opening the URL from outside an isolated container will reach
the wrong localhost unless port forwarding is arranged. `--port` can fix the
callback port, and `--login-timeout` changes the default ten-minute wait.

After the package is visible to the compiler, retry:

```sh
examples/hello-world/build.sh
```

The script has offline verification using synthetic packages and mocked network
responses. A real account login, Garmin's current browser redirect behavior, and
authenticated downloads remain unverified. HTTP 401/403 or a login timeout will
be reported without printing token responses. Protocol details are in
[the local specification](../docs/specs/build/device-download.md).

## Barrel builds and alternate Jungles

`sh scripts/build-barrel.sh examples/number-barrel` packages the independently
authored library to its ignored `build/number-barrel.barrel`. It discovers Java
and project-local device data like the app helper, requires no signing key, and
refuses an existing archive to avoid accepting stale output after an SDK
failure. Move a prior output aside to rebuild it.

Both build helpers accept `CIQ_JUNGLE_FILES`. Relative paths resolve from the
example directory; use absolute paths for inputs elsewhere. For example,
`CIQ_JUNGLE_FILES=monkey-source.jungle sh scripts/build-example.sh examples/barrel-app`
uses the source-barrel configuration instead of its default archive
configuration. All app builds still use the external developer key and `-O 0`.

[The multi-file/barrel investigation][ref-the-multi-file-barrel-investigation]
records successful SDK builds, Rust reconstruction, and simulator runs through
the container-broker plugin. Its approved Garmin setup mounts this workspace and
supports repeated command execution; the simulator session can run under
`xvfb-run` inside that container without nesting Podman.

[ref-podman-s-mount-and-security-options]: https://docs.podman.io/en/latest/markdown/podman-run.1.html

[ref-the-simulator-investigation]: ../docs/investigations/simulator-container.md
[ref-the-multi-file-barrel-investigation]: ../docs/investigations/multi-file-and-barrels.md
