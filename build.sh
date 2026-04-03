#!/bin/bash
set -e

# Comgrow Z1 Build Script with local udev workaround

echo "--- Setting up local build environment ---"

# 1. Create local directories for the workaround
mkdir -p .local/lib .local/pkgconfig

# 2. Locate libudev.so.1 and create a development symlink
UDEV_SO="/usr/lib/x86_64-linux-gnu/libudev.so.1"
if [ ! -f "$UDEV_SO" ]; then
    # Search for libudev.so.1 if the standard Debian path fails
    UDEV_SO=$(find /lib /usr/lib -name "libudev.so.1" 2>/dev/null | head -n 1)
fi

if [ -f "$UDEV_SO" ]; then
    ln -sf "$UDEV_SO" ".local/lib/libudev.so"
    echo "Found $UDEV_SO, created local development symlink."
    
    # Create a local .pc file so pkg-config can find it
    printf 'prefix=%s\nexec_prefix=${prefix}\nlibdir=${prefix}/.local/lib\nincludedir=${prefix}/include\n\nName: libudev\nDescription: Library for accessing udev device information\nVersion: 252\nLibs: -L${libdir} -ludev\nCflags: -I${includedir}\n' "$(pwd)" > .local/pkgconfig/libudev.pc
    echo "Generated .local/pkgconfig/libudev.pc"
else
    echo "Warning: libudev.so.1 not found. Serial port support may fail to compile."
fi

# 3. Check for cmake (required by raylib-sys)
if ! command -v cmake &> /dev/null; then
    echo ""
    echo "ERROR: 'cmake' is not installed. It is required to build the raylib dependency."
    echo "Please run: sudo apt update && sudo apt install -y cmake"
    echo ""
    exit 1
fi

# 4. Configure environment variables
export PKG_CONFIG_PATH="$(pwd)/.local/pkgconfig"
export RUSTFLAGS="-L $(pwd)/.local/lib"

# 5. Execute cargo build
echo "--- Starting Cargo Build ---"
cargo build "$@"
