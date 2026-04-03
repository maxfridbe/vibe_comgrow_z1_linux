#!/bin/bash
set -e

echo "--- Installing System Dependencies for Comgrow Z1 App ---"

# This script installs the necessary development headers for Raylib, SerialPort, and Clay-layout.
# It requires sudo privileges.

sudo apt update
sudo apt install -y \
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
    libclang-dev

echo "--- Dependencies installed successfully! ---"
echo "You can now run ./build.sh to compile the project."
