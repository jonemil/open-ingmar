#!/usr/bin/env python3
"""Run an official simulator session on Xvfb or an interactive desktop."""

import argparse
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time


def stop_process(process):
    """Stop the launcher and its children, including the Java console client."""
    if process is None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--device", default="fr265")
    parser.add_argument("--prg", type=Path, default=Path("examples/hello-world/build/fr265/hello-world.prg"))
    parser.add_argument("--observe-seconds", type=float, default=15,
                        help="seconds to observe the console before capturing the display (default: 15)")
    parser.add_argument("--interactive", action="store_true",
                        help="keep the desktop simulator open until its window is closed or Ctrl+C is pressed")
    args = parser.parse_args()
    if args.observe_seconds <= 0:
        parser.error("--observe-seconds must be positive")
    if args.interactive:
        result = subprocess.run(["xdpyinfo"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                text=True, timeout=5)
        if result.returncode:
            raise RuntimeError(f"Cannot connect to the desktop display: {result.stderr.strip()}")
    print("Display ready. Preparing simulator session...", flush=True)
    repo = Path(__file__).resolve().parents[1]
    sdk = repo / "connectiq"
    data = repo / "scratch/garmin-data"
    prg = args.prg.resolve()
    if not prg.is_file() or not (data / "Devices" / args.device / "simulator.json").is_file():
        raise RuntimeError("PRG or simulator device profile missing; build the example and download its device first.")

    garmin = Path.home() / ".Garmin"
    garmin.mkdir(parents=True, exist_ok=True)
    link = garmin / "ConnectIQ"
    if not link.exists() and not link.is_symlink():
        link.symlink_to(data, target_is_directory=True)
    if link.resolve() != data.resolve():
        raise RuntimeError("Container Garmin data directory already points elsewhere.")

    output = repo / "scratch" / ("simulator-interactive" if args.interactive else "simulator")
    output.mkdir(parents=True, exist_ok=True)
    with (output / "simulator.log").open("w") as log:
        print("Launching Garmin simulator; waiting up to 30 seconds for its shell...", flush=True)
        simulator = subprocess.Popen([str(sdk / "bin/simulator")], cwd=sdk / "bin", stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
        console = None
        try:
            deadline = time.monotonic() + 30
            while True:
                if simulator.poll() is not None:
                    raise RuntimeError(f"Simulator exited during startup; inspect {output / 'simulator.log'}.")
                try:
                    with socket.create_connection(("127.0.0.1", 1234), timeout=0.5):
                        break
                except OSError:
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Simulator shell did not become ready on port 1234 within 30 seconds.")
                    time.sleep(0.25)
            print("Simulator shell ready. Loading PRG...", flush=True)
            console_log = output / "monkeydo.log"
            with console_log.open("w") as client_log:
                console = subprocess.Popen([str(sdk / "bin/monkeydo"), str(prg), args.device],
                                           cwd=repo, stdout=client_log, stderr=subprocess.STDOUT,
                                           start_new_session=True)
                if args.interactive:
                    print("Simulator is open on your desktop. Use its controls and menus; close the window or press Ctrl+C to stop.",
                          flush=True)
                    print("Console logs: scratch/simulator-interactive/", flush=True)
                else:
                    print(f"Observing for {args.observe_seconds:g} seconds; console output is saved as it arrives...",
                          flush=True)
                deadline = time.monotonic() + args.observe_seconds
                client_exit_reported = False
                with console_log.open("r", errors="replace") as reader:
                    while True:
                        print(reader.read(), end="", flush=True)
                        if args.interactive:
                            if simulator.poll() is not None:
                                print(reader.read(), end="", flush=True)
                                if simulator.returncode:
                                    raise RuntimeError(f"Simulator exited with code {simulator.returncode}; inspect {output / 'simulator.log'}.")
                                print("Simulator window closed. Ending session.", flush=True)
                                return
                            if console.poll() is not None and not client_exit_reported:
                                print(f"monkeydo exited with code {console.returncode}; simulator remains open.", flush=True)
                                client_exit_reported = True
                            time.sleep(0.25)
                            continue
                        if console.poll() is not None or simulator.poll() is not None or time.monotonic() >= deadline:
                            break
                        time.sleep(0.25)
                    print(reader.read(), end="", flush=True)
            if simulator.poll() is not None:
                raise RuntimeError("Simulator exited; inspect scratch/simulator/simulator.log and monkeydo.log.")
            if console.poll() is None:
                print("monkeydo is still active. Capturing the display before ending the session...", flush=True)
            else:
                print(f"monkeydo exited with code {console.returncode}. Capturing the display...", flush=True)
            subprocess.run(["import", "-window", "root", str(output / "screen.png")], check=True, timeout=15)
            print("Session captured in scratch/simulator/: screen.png, simulator.log, monkeydo.log.", flush=True)
            if console.poll() is not None and console.returncode:
                raise RuntimeError(f"monkeydo exited with code {console.returncode}; inspect the capture and logs.")
            print("Inspect the capture and logs against the chosen example's expected output; an active console alone does not confirm execution.")
        finally:
            stop_process(console)
            stop_process(simulator)


if __name__ == "__main__":
    def interrupted(signum, frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, interrupted)
    try:
        main()
    except KeyboardInterrupt:
        print("Simulator session stopped.", file=sys.stderr)
        sys.exit(130)
    except (RuntimeError, OSError, subprocess.SubprocessError) as exc:
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)
