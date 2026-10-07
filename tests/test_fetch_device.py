"""Offline checks for device selection, package installation, and secret handling."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location("fetch_device", Path(__file__).resolve().parents[1] / "scripts/fetch-device.py")
fetch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fetch)


def package(entries):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, "w") as archive:
        for name, value in entries.items():
            archive.writestr(name, value)
    return stream.getvalue()


class DeviceDownloadTests(unittest.TestCase):
    def test_selects_newest_matching_device_and_hardware_number(self):
        records = [
            {"name": "fr265", "partNumber": "old", "lastUpdateTime": "2025-01-01"},
            {"name": "fr265", "partNumbers": [{"number": "006-B4257-00"}], "lastUpdateTime": "2026-01-01"},
            {"name": "fr265", "partNumber": "unavailable", "ciqInfoFileExists": False, "lastUpdateTime": "2027-01-01"},
        ]
        self.assertEqual(fetch.select_device(records, "fr265")[1], "006-B4257-00")

    def test_traversal_rejected_before_any_extraction(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaises(fetch.DownloadError):
                fetch.extract_zip(package({"normal": "ok", "../escape": "bad"}), root)
            self.assertEqual(list(root.iterdir()), [])

    def test_symlinks_are_rejected(self):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as archive:
            entry = zipfile.ZipInfo("link")
            entry.create_system = 3
            entry.external_attr = (stat.S_IFLNK | 0o777) << 16
            archive.writestr(entry, "../outside")
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(fetch.DownloadError):
                fetch.extract_zip(stream.getvalue(), Path(tmp))

    def test_invalid_package_leaves_no_installed_device(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaises(fetch.DownloadError):
                fetch.install_device(package({"unrelated": "data"}), root, "fr265")
            self.assertFalse((root / "Devices/fr265").exists())

    def test_existing_device_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / "Devices/fr265"
            target.mkdir(parents=True)
            (target / "keep").write_text("existing")
            with self.assertRaises(fetch.DownloadError):
                fetch.install_device(b"invalid zip", root, "fr265")
            self.assertEqual((target / "keep").read_text(), "existing")

    def test_login_download_install_flow_keeps_secrets_out_of_files_and_output(self):
        data = package({"compiler.json": "{}", "simulator.json": "{}"})
        with tempfile.TemporaryDirectory() as tmp:
            responses = [
                json.dumps({"access_token": "test-access-secret"}).encode(),
                json.dumps([{"name": "fr265", "partNumber": "006-B4257-00"}]).encode(),
                data,
            ]
            output = io.StringIO()
            with patch.object(fetch, "browser_ticket", return_value=("test-ticket-secret", "http://localhost:1234/done/test")), \
                 patch.object(fetch, "request", side_effect=responses) as mocked, \
                 patch("sys.argv", ["fetch-device.py", "fr265", "--root", tmp]), \
                 contextlib.redirect_stdout(output):
                fetch.main()
            self.assertIn("/devices/006-B4257-00/ciqInfo", mocked.call_args_list[2].args[0])
            self.assertEqual(mocked.call_args_list[0].kwargs["form"]["service_ticket"], "test-ticket-secret")
            self.assertNotIn("test-access-secret", output.getvalue())
            self.assertNotIn("test-ticket-secret", output.getvalue())
            files = [p.relative_to(tmp).as_posix() for p in Path(tmp).rglob("*") if p.is_file()]
            self.assertEqual(sorted(files), ["Devices/fr265/compiler.json", "Devices/fr265/simulator.json"])


if __name__ == "__main__":
    unittest.main()
