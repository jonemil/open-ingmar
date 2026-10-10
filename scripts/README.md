# SDK support scripts

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
[the local specification](../docs/specs/device-download.md).

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
