# Simulator container readiness and first launch attempt

Date: 2026-10-07. Goal: run the built Hello World PRG in Garmin's official simulator.

## Observations in the current agent container

- Direct simulator launch exited with code `127`: `libsecret-1.so.0` could not be loaded.
- `ldd connectiq/bin/simulator` also reports missing GTK 3, WebKitGTK 4.0, JavaScriptCoreGTK 4.0, Soup 2.4, libusb, JPEG 8, graphics, and font libraries.
- `DISPLAY`, `WAYLAND_DISPLAY`, `XAUTHORITY`, and `XDG_RUNTIME_DIR` are unset.
- `/tmp/.X11-unix` and `/run/user/1000` do not exist here.
- No `Xvfb`, `xauth`, `xdpyinfo`, `sudo`, or `podman` command is available in this container.
- The SDK shell helper's embedded transport usage names `127.0.0.1:1234` as its default simulator connection.

This is a library and display-transport problem before any X authorization test can take place. An elevated launch produced the same library error. The separate host container subsequently ran the app successfully, as recorded below.

## Prepared virtual-display route

[containers/simulator/Dockerfile](../../containers/simulator/Dockerfile) defines a separate Ubuntu 22.04 image with Java, native simulator dependencies, and Xvfb. [Ubuntu's package index](https://packages.ubuntu.com/jammy/libwebkit2gtk-4.0-37) confirms the required WebKitGTK 4.0 ABI is available there. Garmin's [setup documentation](https://developer.garmin.com/connect-iq/connect-iq-basics/getting-started/) specifies Ubuntu as its supported Linux distribution.

From the host, in this repository:

```sh
sh scripts/run-simulator-container.sh
```

The wrapper builds the image if absent, mounts this repository using shared SELinux relabeling, and runs the official SDK simulator under a virtual display. It exposes no host display or X cookie. The session script points the container user's Garmin data root at `scratch/garmin-data`, waits for the simulator shell, loads Hello World using `monkeydo`, and saves a screenshot and logs under `scratch/simulator/` before shutting down.

The user built the image on the host and, after the corrections below, successfully ran Hello World. Podman remains unavailable inside the agent container, but shared capture files allowed direct inspection of the resulting screen and console log. A successful upload alone must not be reported as changed behavior or successful app execution; this baseline is confirmed by the visible greeting and `41` plus console output.

## Diagnosing startup

The user reported a successful host-side image build (image ID prefix `854b9de666ca`), followed by no terminal output. At the time of inspection, `scratch/simulator/` had not appeared, so the Python session had not reached log creation. Whether the delay is Podman startup or Xvfb startup is not yet established.

The wrapper and session now print progress at container launch, after Xvfb starts, and before the simulator shell wait. Changes are in mounted scripts and require no image rebuild. For a running attempt, inspect containers on the host:

```sh
podman ps -a --format '{{.ID}}  {{.Status}}  {{.Image}}  {{.Names}}'
podman top <simulator-container-id> pid args
podman logs <simulator-container-id>
```

The shell-readiness wait is bounded to 30 seconds, but the outer image/container/Xvfb startup has not yet been bounded. Preserve host output to determine which stage needs correction.

The host process list subsequently showed only PID 1 running `xvfb-run` and its Xvfb child, with no Python process. This locates the stall in the Xvfb readiness handshake. The launcher now uses `podman run --init` so a container init process occupies PID 1 and `xvfb-run` runs as a normal child. After this change, the user reported both Python startup and simulator shell readiness, confirming that this attempt progressed past the earlier stall. [Podman's init option](https://docs.podman.io/en/latest/markdown/podman-run.1.html) forwards signals and reaps child processes.

## Console timeout and capture correction

The next host attempt reached `Simulator shell ready. Loading PRG...`, then the Python runner reported that `monkeydo` exceeded its 60-second timeout. The only simulator log entries were JPEG-handler and layout debug messages. The runner had buffered the console output and waited for the client to exit before saving it or taking a screenshot, so this attempt supplies no evidence that distinguishes a stalled load from a running app with an open console session.

The revised runner writes `monkeydo.log` immediately and echoes new console output during a bounded observation window (15 seconds by default). It captures the Xvfb desktop while a still-active client remains connected, then stops the client and simulator process groups. A nonzero client exit still reports failure after attempting the capture. An active client is reported as an observation, not as proof of successful loading or execution.

Rerun the host command above; mounted-script changes require no image rebuild. Increase the observation window if needed:

```sh
sh scripts/run-simulator-container.sh --observe-seconds 30
```

Local verification used synthetic simulator and console processes, with the display capture command replaced by a fixture. It confirmed that a long-lived client remains alive during capture, console output is preserved, a client exiting with code 7 still produces a capture and reports failure, and both client processes are stopped afterward. This checks runner lifecycle behavior; the subsequent real simulator result follows.

## Successful baseline execution

Experimentally observed on 2026-10-07: the user reran `sh scripts/run-simulator-container.sh` on the host. During the 15-second observation window, `monkeydo` printed:

```text
Hello from Monkey C!
41
```

The runner captured the display while `monkeydo` remained active. Direct inspection of `scratch/simulator/screen.png` shows the simulated Forerunner 265 displaying `Hello world!` and `41`. The shared `monkeydo.log` matches the terminal output. This establishes actual startup and rendering of the original Garmin-generated Hello World PRG, rather than merely shell readiness or successful upload. The successful run also demonstrates that client termination is not required before inspecting app behavior.

Inputs: SDK 9.2.0, target `fr265`, the project-local device package and fonts, and the Ubuntu 22.04/Xvfb image described above. The PRG hash still matches the [successful build record](hello-world-build.md).

| Local artifact | SHA-256 |
|---|---|
| `examples/hello-world/build/fr265/hello-world.prg` | `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7` |
| `scratch/simulator/screen.png` | `0f93c99169e458528b78cb1e781eef5c06965c108f42f8ed99cb7906321ebbae` |
| `scratch/simulator/monkeydo.log` | `677d78dc9d8956d941e7be0f1900cdd6d07f0719775d63f6be7881f1a24898e6` |

Captures remain ignored local evidence and are overwritten by subsequent sessions. No PRG rewrite or instruction modification has yet been tested; this result provides the baseline for those experiments.

## Broker execution verification

Experimentally observed on 2026-10-10: the MCP adapter started the
`garmin_simulator` setup through the host container broker, then executed:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' python3 scripts/simulator-session.py
```

The setup uses `localhost/ciq-simulator:9.2.0`, a writable shared-label mount of
the host Garmin repository at `/workspace/garmin`, keep-id UID/GID 1000, and a
`/bin/sleep infinity` entrypoint. Running Xvfb through broker exec keeps its
launcher away from PID 1. No host desktop socket or X authority cookie is mounted.

The simulator command returned exit code 0. Console output showed
`Hello from Monkey C!` and `41`; direct screenshot inspection showed
`Hello world!` and `41` on the simulated Forerunner 265. The PRG, screenshot,
and console-log hashes exactly match the baseline artifact table above.
The broker successfully stopped and removed the test workspace; its instance
list was empty afterward. This verifies headless execution and cleanup through
the broker, without establishing desktop interactive support.

## Interactive desktop route

The same Ubuntu image can now run directly on the host's X11/Xwayland desktop, without rebuilding or modifying the agent container:

```sh
sh scripts/run-simulator-container.sh --interactive
```

The wrapper checks the local `DISPLAY` socket, uses host `xauth` to copy only that display's authority entries into a temporary file, and changes the numeric address family to FamilyWild so the cookie works with the container's hostname. It mounts the socket directory and authority file read-only and selects the Python entry point directly, bypassing Xvfb. GTK is directed to use X11. The copied authority file is deleted when the launcher exits; no X server access-control settings are changed.

This invocation uses `--security-opt=label=disable` so the GUI can access host desktop objects without relabeling the X11 socket directory or session authority file. This relaxes SELinux container separation for the GUI container specifically. The container still runs as the mapped unprivileged user with dropped capabilities and `no-new-privileges`. Headless invocations retain their existing labeling configuration. [Podman's mount documentation](https://docs.podman.io/en/latest/markdown/podman-run.1.html) explains bind mounts and SELinux labels.

The session first checks display connectivity using `xdpyinfo`, then launches the simulator and loads the PRG. Interactive mode streams console output and waits until the simulator exits or the user interrupts the session. A console-client exit is reported without closing the simulator. Both child process groups are cleaned up on shutdown. Logs are kept separately under `scratch/simulator-interactive/`, and no desktop screenshot is taken.

Local synthetic-process checks verified that the interactive session outlives the observation timeout, a console-client exit leaves the simulator open, closing the simulator ends the session and stops a live client, and headless capture still works. A mocked Podman/xauth check with a real local Unix socket verified desktop mount arguments, entry-point selection, preservation of the display screen number, temporary-cookie cleanup, and rejection of a missing display. Shell and Python syntax checks passed. These checks do not establish a real host display connection; that remains the next host-side experiment.

The virtual-display route needs no host X access or X authority changes.

## Broker desktop connection

Experimentally observed on 2026-10-10: MCP `broker_start` launched the
`garmin_simulator_desktop` profile using `localhost/ciq-simulator:9.2.0` with
`python3 scripts/simulator-session.py --interactive` and a container init process.
The broker used its inherited desktop environment, copied the current display's
X authority cookie with FamilyWild, and mounted the cookie and X11 socket
read-only. This profile uses `label=disable`; it does not set `GDK_BACKEND`.

The initial `xdpyinfo` check succeeded, the simulator shell became ready, and
both broker logs and `scratch/simulator-interactive/monkeydo.log` showed
`Hello from Monkey C!` and `41`. The container remained running and was left
open for the user. This establishes desktop connectivity and app console
execution without a forced GTK backend. Visible window/rendering confirmation
is pending the user's observation; no desktop screenshot was taken.

The user subsequently reported no visible window. Inspection inside the running
container confirmed `--interactive`, no Xvfb process, and `DISPLAY=:0`.
`xwininfo` found the simulator window (485 by 700 pixels at X=880, Y=125) with
`Map State: IsViewable`; `xprop` reported normal state and the simulator PID.
A window-manager activation request was sent. These observations establish an
X11 window on that display but do not establish that it is the user's expected
desktop session; comparison with the host terminal's `DISPLAY` is pending.

The user clarified that the terminal was accessed through SSH and `screen`,
so the host desktop was not visible. A fresh `ssh -X` connection successfully
ran `xterm`. After passing that SSH session's display and authority path into
the broker's separate `BROKER_SSH_DISPLAY` and `BROKER_SSH_XAUTHORITY` variables
and restarting the service, MCP launched `garmin_simulator_ssh` on 2026-10-10.
This profile uses host networking to reach sshd's loopback X11 proxy and mounts
only the copied display cookie, with no host X11 socket directory.

Inspection inside the running SSH profile confirmed `DISPLAY=localhost:10.0`.
The initial `xdpyinfo` check succeeded and the simulator shell became ready.
The workspace was left running, and the user confirmed that the simulator
window appeared on their local computer. Broker logs also showed
`Hello from Monkey C!` and `41`. This establishes successful interactive
SSH-forwarded simulator launch without setting `GDK_BACKEND`.
