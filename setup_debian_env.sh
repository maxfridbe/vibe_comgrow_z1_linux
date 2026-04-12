#!/bin/bash
set -e

echo "--- Setting up Debian Environment for Comgrow Z1 ---"

# 1. Update and install system packages
echo "Installing system dependencies..."
sudo apt update
sudo apt install -y \
    git \
    curl \
    build-essential \
    cmake \
    libudev-dev \
    libx11-dev \
    libxrandr-dev \
    libxinerama-dev \
    libxcursor-dev \
    libxi-dev \
    libgl1-mesa-dev \
    libasound2-dev \
    pkg-config \
    libclang-dev \
    libfontconfig1-dev \
    libfreetype6-dev \
    libxkbcommon-dev \
    libwayland-dev \
    libwayland-egl1 \
    libvulkan1 \
    mesa-vulkan-drivers \
    libegl1-mesa-dev \
    libwayland-cursor0 \
    libxkbcommon-dev \
    libvte-2.91-dev \
    librsvg2-dev \
    libcairo2-dev \
    libdbus-1-dev

# 2. Install Rust (if not already installed)
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed."
    rustup update
fi

# 3. Ensure we have the right toolchain components
rustup component add rustfmt clippy

# 4. Run the local build setup (from build.sh logic)
echo "Configuring local build environment (udev workarounds)..."
mkdir -p .local/lib .local/pkgconfig

UDEV_SO="/usr/lib/x86_64-linux-gnu/libudev.so.1"
if [ ! -f "$UDEV_SO" ]; then
    UDEV_SO=$(find /lib /usr/lib -name "libudev.so.1" 2>/dev/null | head -n 1)
fi

if [ -f "$UDEV_SO" ]; then
    ln -sf "$UDEV_SO" "$(pwd)/.local/lib/libudev.so"
    echo "Found $UDEV_SO, created local development symlink."
    
    printf 'prefix=%s\nexec_prefix=${prefix}\nlibdir=${prefix}/.local/lib\nincludedir=${prefix}/include\n\nName: libudev\nDescription: Library for accessing udev device information\nVersion: 252\nLibs: -L${libdir} -ludev\nCflags: -I${includedir}\n' "$(pwd)" > .local/pkgconfig/libudev.pc
    echo "Generated .local/pkgconfig/libudev.pc"
else
    echo "Warning: libudev.so.1 not found. Serial port support may fail to compile."
fi

echo "--- Debian Environment Setup Complete! ---"
echo "You may need to restart your shell or run 'source \$HOME/.cargo/env' to use 'cargo'."
echo "To build the project, run: ./build.sh"
