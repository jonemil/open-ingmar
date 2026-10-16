#!/usr/bin/env python3
"""Run final Rust checks alongside an independent, freshly built example audit."""

import argparse
from contextlib import contextmanager
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[1]


def process_identity(pid):
    """Read Linux process start ticks to distinguish a reused PID."""
    try:
        return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19]
    except (OSError, IndexError):
        return None


class Checks:
    """Save checked subprocess outcomes and timings without parsing their logs."""

    def __init__(self, output, timings=False):
        self.output = output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.lock = threading.RLock()
        self.cancelled = threading.Event()
        self.processes = set()
        self.timings = timings
        self.clock = time.monotonic() if timings else None
        self.report = {"status": "running", "pid": os.getpid(),
                       "process_identity": process_identity(os.getpid()),
                       "timings": timings, "jobs": []}
        if timings:
            self.report["start"] = time.time()
        self.save()

    def save(self):
        """Atomically replace the report while holding the state lock."""
        with self.lock:
            temporary = self.output / "report.tmp"
            temporary.write_text(json.dumps(self.report, indent=2) + "\n")
            temporary.replace(self.output / "report.json")

    def job(self, name, category, argv):
        """Run one check and retain its exact exit status, log, and duration."""
        if self.cancelled.is_set():
            return 1
        clock = time.monotonic() if self.timings else None
        row = {"name": name, "category": category, "argv": list(map(str, argv)),
               "status": "running", "log": f"{name}.log"}
        if self.timings:
            row["start"] = time.time()
        with self.lock:
            self.report["jobs"].append(row)
            self.save()
        code = 127
        with (self.output / row["log"]).open("w") as log:
            try:
                with self.lock:
                    if self.cancelled.is_set():
                        raise RuntimeError("validation cancelled")
                    process = subprocess.Popen(argv, cwd=ROOT, stdout=log,
                                               stderr=subprocess.STDOUT,
                                               start_new_session=True)
                    self.processes.add(process)
                try:
                    code = process.wait()
                finally:
                    with self.lock:
                        self.processes.discard(process)
            except (OSError, RuntimeError) as exc:
                log.write(f"{exc}\n")
        with self.lock:
            row.update(exit_code=code, status="passed" if code == 0 else "failed")
            if self.timings:
                row.update(end=time.time(), duration_seconds=time.monotonic() - clock)
            self.save()
        elapsed = f" ({row['duration_seconds']:.1f}s)" if self.timings else ""
        print(f"{'PASS' if code == 0 else 'FAIL'} {name}{elapsed}",
              flush=True)
        return code

    def lane(self, jobs):
        """Run dependent checks serially, stopping this lane on its first failure."""
        for name, category, argv in jobs:
            code = self.job(name, category, argv)
            if code:
                return code
        return 0

    def parallel(self, lanes):
        """Run independent lanes concurrently; every lane must succeed."""
        with ThreadPoolExecutor(max_workers=len(lanes)) as pool:
            futures = [pool.submit(self.lane, jobs) for jobs in lanes]
            try:
                codes = [future.result() for future in futures]
            except BaseException:
                self.cancel()
                raise
        return 1 if any(codes) else 0

    def cancel(self):
        """Stop process groups started by this runner after an interruption."""
        self.cancelled.set()
        with self.lock:
            processes = list(self.processes)
        for process in processes:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        for process in processes:
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()

    def finish(self, code, status="complete"):
        """Publish the final verdict only after all selected lanes have ended."""
        self.report.update(status=status, exit_code=code)
        if self.timings:
            self.report.update(end=time.time(), duration_seconds=time.monotonic() - self.clock)
        self.save()


@contextmanager
def session(output, timings=False):
    """Record interrupted runs and clean up children before restoring signals."""
    checks = Checks(output, timings=timings)
    def interrupt(*_):
        raise KeyboardInterrupt()
    previous = signal.signal(signal.SIGTERM, interrupt)
    try:
        yield checks
    except BaseException:
        checks.cancel()
        checks.finish(1, "interrupted")
        raise
    finally:
        signal.signal(signal.SIGTERM, previous)


def run(args):
    """Build audit tools, then overlap the audit with a serial Cargo-check lane."""
    with session(args.output, timings=getattr(args, "timings", False)) as checks:
        if args.audit:
            code = checks.job("tools", "rust.build", [
                "cargo", "build", "--locked", "-p", "ciq-compiler", "-p", "ciq-dis",
                "-p", "ciq-asm", "-p", "ciq-package"])
            if code:
                checks.finish(code)
                return code
        rust = [
            ("workspace-tests", "rust.tests", ["cargo", "test", "--locked", "--workspace"]),
            ("clippy", "rust.lint-format", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"]),
            ("format", "rust.lint-format", ["cargo", "fmt", "--all", "--", "--check"]),
        ]
        if args.rustdoc:
            rust.append(("rustdoc", "rust.docs", ["cargo", "doc", "--locked", "--workspace", "--no-deps"]))
        if args.msrv:
            version = re.search(r'^rust-version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M).group(1)
            if version.count(".") == 1:
                version += ".0"
            rust.append(("msrv", "rust.build", ["cargo", f"+{version}", "check", "--locked", "--workspace", "--all-targets"]))
        lanes = [rust]
        if args.audit:
            argv = [sys.executable, "scripts/audit-examples.py", "--output", str(checks.output / "audit")]
            if args.sdk_index:
                argv += ["--sdk-index", str(args.sdk_index.resolve())]
            lanes.append([("example-audit", "example.audit", argv)])
        code = checks.parallel(lanes)
        checks.finish(code)
        print(f"{'PASS' if code == 0 else 'FAIL'} validation; report: {checks.output / 'report.json'}")
        return code

def command(args):
    """Time an SDK, simulator, or other command with the same checked reporting."""
    argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
    if not argv:
        raise ValueError("command requires an executable after --")
    with session(args.output, timings=getattr(args, "timings", False)) as checks:
        code = checks.job("command", args.category, argv)
        checks.finish(code)
        return code



def wait(output, timeout):
    """Wait for the runner's process exit, without fixed sleeps or polling loops.

    Linux pidfd support and the runner's PID namespace are required. Return 2
    when still running, 1 for an incomplete/crashed run, or the final exit code.
    """
    path = output / "report.json"
    report = json.loads(path.read_text())
    if report["status"] == "running":
        if not hasattr(os, "pidfd_open"):
            raise RuntimeError("wait requires Linux pidfd support; run checks in the foreground")
        try:
            fd = os.pidfd_open(report["pid"])
        except ProcessLookupError:
            fd = None
        if fd is not None:
            try:
                if process_identity(report["pid"]) != report["process_identity"]:
                    raise RuntimeError("runner PID was reused or belongs to another PID namespace")
                if not select.select([fd], [], [], timeout)[0]:
                    print(f"RUNNING; report: {path}")
                    return 2
            finally:
                os.close(fd)
        report = json.loads(path.read_text())
    code = report.get("exit_code", 1) if report["status"] == "complete" else 1
    print(f"{'PASS' if code == 0 else 'FAIL'} validation; report: {path}")
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    phases = parser.add_subparsers(dest="phase", required=True)
    start = phases.add_parser("run", help="run final checks; output must be new")
    start.add_argument("--output", type=Path, required=True)
    start.add_argument("--timings", action="store_true", help="record timing only when profiling is requested")
    start.add_argument("--audit", action="store_true")
    start.add_argument("--sdk-index", type=Path)
    start.add_argument("--rustdoc", action="store_true")
    start.add_argument("--msrv", action="store_true")
    timed = phases.add_parser("command", help="time an individual checked command")
    timed.add_argument("--output", type=Path, required=True)
    timed.add_argument("--timings", action="store_true")
    timed.add_argument("--category", required=True)
    timed.add_argument("argv", nargs=argparse.REMAINDER)
    waiting = phases.add_parser("wait", help="wait in the same Linux container/PID namespace")
    waiting.add_argument("output", type=Path)
    waiting.add_argument("--timeout", type=float, default=45)
    args = parser.parse_args()
    if args.phase == "wait":
        if args.timeout < 0:
            parser.error("timeout must be nonnegative")
        return wait(args.output, args.timeout)
    if args.phase == "command":
        return command(args)
    if args.sdk_index and not args.audit:
        parser.error("--sdk-index requires --audit")
    return run(args)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)
