"""Check concurrency and verdicts without running Cargo or requiring SDK data."""

from contextlib import redirect_stdout
import argparse
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "check_workspace", Path(__file__).resolve().parents[1] / "check-workspace.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class WorkspaceChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name) / "check"

    def test_independent_lanes_reach_each_other_before_either_finishes(self):
        checks = CHECK.Checks(self.output)
        barrier = threading.Barrier(2)
        def job(*_):
            barrier.wait(timeout=3)
            return 0
        with patch.object(checks, "job", side_effect=job):
            self.assertEqual(checks.parallel([[('rust', '', [])], [('audit', '', [])]]), 0)

    def test_failed_tool_build_never_starts_an_audit_with_stale_binaries(self):
        checks = CHECK.Checks(self.output)
        args = argparse.Namespace(output=self.output, audit=True, sdk_index=None,
                                  rustdoc=False, msrv=False)
        with patch.object(CHECK, 'Checks', return_value=checks), \
             patch.object(checks, 'job', return_value=7), \
             patch.object(checks, 'parallel') as parallel:
            self.assertEqual(CHECK.run(args), 7)
        parallel.assert_not_called()
        self.assertEqual(checks.report['status'], 'complete')
        self.assertEqual(checks.report['exit_code'], 7)

    def test_failure_stops_only_its_lane_and_cannot_report_success(self):
        checks = CHECK.Checks(self.output)
        barrier = threading.Barrier(2)
        visited = set()
        def job(name, *_):
            visited.add(name)
            barrier.wait(timeout=3)
            return 7 if name == 'rust' else 0
        with patch.object(checks, "job", side_effect=job):
            code = checks.parallel([[('rust', '', []), ('skipped', '', [])], [('audit', '', [])]])
        checks.finish(code)
        self.assertEqual(visited, {'rust', 'audit'})
        self.assertEqual(code, 1)
        with redirect_stdout(io.StringIO()):
            self.assertEqual(CHECK.wait(self.output, 0), 1)

    def test_actual_process_status_logs_and_timings_are_preserved(self):
        checks = CHECK.Checks(self.output, timings=True)
        with redirect_stdout(io.StringIO()):
            self.assertEqual(checks.job('pass', 'rust.tests', [sys.executable, '-c', 'print("ok")']), 0)
            self.assertEqual(checks.job('fail', 'rust.tests', [sys.executable, '-c', 'raise SystemExit(7)']), 7)
            self.assertEqual(checks.job('missing', 'other', ['/does/not/exist']), 127)
        jobs = json.loads((self.output / 'report.json').read_text())['jobs']
        self.assertEqual([j['status'] for j in jobs], ['passed', 'failed', 'failed'])
        self.assertEqual([j['exit_code'] for j in jobs], [0, 7, 127])
        self.assertTrue(all(j['duration_seconds'] >= 0 and j['end'] >= j['start'] for j in jobs))
        self.assertEqual((self.output / 'pass.log').read_text(), 'ok\n')
        with self.assertRaises(FileExistsError):
            CHECK.Checks(self.output)

    @unittest.skipUnless(hasattr(os, 'pidfd_open'), 'Linux pidfd required')
    def test_wait_timeout_exit_and_missing_final_verdict(self):
        self.output.mkdir()
        path = self.output / 'report.json'
        with subprocess.Popen([sys.executable, '-c', 'import sys; sys.stdin.read()'], stdin=subprocess.PIPE) as child:
            report = {'pid': child.pid, 'process_identity': CHECK.process_identity(child.pid), 'status': 'running'}
            path.write_text(json.dumps(report))
            with redirect_stdout(io.StringIO()):
                self.assertEqual(CHECK.wait(self.output, 0), 2)
            child.communicate()
        with redirect_stdout(io.StringIO()):
            self.assertEqual(CHECK.wait(self.output, 0), 1)
        report.update(status='complete', exit_code=0)
        path.write_text(json.dumps(report))
        with redirect_stdout(io.StringIO()):
            self.assertEqual(CHECK.wait(self.output, 0), 0)

    def test_cancellation_reaps_a_real_child(self):
        checks = CHECK.Checks(self.output)
        process = subprocess.Popen([sys.executable, '-c', 'import sys; sys.stdin.read()'],
                                   stdin=subprocess.PIPE, start_new_session=True)
        self.addCleanup(process.stdin.close)
        checks.processes.add(process)
        checks.cancel()
        self.assertIsNotNone(process.poll())
        self.assertTrue(checks.cancelled.is_set())

    def test_normal_validation_does_not_enable_timing(self):
        checks = CHECK.Checks(self.output)
        with redirect_stdout(io.StringIO()):
            self.assertEqual(checks.job('normal', 'other', [sys.executable, '-c', 'pass']), 0)
        checks.finish(0)
        self.assertFalse(checks.report['timings'])
        self.assertNotIn('duration_seconds', checks.report)
        self.assertNotIn('start', checks.report['jobs'][0])
        self.assertNotIn('duration_seconds', checks.report['jobs'][0])

    def test_command_cli_preserves_failure_and_rejects_reusing_output(self):
        argv = [sys.executable, str(Path(CHECK.__file__)), "command", "--output",
                str(self.output), "--category", "sdk.build", "--",
                sys.executable, "-c", "raise SystemExit(7)"]
        result = subprocess.run(argv, capture_output=True, text=True)
        self.assertEqual(result.returncode, 7, result.stderr)
        report = json.loads((self.output / "report.json").read_text())
        self.assertEqual(report['status'], 'complete')
        self.assertEqual(report['jobs'][0]['category'], 'sdk.build')
        self.assertEqual(report['exit_code'], 7)
        self.assertNotEqual(subprocess.run(argv, capture_output=True).returncode, 0)


if __name__ == '__main__':
    unittest.main()
