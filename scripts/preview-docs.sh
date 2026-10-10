#!/bin/sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
preview_dir="$repo_dir/scratch/markdown-preview"
requirements="$repo_dir/scripts/markdown-preview-requirements.txt"
python_bin=${PYTHON:-python3}

if [ ! -x "$preview_dir/venv/bin/python" ]; then
    "$python_bin" -m venv "$preview_dir/venv"
fi
if ! cmp -s "$requirements" "$preview_dir/installed-requirements.txt" ||
    ! "$preview_dir/venv/bin/python" -c \
        'from markdown_it import MarkdownIt; import linkify_it; MarkdownIt("gfm-like2")' \
        >/dev/null 2>&1; then
    "$preview_dir/venv/bin/python" -m pip install \
        --disable-pip-version-check -r "$requirements"
    cp "$requirements" "$preview_dir/installed-requirements.txt"
fi

exec "$preview_dir/venv/bin/python" "$repo_dir/scripts/preview-docs.py" "$@"
