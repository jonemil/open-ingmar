FROM ubuntu:22.04

# Ubuntu 22.04 provides the WebKitGTK 4.0 ABI used by SDK 9.2.0.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates python3 openjdk-17-jre-headless \
        libsecret-1-0 libusb-1.0-0 libudev1 libgtk-3-0 \
        libwebkit2gtk-4.0-37 libjpeg-turbo8 libxxf86vm1 libsm6 \
        xvfb xauth x11-utils imagemagick fonts-dejavu-core \
    && apt-get clean \
    && useradd --create-home --uid 1000 --shell /bin/bash ciq

USER ciq
WORKDIR /workspace/garmin
ENTRYPOINT ["xvfb-run", "-a", "-s", "-screen 0 1280x1024x24", "python3", "scripts/simulator-session.py"]
