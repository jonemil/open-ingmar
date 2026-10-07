#!/bin/sh
set -eu
example_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$example_dir/../../scripts/build-example.sh" "$example_dir" "$@"
