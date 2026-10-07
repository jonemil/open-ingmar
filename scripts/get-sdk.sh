#!/usr/bin/env bash
set -euo pipefail

BASE="https://developer.garmin.com/downloads/connect-iq/sdks"
DEST="${1:-$HOME/garmin/connectiq}"

command -v curl >/dev/null || {
    echo "Missing curl. Install with: sudo dnf install curl"
    exit 1
}

command -v jq >/dev/null || {
    echo "Missing jq. Install with: sudo dnf install jq"
    exit 1
}

mkdir -p "$DEST"
cd "$DEST"

echo "Fetching Garmin Connect IQ SDK manifest..."
curl -fsSL "$BASE/sdks.json" -o sdks.json

echo "Available Linux SDKs:"
jq -r '.[] | select(.linux != null) | .linux' sdks.json

SDK_FILE="$(
    jq -r '.[] | select(.linux != null) | .linux' sdks.json |
    sort -V |
    tail -n1
)"

if [[ -z "$SDK_FILE" || "$SDK_FILE" == "null" ]]; then
    echo "Could not determine Linux SDK archive."
    exit 1
fi

echo
echo "Downloading: $SDK_FILE"
curl -fL --progress-bar \
    "$BASE/$SDK_FILE" \
    -o "$SDK_FILE"

echo
echo "Extracting..."

case "$SDK_FILE" in
    *.zip)
        command -v unzip >/dev/null || {
            echo "Missing unzip. Install with: sudo dnf install unzip"
            exit 1
        }
        unzip "$SDK_FILE"
        ;;

    *.tar.gz|*.tgz)
        tar -xzf "$SDK_FILE"
        ;;

    *.tar.xz)
        tar -xJf "$SDK_FILE"
        ;;

    *)
        echo "Unknown archive format: $SDK_FILE"
        echo "Downloaded successfully, but not extracting automatically."
        exit 0
        ;;
esac

echo
echo "Installed under:"
echo "  $DEST"

echo
echo "Interesting binaries:"
find "$DEST" -type f \
    \( -name monkeyc -o -name monkeydo -o -name simulator \) \
    -print 2>/dev/null

echo
echo "Done."
