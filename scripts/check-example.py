#!/usr/bin/env python3
"""Prepare and validate a console specimen across Rust and SDK environments.

Preparation reuses the example audit for saved stages and structured PRG round
trips, then signs generated output. Simulation compares both programs to a
snapshot of expected console text. Each phase saves logs and reports separately;
preparation alone never establishes runtime acceptance.
"""

import argparse
import difflib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def run(argv, log, timeout=300):
    """Run a checked stage, keeping process diagnostics out of routine output."""
    with log.open("w") as stream:
        try:
            result = subprocess.run(list(map(str, argv)), cwd=ROOT, stdout=stream,
                                    stderr=subprocess.STDOUT, timeout=timeout)
        except (OSError, subprocess.TimeoutExpired) as exc:
            stream.write(f"\n{exc}\n")
            raise RuntimeError(f"stage could not finish; inspect {log}") from exc
    if result.returncode:
        raise RuntimeError(f"stage exited {result.returncode}; inspect {log}")


def prepare(args):
    example = ROOT / "examples" / args.example
    if example.parent != ROOT / "examples" or not (example / "README.md").is_file():
        raise RuntimeError("example must name a directory directly under examples/")
    expected = args.expected or example / "expected.txt"
    for path in [args.sdk_prg, expected, args.key]:
        if not path.is_file():
            raise RuntimeError(f"missing input: {path}")
    if not expected.read_text().strip():
        raise RuntimeError("console acceptance requires nonempty expected output")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    # Snapshot inputs so another SDK build cannot change the runtime baseline.
    shutil.copyfile(args.sdk_prg, output / "sdk.prg")
    shutil.copyfile(expected, output / "expected.txt")
    index = output / "sdk-index.json"
    index.write_text(json.dumps({args.example: str(output / "sdk.prg")}) + "\n")
    run(["cargo", "build", "--locked", "-p", "ciq-compiler", "-p", "ciq-dis",
         "-p", "ciq-asm", "-p", "ciq-package"], output / "build.log")
    run([sys.executable, ROOT / "scripts/audit-examples.py", "--example", args.example,
         "--output", output / "audit", "--sdk-index", index], output / "audit.log")
    row, = json.loads((output / "audit/report.json").read_text())
    programs = [s["prg"]["path"] for s in row["source"] if "prg" in s]
    if len(programs) != 1:
        raise RuntimeError("helper requires exactly one documented complete-PRG command")
    generated = output / "generated.prg"
    run([ROOT / "target/debug/ciq-asm", "sign", ROOT / programs[0], generated,
         "-y", args.key.resolve()], output / "sign.log")
    (output / "prepared.json").write_text(json.dumps({
        "example": args.example, "device": args.device,
        "runtime_checked": False,
    }, indent=2) + "\n")
    print(f"PASS preparation: {args.example}; runtime pending. Artifacts: {output}")


def simulate(args):
    prepared = args.prepared.resolve()
    manifest = json.loads((prepared / "prepared.json").read_text())
    expected = (prepared / "expected.txt").read_text()
    output = (args.output or prepared / "runtime").resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {"example": manifest["example"], "runtime_checked": False, "programs": {}}
    try:
        for name in ["sdk", "generated"]:
            capture = output / name
            run(["xvfb-run", "-a", "-s", "-screen 0 1280x1024x24", sys.executable,
                 ROOT / "scripts/simulator-session.py", "--device", manifest["device"],
                 "--prg", prepared / f"{name}.prg", "--output-dir", capture,
                 "--observe-seconds", args.observe_seconds], output / f"{name}-session.log",
                timeout=180 + args.observe_seconds)
            actual = (capture / "monkeydo.log").read_text()
            matches = actual == expected
            report["programs"][name] = {"console_matches": matches}
            if not matches:
                diff = output / f"{name}-console.diff"
                diff.write_text("".join(difflib.unified_diff(
                    expected.splitlines(keepends=True), actual.splitlines(keepends=True),
                    fromfile="expected", tofile=name)))
                raise RuntimeError(f"{name} console differs; inspect {diff}")
        report["runtime_checked"] = True
    except (RuntimeError, OSError) as exc:
        report["error"] = str(exc)
        raise
    finally:
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"PASS runtime: SDK and generated output match expected text. Artifacts: {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    phases = parser.add_subparsers(dest="phase", required=True)
    build = phases.add_parser("prepare", help="build tools, audit an example, and sign its PRG")
    build.add_argument("example", help="directory name under examples/")
    build.add_argument("--sdk-prg", type=Path, required=True,
                       help="fresh SDK build of the same source/configuration")
    build.add_argument("--output", type=Path, required=True, help="new ignored output directory")
    build.add_argument("--expected", type=Path, help="default: examples/<name>/expected.txt")
    build.add_argument("--key", type=Path, default=ROOT / "keys/developer_key.der")
    build.add_argument("--device", default="fr265")
    runtime = phases.add_parser("simulate", help="run prepared programs in the SDK environment")
    runtime.add_argument("prepared", type=Path)
    runtime.add_argument("--output", type=Path, help="new capture directory; default: <prepared>/runtime")
    runtime.add_argument("--observe-seconds", type=float, default=15)
    args = parser.parse_args()
    if args.phase == "simulate" and args.observe_seconds <= 0:
        parser.error("--observe-seconds must be positive")
    try:
        (prepare if args.phase == "prepare" else simulate)(args)
    except (RuntimeError, OSError, ValueError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
