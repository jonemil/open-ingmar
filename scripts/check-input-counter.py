#!/usr/bin/env python3
"""Check three taps on the Forerunner 265 compiler-input-counter example.

Run under a fresh 1280x1024 Xvfb display without a window manager. The fixed tap
position assumes the simulator's default window placement and device skin.
Logs and a final screenshot are retained; inspect the screenshot separately.
"""

import argparse
import ctypes
import ctypes.util
from pathlib import Path
import subprocess
import sys
import time


def wait_for(process, log, expected, seconds):
    """Wait for an expected console fragment, failing if the session stops."""
    deadline = time.monotonic() + seconds
    while expected not in log.read_text(errors="replace"):
        if process.poll() is not None:
            raise RuntimeError("Simulator session stopped; inspect the saved session log.")
        if time.monotonic() >= deadline:
            raise RuntimeError(f"Timed out waiting for console output {expected!r}.")
        time.sleep(0.1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prg", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path,
                        help="new directory for logs and the final screenshot")
    args = parser.parse_args()
    prg = args.prg.resolve()
    output = args.output_dir.resolve()
    repo = Path(__file__).resolve().parents[1]
    if not prg.is_file():
        raise RuntimeError(f"Program does not exist: {prg}")
    libraries = [ctypes.util.find_library(name) for name in ("X11", "Xtst")]
    if not all(libraries):
        raise RuntimeError("X11 and Xtst libraries are required for simulated taps.")
    x11, xtst = [ctypes.CDLL(library) for library in libraries]
    x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x11.XOpenDisplay.restype = ctypes.c_void_p
    x11.XFlush.argtypes = [ctypes.c_void_p]
    x11.XCloseDisplay.argtypes = [ctypes.c_void_p]
    xtst.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int,
                                        ctypes.c_int, ctypes.c_int, ctypes.c_ulong]
    xtst.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint,
                                        ctypes.c_int, ctypes.c_ulong]
    display = x11.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot open the X display; run this helper under Xvfb.")
    process = None
    try:
        output.mkdir(parents=True, exist_ok=False)
        simulator_output = repo / "scratch/simulator"
        simulator_output.mkdir(parents=True, exist_ok=True)
        log = simulator_output / "monkeydo.log"
        log.write_text("")  # Exclude console output retained from a previous run.
        with (output / "session.log").open("w") as session_log:
            process = subprocess.Popen(
                [sys.executable, str(repo / "scripts/simulator-session.py"),
                 "--prg", str(prg), "--observe-seconds", "25"],
                cwd=repo, stdout=session_log, stderr=subprocess.STDOUT)
            try:
                wait_for(process, log, "draw\n7\n", 22)
                for count in (8, 9, 10):
                    if not xtst.XTestFakeMotionEvent(display, -1, 322, 466, 0):
                        raise RuntimeError("Cannot move the X pointer.")
                    if not xtst.XTestFakeButtonEvent(display, 1, 1, 0):
                        raise RuntimeError("Cannot press the X pointer button.")
                    x11.XFlush(display)
                    time.sleep(0.1)
                    if not xtst.XTestFakeButtonEvent(display, 1, 0, 0):
                        raise RuntimeError("Cannot release the X pointer button.")
                    x11.XFlush(display)
                    wait_for(process, log, f"select\n{count}\n", 4)
                    wait_for(process, log, f"draw\n{count}\n", 4)
                    time.sleep(0.5)
                if process.wait(timeout=30) != 0:
                    raise RuntimeError("Simulator session failed; inspect session.log.")
                console = log.read_text(errors="replace")
                if console.count("select\n") != 3:
                    raise RuntimeError("Expected exactly three selection callbacks.")
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=15)
                (output / "console.log").write_text(log.read_text(errors="replace"))
                (output / "simulator.log").write_bytes(
                    (simulator_output / "simulator.log").read_bytes())
            (output / "screen.png").write_bytes(
                (simulator_output / "screen.png").read_bytes())
        print(f"PASS: initial 7; three selections and redraws at 8, 9, 10. Captures: {output}")
    finally:
        x11.XCloseDisplay(display)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
