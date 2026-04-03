# Comgrow Z1 Laser GRBL Runner

A custom engineering tool for the Comgrow Z1 Laser engraver, featuring a modern UI and a powerful CLI mode.

## Features
- **Modern UI**: Built with `raylib` and `clay-layout`.
- **SVG Support**: Load and burn `.svg` files from the `assets/` folder.
- **Path Flattening**: Smoothly converts SVG curves and arcs into precise G-code line segments.
- **Live Status**: Real-time position and machine state decoding (Idle, Run, Alarm, etc.).
- **Smart Serial**: Handles buffered responses and priority real-time commands (`!`, `~`, `?`, `0x18`).
- **Test Patterns**: Built-in shapes (Square, Heart, Star) with configurable power, speed, and scale.
- **CLI Mode**: Full control from the terminal with timestamped and interpreted logs.
- **Safety**: Automatic E-STOP sequence sent on program termination or Ctrl-C.

## Setup

### 1. Install Dependencies
Run the included installation script to set up required system headers:
```bash
./install-dependent-packages.sh
```

### 2. Build
Use the build script which handles the local `libudev` workaround:
```bash
./build.sh
```

### 3. Permissions
Ensure you have access to the serial port:
```bash
sudo chmod 666 /dev/ttyUSB0
```

## Usage

### GUI Mode
Run without arguments to launch the graphical interface:
```bash
./target/debug/comgrow-z1-app
```

### CLI Mode (Command Labels)
Run any command defined in the UI by its label:
```bash
./target/debug/comgrow-z1-app Status
./target/debug/comgrow-z1-app Unlock
./target/debug/comgrow-z1-app Home
```

### CLI Mode (Raw G-Code)
Send raw G-Code directly to the machine:
```bash
./target/debug/comgrow-z1-app "G91 G0 X20 Y20"
```

### Test Patterns (Dynamic & SVG)
Execute predefined shapes or SVG files with custom power, speed, scale, and passes:
```bash
./target/debug/comgrow-z1-app test-pattern [Square|Heart|Star|svg_filename] [power%] [speed%] [scale] [passes]
```
Example (SVG car at 40% power, 100% speed, 1x size):
```bash
./target/debug/comgrow-z1-app test-pattern car 40% 100% 1x 1
```
*Note: The tool automatically checks if the scaled shape exceeds the 400mm bed limits.*

## Safety
- **Soft Reset**: Always available in the UI or via `Reset` in CLI.
- **E-STOP**: Priority commands (`!`, `M5`, `0x18`) bypass the queue for immediate response.
- **Auto-Guard**: The `SafetyGuard` ensures the laser is powered down if the app closes.
- **Dynamic Power**: Uses `M4` mode to prevent over-burn during speed changes.
