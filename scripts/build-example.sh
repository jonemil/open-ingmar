#!/bin/sh
set -eu

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 example-directory [device]" >&2
    exit 1
fi
example_dir=$(CDPATH= cd -- "$1" && pwd)
repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
example_name=$(basename -- "$example_dir")
sdk_dir=${CIQ_SDK_HOME:-"$repo_dir/connectiq"}
key_path=${CIQ_DEVELOPER_KEY:-"$repo_dir/developer_key.der"}
data_dir=${CIQ_DATA_HOME:-"$repo_dir/scratch/garmin-data"}
device=${2:-fr265}
jungle_files=${CIQ_JUNGLE_FILES:-"$example_dir/monkey.jungle"}

# Prefer the user's runtime; the local research runtime is a fallback.
if ! command -v java >/dev/null 2>&1; then
    if [ -n "${JAVA_HOME:-}" ] && [ -x "$JAVA_HOME/bin/java" ]; then
        PATH="$JAVA_HOME/bin:$PATH"
    else
        for java_bin in "$repo_dir"/scratch/tools/java/*/bin; do
            if [ -x "$java_bin/java" ]; then
                PATH="$java_bin:$PATH"
                break
            fi
        done
    fi
    export PATH
fi

if ! command -v java >/dev/null 2>&1; then
    echo "Java is required. Add a compatible Java runtime's bin directory to PATH." >&2
    exit 1
fi
if [ ! -f "$key_path" ]; then
    echo "Signing key not found: $key_path (override with CIQ_DEVELOPER_KEY)." >&2
    exit 1
fi
if [ ! -f "$sdk_dir/bin/monkeybrains.jar" ]; then
    echo "Compiler not found: $sdk_dir/bin/monkeybrains.jar (override with CIQ_SDK_HOME)." >&2
    exit 1
fi
if [ ! -f "$data_dir/Devices/$device/compiler.json" ]; then
    echo "Device profile not found: $data_dir/Devices/$device/compiler.json (override with CIQ_DATA_HOME)." >&2
    exit 1
fi

# Make relative overrides independent of the compiler's working directory.
sdk_dir=$(CDPATH= cd -- "$sdk_dir" && pwd)
data_dir=$(CDPATH= cd -- "$data_dir" && pwd)
key_path=$(CDPATH= cd -- "$(dirname -- "$key_path")" && pwd)/$(basename -- "$key_path")

# The SDK discovers Devices and Fonts under Java's user.home on Linux.
sdk_user_dir="$example_dir/build/sdk-user-home"
mkdir -p "$sdk_user_dir/.Garmin"
if [ -e "$sdk_user_dir/.Garmin/ConnectIQ" ] && [ ! -L "$sdk_user_dir/.Garmin/ConnectIQ" ]; then
    echo "Expected a generated symlink at $sdk_user_dir/.Garmin/ConnectIQ." >&2
    exit 1
fi
ln -sfn "$data_dir" "$sdk_user_dir/.Garmin/ConnectIQ"

mkdir -p "$example_dir/build/$device"
cd "$example_dir"
exec java -Xms1g -Dfile.encoding=UTF-8 -Dapple.awt.UIElement=true \
    "-Duser.home=$sdk_user_dir" \
    -classpath "$sdk_dir/bin/monkeybrains.jar" \
    com.garmin.monkeybrains.Monkeybrains \
    -f "$jungle_files" \
    -d "$device" \
    -o "$example_dir/build/$device/$example_name.prg" \
    -y "$key_path" \
    -O 0
