#!/usr/bin/env python3
"""Serve repository Markdown as HTML, or check its local links and anchors."""

import argparse
import html
from html.parser import HTMLParser
from http.server import BaseHTTPRequestHandler, HTTPServer
import mimetypes
from pathlib import Path
import re
import subprocess
from urllib.parse import parse_qs, quote, unquote, urlsplit
from urllib.request import urlopen

from markdown_it import MarkdownIt


ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / "scratch" / "markdown-preview"
CSS_URL = (
    "https://raw.githubusercontent.com/sindresorhus/"
    "github-markdown-css/v5.9.0/github-markdown.css"
)


def repository_files():
    """List existing Git-tracked and non-ignored files inside the repository."""
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
    )
    return {
        path.resolve() for name in output.decode().split("\0") if name
        for path in [ROOT / name]
        if path.is_file() and path.resolve().is_relative_to(ROOT)
    }


def render(source):
    """Render GFM-style HTML with heading IDs for local fragment links."""
    parser = MarkdownIt("gfm-like2", {"html": False})
    tokens = parser.parse(source)
    used = set()
    for index, token in enumerate(tokens):
        if token.type != "heading_open":
            continue
        children = tokens[index + 1].children or []
        title = "".join(
            child.content if child.type in {"text", "code_inline", "image"}
            else " " if child.type in {"softbreak", "hardbreak"} else ""
            for child in children
        )
        base = re.sub(r"[^\w\- ]", "", title.lower()).replace(" ", "-")
        identifier = base
        suffix = 0
        while identifier in used:
            suffix += 1
            identifier = f"{base}-{suffix}"
        used.add(identifier)
        token.attrSet("id", identifier)
    return parser.renderer.render(tokens, parser.options, {})


class Links(HTMLParser):
    """Collect rendered link destinations and heading IDs for verification."""

    def __init__(self, content):
        super().__init__()
        self.destinations = []
        self.ids = set()
        self.feed(content)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag == "a" and "href" in attrs:
            self.destinations.append(attrs["href"])
        elif tag == "img" and "src" in attrs:
            self.destinations.append(attrs["src"])


def check_links(files):
    """Check repository link targets and rendered Markdown heading fragments."""
    pages = {
        path: Links(render(path.read_text()))
        for path in sorted(files) if path.suffix.lower() == ".md"
    }
    problems = []
    checked = 0
    for path, page in pages.items():
        for link in page.destinations:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            checked += 1
            target = (
                ROOT / unquote(url.path).lstrip("/") if url.path.startswith("/")
                else path.parent / unquote(url.path) if url.path else path
            ).resolve()
            if target.is_dir():
                target = target / "README.md"
            if target not in files:
                problems.append((path, link, "missing or ignored file"))
            elif url.fragment and target in pages:
                if unquote(url.fragment) not in pages[target].ids:
                    problems.append((path, link, "missing heading"))
    for path, link, reason in problems:
        print(f"{path.relative_to(ROOT)}: {reason}: {link}")
    print(f"Checked {checked} local links in {len(pages)} Markdown pages; "
          f"{len(problems)} problems.")
    return bool(problems)


def stylesheet():
    """Cache the pinned GitHub Markdown stylesheet for offline preview sessions."""
    path = CACHE / "github-markdown.css"
    if not path.exists():
        CACHE.mkdir(parents=True, exist_ok=True)
        with urlopen(CSS_URL, timeout=30) as response:
            content = response.read()
        path.write_bytes(content)
    return path.read_bytes()


class Preview(BaseHTTPRequestHandler):
    def respond(self, content, content_type):
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(content)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(content)

    def do_GET(self):
        url = urlsplit(self.path)
        if url.path == "/__preview/style.css":
            self.respond(self.server.css, "text/css; charset=utf-8")
            return
        name = unquote(url.path).lstrip("/") or "README.md"
        path = (ROOT / name).resolve()
        if path.is_dir():
            self.send_response(302)
            self.send_header("Location", "/" + quote(name.rstrip("/") + "/README.md"))
            self.end_headers()
            return
        if path not in repository_files():
            self.send_error(404, "File is not in the preview")
            return
        if path.suffix.lower() != ".md" or "raw" in parse_qs(url.query):
            content_type = (
                "text/plain; charset=utf-8" if path.suffix.lower() == ".md"
                else mimetypes.guess_type(str(path))[0] or "text/plain"
            )
            self.respond(path.read_bytes(), content_type)
            return
        relative = path.relative_to(ROOT).as_posix()
        body = render(path.read_text())
        page = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(relative)}</title>
<link rel="stylesheet" href="/__preview/style.css">
<style>
body {{ margin: 0; background: var(--bgColor-default, white); }}
.markdown-body {{ box-sizing: border-box; max-width: 980px;
  margin: auto; padding: 45px; }}
nav {{ margin-bottom: 2rem; font-size: .85rem; }}
@media (max-width: 767px) {{ .markdown-body {{ padding: 15px; }} }}
</style>
</head>
<body><main class="markdown-body">
<nav><a href="/README.md">Project</a> ·
<a href="/docs/README.md">Documentation</a> ·
<a href="/{quote(relative)}?raw=1">Markdown source</a></nav>
{body}
</main></body></html>"""
        self.respond(page.encode(), "text/html; charset=utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--check", action="store_true",
                        help="check local file/heading links, then exit")
    args = parser.parse_args()
    if args.check:
        return check_links(repository_files())
    css = stylesheet()
    with HTTPServer(("127.0.0.1", args.port), Preview) as server:
        server.css = css
        print(f"Markdown preview: http://localhost:{server.server_port}/README.md",
              flush=True)
        print("Refresh after edits; Ctrl+C stops the server.", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
