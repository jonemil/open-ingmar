# Simulator environment and execution evidence

Experiments: 2026-10-07 and 2026-10-10. SDK 9.2.0, Forerunner 265, project-local
device data/fonts, and the [Hello World build](hello-world-build.md).
The [support scripts](../../scripts/README.md) own current launch commands;
[the container recipe](../../containers/simulator/Dockerfile) owns dependencies.

## Environment and failed attempts

The original agent container could not launch the simulator: exit 127 named
missing `libsecret-1.so.0`; dependency inspection also found missing GTK 3,
WebKitGTK/JavaScriptCoreGTK 4.0, Soup 2.4, libusb, JPEG 8, graphics, and fonts.
No display environment/socket or Xvfb/Podman tooling was available there.
Elevation produced the same library failure, before X authorization was tested.

The separate Ubuntu 22.04 image supplies those ABIs, Java, and Xvfb.
[Ubuntu's package index][ref-ubuntu-s-package-index] identifies WebKitGTK 4.0;
Garmin's [setup documentation][ref-setup-documentation] names Ubuntu support.
Two launcher failures were isolated:

- Only `xvfb-run` as PID 1 and its Xvfb child appeared; Python/log creation never
  started. Using a container init process moved Xvfb away from PID 1 and startup
  progressed. [Podman init behavior][ref-podman-s-init-option] is the reference.
- After shell readiness, a 60-second `monkeydo` timeout supplied no execution
  evidence because output/capture had been buffered until client exit. Streaming
  logs and capturing while the client remained connected resolved that ambiguity.

Synthetic-process checks covered a long-lived console during capture, preserved
output, nonzero client exit (7) with failure reporting, and process-group cleanup.
These checks establish runner behavior, not Garmin app execution.

## Successful baseline execution

On 2026-10-07, a 15-second headless session printed:

```text
Hello from Monkey C!
41
```

The screenshot showed `Hello world!` and `41` on the simulated watch. Console and
display were inspected independently; shell readiness/upload alone were not the
acceptance criterion. The client remained active during capture, establishing
that it need not exit before behavior can be observed.

- **Local artifact:** `examples/hello-world/build/fr265/hello-world.prg`.
  **SHA-256:**
  `8608ed040f18294a2a9af870f53e26e64a0d2fe3ef8c032d600043b591d182d7`.

- **Local artifact:** `scratch/simulator/screen.png`. **SHA-256:**
  `0f93c99169e458528b78cb1e781eef5c06965c108f42f8ed99cb7906321ebbae`.

- **Local artifact:** `scratch/simulator/monkeydo.log`. **SHA-256:**
  `677d78dc9d8956d941e7be0f1900cdd6d07f0719775d63f6be7881f1a24898e6`.

On 2026-10-10, the broker repeated the same result using
`localhost/ciq-simulator:9.2.0`, a shared-label `/workspace/garmin` mount, mapped
UID/GID 1000, and Xvfb launched through exec rather than as PID 1. The PRG,
console, and screenshot hashes matched the baseline. Broker shutdown removed
the test workspace. This is headless execution/cleanup evidence.

## Interactive desktop and SSH

The host desktop route mounts the selected X11 socket and a temporary copy of
that display's authority entries read-only. FamilyWild permits the cookie under
the container hostname. `label=disable` avoids relabeling host desktop objects;
the GUI container remains unprivileged with dropped capabilities and
`no-new-privileges`. No X server access-control changes are needed. See
[Podman's mount reference][ref-podman-s-mount-documentation].

Synthetic checks covered continued interaction after the observation timeout,
console-client exit leaving the simulator open, simulator-close cleanup,
temporary-cookie removal, display screen numbers, and missing-display rejection.
They did not establish a real desktop connection.

The broker desktop profile then passed `xdpyinfo`, loaded the app, and printed
the greeting/41 without a forced GTK backend. The user saw no local window:
`DISPLAY=:0` contained a viewable simulator window, but was the host desktop
rather than the user's SSH session. Window existence was insufficient evidence
of visibility on the intended screen.

A fresh `ssh -X` session ran `xterm`. Passing its display/authority through
`BROKER_SSH_DISPLAY` and `BROKER_SSH_XAUTHORITY` selected `garmin_simulator_ssh`,
with host networking for sshd's loopback proxy and only the copied cookie mount.
`DISPLAY=localhost:10.0` passed `xdpyinfo`; the app printed greeting/41 and the
user confirmed the window on their local computer. This establishes SSH-forwarded
interactive launch for that environment, not arbitrary display configurations.

[ref-ubuntu-s-package-index]: https://packages.ubuntu.com/jammy/libwebkit2gtk-4.0-37
[ref-setup-documentation]: https://developer.garmin.com/connect-iq/connect-iq-basics/getting-started/
[ref-podman-s-init-option]: https://docs.podman.io/en/latest/markdown/podman-run.1.html
[ref-podman-s-mount-documentation]: https://docs.podman.io/en/latest/markdown/podman-run.1.html
