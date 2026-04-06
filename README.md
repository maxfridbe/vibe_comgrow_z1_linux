# Comgrow Z1 Laser GRBL Runner


![Screenshot Example](./assets/screenshot.png)
A custom engineering tool for the Comgrow Z1 Laser engraver, featuring a high-fidelity tabbed UI and a safety-first CLI.

## Features
- **Tabbed Interface**: 
    - **Manual**: Full 3-column layout with 40+ Quick Commands and Jog controls.
    - **Test**: Direct access to built-in burn patterns and shape testing.
    - **SVG**: Vector path preview and SVG file loading.
- **Docked Engineering Console**: Persistent Serial Log and massive E-STOP button fixed to the bottom of the screen.
- **Virtual Grid**: Real-time persistent visualizer showing machine and virtual head positions.
- **UI Scaling**: Full interface zooming via `Ctrl` + `+/-` that respects layout boundaries.
- **Advanced CLI**: Named parameters, machine boundary checking, and interpreted G-code logs.
- **Safety**: Robust `SafetyGuard` ensures the laser is powered down and machine reset on any exit or crash.

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

### Test Patterns (CLI)
Execute predefined shapes or assets using named parameters:
```bash
./target/debug/comgrow-z1-app test-pattern [shape] --power [pct]% --speed [pct]% --scale [scale]x --passes [count]
```
Example (Low power, full speed, 4x size, 2 passes):
```bash
./target/debug/comgrow-z1-app test-pattern Square --power 1% --speed 100% --scale 4x --passes 2
```
*Note: Supports assets! Try `test-pattern car` if `assets/car.svg` exists.*

## Safety
- **Soft Reset**: Always available in the UI or via `Reset` in CLI.
- **E-STOP on Exit**: Automatically sends `!`, `M5`, `0x18` on normal exit or `Ctrl-C`.
- **Dual E-STOP**: Global E-STOP buttons located in both Header and Footer.
- **Dynamic Power**: Uses `M4` mode to prevent over-burn during speed changes.
