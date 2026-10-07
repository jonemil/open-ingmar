#!/usr/bin/env python3
"""Browser sign-in and device installation; protocol: docs/specs/device-download.md."""

import argparse
import hashlib
import http.server
import io
import json
from pathlib import Path, PurePosixPath
import re
import secrets
import shutil
import stat
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile

GCS = "https://api.gcs.garmin.com/ciq-product-onboarding"
TOKEN_URL = "https://mobile-auth.security.services.garmin.com/oauth/service_ticket/token"


class DownloadError(Exception):
    pass


class HttpsRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if urllib.parse.urlsplit(newurl).scheme != "https":
            raise DownloadError("Refusing a non-HTTPS API redirect.")
        redirected = super().redirect_request(req, fp, code, msg, headers, newurl)
        if redirected and urllib.parse.urlsplit(req.full_url).netloc != urllib.parse.urlsplit(newurl).netloc:
            redirected.remove_header("Authorization")
        return redirected


def request(url, token=None, form=None):
    headers = {"Accept": "application/json", "User-Agent": "ciq-device-fetch/0.1"}
    if token:
        headers["Authorization"] = "Bearer " + token
    data = None
    if form is not None:
        data = urllib.parse.urlencode(form).encode()
        headers["Content-Type"] = "application/x-www-form-urlencoded"
    req = urllib.request.Request(url, data=data, headers=headers)
    try:
        with urllib.request.build_opener(HttpsRedirect()).open(req, timeout=60) as response:
            return response.read()
    except urllib.error.HTTPError as exc:
        # Never include response bodies, request URLs, or tokens in errors.
        raise DownloadError(f"Garmin request failed with HTTP {exc.code}.") from None
    except urllib.error.URLError:
        raise DownloadError("Could not reach Garmin; check network access and TLS certificates.") from None


def login_url(callback):
    params = {
        "gauthHost": "https://sso.garmin.com/sso",
        "clientId": "ConnectIqSdkManager",
        "id": "gauth-widget",
        "locale": "en",
        "embedWidget": "true",
        "createAccountShown": "true",
        "consumeServiceTicket": "true",
        "service": callback,
        "source": callback,
        "redirectAfterAccountLoginUrl": callback,
        "redirectAfterAccountCreationUrl": callback,
    }
    return "https://sso.garmin.com/sso/signin?" + urllib.parse.urlencode(params)


def browser_ticket(port, timeout):
    callback_path = "/done/" + secrets.token_urlsafe(24)
    received = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def setup(self):
            self.request.settimeout(5)
            super().setup()

        def log_message(self, *args):
            pass  # Callback query parameters contain the service ticket.

        def do_GET(self):
            url = urllib.parse.urlsplit(self.path)
            if url.path == "/login":
                self.send_response(303)
                self.send_header("Location", login_url(callback))
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                return
            tickets = urllib.parse.parse_qs(url.query).get("ticket", [])
            if url.path != callback_path or len(tickets) != 1 or not tickets[0]:
                self.send_error(404, "No valid login callback received")
                return
            received.append(tickets[0])
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Referrer-Policy", "no-referrer")
            self.end_headers()
            self.wfile.write(b"Login callback received. Return to your terminal to check the download result.")

    with http.server.HTTPServer(("127.0.0.1", port), Handler) as server:
        callback = f"http://localhost:{server.server_port}{callback_path}"
        server.timeout = 1
        print(f"Open this URL in a browser on this computer:\nhttp://localhost:{server.server_port}/login", flush=True)
        deadline = time.monotonic() + timeout
        while not received and time.monotonic() < deadline:
            server.handle_request()
        if not received:
            raise DownloadError("Login timed out. Browser and script must share localhost; retry on your desktop.")
    return received[0], callback


def select_device(catalogue, name):
    if not isinstance(catalogue, list):
        raise DownloadError("Unexpected device catalogue schema; expected an array.")
    matches = [item for item in catalogue if item.get("name") == name and item.get("ciqInfoFileExists", True)]
    if not matches:
        raise DownloadError(f"No downloadable device named {name!r} in the catalogue.")
    device = max(matches, key=lambda item: str(item.get("lastUpdateTime", "")))
    number = device.get("partNumber")
    if not number:
        numbers = device.get("partNumbers", [])
        if numbers:
            number = numbers[0].get("number") if isinstance(numbers[0], dict) else numbers[0]
    if not isinstance(number, str) or not number:
        raise DownloadError("Selected device has no recognised hardware part number.")
    return device, number


def extract_zip(data, destination):
    """Reject unsafe archive entries before writing any files."""
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for item in archive.infolist():
            path = PurePosixPath(item.filename)
            if path.is_absolute() or ".." in path.parts or "\\" in item.filename or ":" in item.filename:
                raise DownloadError("Archive contains an unsafe path.")
            mode = item.external_attr >> 16
            if stat.S_ISLNK(mode):
                raise DownloadError("Archive contains a symbolic link.")
        archive.extractall(destination)


def install_device(data, root, name):
    target = root / "Devices" / name
    if target.exists():
        raise DownloadError(f"Device directory already exists: {target}. Existing files were preserved.")
    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".ciq-device-", dir=target.parent) as tmp:
        stage = Path(tmp) / name
        stage.mkdir()
        extract_zip(data, stage)
        if not (stage / "compiler.json").is_file() or not (stage / "simulator.json").is_file():
            raise DownloadError("Device ZIP lacks compiler.json or simulator.json at its root; installation stopped.")
        # Publish a fully extracted package only after validation.
        stage.rename(target)
    return target


def install_fonts(root, device_path, token):
    simulator = json.loads((device_path / "simulator.json").read_text())
    names = {font["filename"] for group in simulator.get("fonts", []) for font in group.get("fonts", [])}
    font_dir = root / "Fonts"
    font_dir.mkdir(parents=True, exist_ok=True)
    for name in sorted(names):
        if not name or Path(name).name != name or "\\" in name or ":" in name:
            raise DownloadError("Unexpected font filename in simulator profile.")
        if " " in name:
            continue  # System font descriptions are not downloadable filenames.
        if (font_dir / name).exists() or (font_dir / (name + ".md5")).exists():
            continue
        print(f"Downloading font {name}...", flush=True)
        data = request(GCS + "/fonts/font?" + urllib.parse.urlencode({"fontName": name}), token)
        with tempfile.TemporaryDirectory(prefix=".ciq-font-", dir=font_dir) as tmp:
            stage = Path(tmp)
            extract_zip(data, stage)
            files = [p for p in stage.rglob("*") if p.is_file()]
            if not files:
                raise DownloadError("Empty font ZIP.")
            for path in files:
                target = font_dir / path.relative_to(stage)
                if target.exists():
                    continue
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, target)
        (font_dir / (name + ".md5")).write_text(hashlib.md5(data, usedforsecurity=False).hexdigest())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("device", nargs="?", default="fr265")
    parser.add_argument("--root", type=Path, default=Path.home() / ".Garmin" / "ConnectIQ",
                        help="Garmin data root (default: ~/.Garmin/ConnectIQ)")
    parser.add_argument("--include-fonts", action="store_true", help="also download simulator fonts")
    parser.add_argument("--port", type=int, default=0, help="localhost callback port (default: automatic)")
    parser.add_argument("--login-timeout", type=int, default=600, help="seconds to wait for browser login")
    args = parser.parse_args()
    if not re.fullmatch(r"[a-zA-Z0-9_-]+", args.device):
        parser.error("device must contain only letters, digits, underscores, or hyphens")
    if not 0 <= args.port <= 65535 or args.login_timeout <= 0:
        parser.error("invalid port or login timeout")
    root = args.root.expanduser().resolve()
    device_path = root / "Devices" / args.device
    if device_path.exists() and not args.include_fonts:
        print(f"Already present: {device_path}")
        return
    ticket, callback = browser_ticket(args.port, args.login_timeout)
    result = json.loads(request(TOKEN_URL, form={
        "grant_type": "service_ticket", "client_id": "CIQ_SDK_MANAGER",
        "service_ticket": ticket, "service_url": callback,
    }))
    token = result.get("access_token")
    if not isinstance(token, str) or not token:
        raise DownloadError("Token exchange returned no access token.")
    if not device_path.exists():
        catalogue = json.loads(request(GCS + "/devices?sdkManagerVersion=1.0.16", token))
        _, number = select_device(catalogue, args.device)
        print(f"Downloading {args.device} (part number {number})...", flush=True)
        data = request(GCS + "/devices/" + urllib.parse.quote(number, safe="") + "/ciqInfo", token)
        device_path = install_device(data, root, args.device)
        print(f"Device ZIP SHA-256: {hashlib.sha256(data).hexdigest()}")
        print(f"Installed device: {device_path}")
    if args.include_fonts:
        install_fonts(root, device_path, token)
        print(f"Simulator fonts installed under {root / 'Fonts'}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nCancelled.", file=sys.stderr)
        sys.exit(130)
    except (DownloadError, OSError, ValueError, zipfile.BadZipFile, KeyError, TypeError) as exc:
        if isinstance(exc, DownloadError):
            print(f"Error: {exc}", file=sys.stderr)
        else:
            print(f"Error: {type(exc).__name__}; download or installation could not be completed.", file=sys.stderr)
        sys.exit(1)
