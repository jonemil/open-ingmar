#!/bin/sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
image=${CIQ_SIMULATOR_IMAGE:-localhost/ciq-simulator:9.2.0}
interactive=false
if [ "${1:-}" = "--interactive" ]; then
    interactive=true
    shift
fi

if ! command -v podman >/dev/null 2>&1; then
    echo "Run this script on the host, where Podman is installed." >&2
    exit 1
fi
if [ "$interactive" = true ]; then
    case "${DISPLAY:-}" in
        :*|unix:*) ;;
        *) echo "Interactive mode needs a local DISPLAY. Run from a terminal on your desktop (X11 or Xwayland)." >&2; exit 1 ;;
    esac
    display_number=${DISPLAY#unix:}
    display_number=${display_number#:}
    display_number=${display_number%%.*}
    case "$display_number" in
        ''|*[!0-9]*) echo "Invalid local DISPLAY: $DISPLAY" >&2; exit 1 ;;
    esac
    if [ ! -S "/tmp/.X11-unix/X$display_number" ]; then
        echo "No X11 socket for $DISPLAY under /tmp/.X11-unix." >&2
        exit 1
    fi
    if ! command -v xauth >/dev/null 2>&1; then
        echo "Interactive mode requires xauth on the host." >&2
        exit 1
    fi
    auth_dir=$(mktemp -d "${TMPDIR:-/tmp}/ciq-xauth.XXXXXX")
    trap 'rm -f -- "$auth_dir/records" "$auth_dir/wildcard" "$auth_dir/Xauthority"; rmdir -- "$auth_dir"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' HUP TERM
    # Copy only this display's cookie. FamilyWild allows the container hostname.
    xauth nlist "$DISPLAY" > "$auth_dir/records"
    if [ ! -s "$auth_dir/records" ]; then
        echo "No X authority entry for $DISPLAY. Set XAUTHORITY to your desktop session's authority file and retry." >&2
        exit 1
    fi
    sed 's/^..../ffff/' "$auth_dir/records" > "$auth_dir/wildcard"
    touch "$auth_dir/Xauthority"
    xauth -f "$auth_dir/Xauthority" nmerge "$auth_dir/wildcard"
    chmod 600 "$auth_dir/Xauthority"
fi
if ! podman image exists "$image"; then
    echo "Building simulator image: $image" >&2
    podman build -t "$image" -f "$repo_dir/containers/simulator/Dockerfile" "$repo_dir/containers/simulator"
fi
if [ "$interactive" = true ]; then
    echo "Starting interactive simulator on $DISPLAY. Close its window or press Ctrl+C to stop." >&2
    # Desktop socket/cookie retain their host labels. Disable SELinux separation
    # only for this GUI container; do not relabel the host's X11 socket directory.
    podman run --rm --init \
        --userns=keep-id:uid=1000,gid=1000 --user 1000:1000 \
        --cap-drop=ALL --security-opt=no-new-privileges --security-opt=label=disable \
        --env PYTHONUNBUFFERED=1 --env "DISPLAY=${DISPLAY#unix}" \
        --env GDK_BACKEND=x11 --env XAUTHORITY=/tmp/ciq.Xauthority \
        --volume "$repo_dir:/workspace/garmin:rw" \
        --volume /tmp/.X11-unix:/tmp/.X11-unix:ro \
        --volume "$auth_dir/Xauthority:/tmp/ciq.Xauthority:ro" \
        --entrypoint python3 \
        "$image" scripts/simulator-session.py --interactive "$@"
    exit "$?"
fi
echo "Starting simulator container with a virtual display..." >&2
# Keep xvfb-run away from PID 1 so its readiness-signal handshake works.
exec podman run --rm --init \
    --userns=keep-id:uid=1000,gid=1000 --user 1000:1000 \
    --cap-drop=ALL --security-opt=no-new-privileges \
    --env PYTHONUNBUFFERED=1 \
    --volume "$repo_dir:/workspace/garmin:rw,z" \
    "$image" "$@"
