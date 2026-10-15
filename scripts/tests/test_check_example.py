"""Guard acceptance verdicts against failures, mismatches, and stale captures."""

import argparse
from contextlib import redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "check-example.py"
SPEC = importlib.util.spec_from_file_location("check_example", SCRIPT)
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class SpecimenChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.home = Path(self.temporary.name)
        self.prepared = self.home / "prepared"
        self.prepared.mkdir()
        (self.prepared / "prepared.json").write_text(json.dumps({
            "example": "probe", "device": "fr265", "runtime_checked": False,
        }))
        (self.prepared / "expected.txt").write_text("42\n")
        self.args = argparse.Namespace(prepared=self.prepared, output=None, observe_seconds=1)

    def capture(self, argv, log, timeout):
        path = Path(argv[argv.index("--output-dir") + 1])
        path.mkdir()
        (path / "monkeydo.log").write_bytes(b"42\r\n")

    def report(self):
        return json.loads((self.prepared / "runtime/report.json").read_text())

    def test_both_programs_must_match_expected_and_use_distinct_capture_directories(self):
        with patch.object(CHECK, "run", side_effect=self.capture) as process, redirect_stdout(io.StringIO()):
            CHECK.simulate(self.args)
        self.assertEqual(process.call_count, 2)
        self.assertTrue(self.report()["runtime_checked"])
        self.assertEqual(set(self.report()["programs"]), {"sdk", "generated"})
        self.assertTrue((self.prepared / "runtime/sdk/monkeydo.log").is_file())
        self.assertTrue((self.prepared / "runtime/generated/monkeydo.log").is_file())

    def test_failed_process_cannot_pass_even_with_matching_console(self):
        def fail(argv, log, timeout):
            self.capture(argv, log, timeout)
            raise RuntimeError("simulator failed")
        with patch.object(CHECK, "run", side_effect=fail):
            with self.assertRaisesRegex(RuntimeError, "simulator failed"):
                CHECK.simulate(self.args)
        self.assertFalse(self.report()["runtime_checked"])

    def test_zero_exit_without_capture_cannot_pass(self):
        with patch.object(CHECK, "run"):
            with self.assertRaises(OSError):
                CHECK.simulate(self.args)
        self.assertFalse(self.report()["runtime_checked"])

    def test_generated_mismatch_preserves_failed_verdict_and_diff(self):
        def mismatch(argv, log, timeout):
            self.capture(argv, log, timeout)
            path = Path(argv[argv.index("--output-dir") + 1])
            if path.name == "generated":
                (path / "monkeydo.log").write_text("43\n")
        with patch.object(CHECK, "run", side_effect=mismatch):
            with self.assertRaisesRegex(RuntimeError, "generated console differs"):
                CHECK.simulate(self.args)
        self.assertFalse(self.report()["runtime_checked"])
        self.assertTrue(self.report()["programs"]["sdk"]["console_matches"])
        self.assertIn("+43", (self.prepared / "runtime/generated-console.diff").read_text())

    def test_existing_capture_directory_is_not_reused(self):
        (self.prepared / "runtime").mkdir()
        with patch.object(CHECK, "run") as process:
            with self.assertRaises(FileExistsError):
                CHECK.simulate(self.args)
        process.assert_not_called()

    def test_stage_failure_preserves_status_and_log(self):
        log = self.home / "failure.log"
        with self.assertRaisesRegex(RuntimeError, "stage exited 9"):
            CHECK.run([sys.executable, "-c", "print('diagnostic'); raise SystemExit(9)"], log)
        self.assertIn("diagnostic", log.read_text())

    def test_unknown_example_is_rejected_before_creating_an_audit(self):
        output = self.home / "audit"
        result = subprocess.run([sys.executable, SCRIPT.with_name("audit-examples.py"),
                                 "--example", "no-such-example", "--output", output],
                                cwd=CHECK.ROOT, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unknown examples", result.stderr)
        self.assertFalse(output.exists())

    def test_empty_expected_text_is_not_execution_evidence(self):
        example = self.home / "examples/probe"
        example.mkdir(parents=True)
        (example / "README.md").write_text("probe")
        expected = example / "expected.txt"
        expected.write_text("")
        sdk = self.home / "sdk.prg"
        sdk.write_bytes(b"test input")
        key = self.home / "key.der"
        key.write_bytes(b"test input")
        args = argparse.Namespace(example="probe", expected=None, sdk_prg=sdk,
                                  key=key, output=self.home / "output")
        with patch.object(CHECK, "ROOT", self.home):
            with self.assertRaisesRegex(RuntimeError, "nonempty expected output"):
                CHECK.prepare(args)
        self.assertFalse(args.output.exists())


if __name__ == "__main__":
    unittest.main()
