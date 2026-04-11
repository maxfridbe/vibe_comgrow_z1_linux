# Trogdor Laser GRBL Runner

> **⚠️ WARNING:** Trogdor has only ever been tested on Comgrow Z1 10W Laser and nothing else. Use at your own risk.

![Screenshot Main](./assets/screenshot.png)
![Screenshot Text](./assets/screenshot_text.png)

Trogdor is a high-fidelity GRBL runner and custom engineering tool for the Comgrow Z1 Laser engraver. It features a modern, tabbed user interface built with Rust, Raylib, and the Clay layout engine, offering precise control and advanced processing capabilities.

## Key Features

### 🎨 20 Dynamic UI Themes
- **10 Dark & 10 Light Themes**: Cycle through a wide variety of aesthetics including Slate Night, Cyberpunk, Nordic, Solarized (Light/Dark), Dracula, Gruvbox, and more.
- **On-the-fly Switching**: Use `Alt + T` to cycle themes instantly without restarting the application.
- **Full Compatibility**: All UI components, including logs and interactive elements, adapt perfectly to both light and dark backgrounds.

### 💬 Floating Toast System
- **Real-time Notifications**: Non-intrusive popover messages for important events like file loading, burn completion, and system alerts.
- **Interactive Feedback**: Supports timed auto-dismiss, manual dismissal, and optional action buttons.

### 📜 Session-Based Serial Logging
- **Detailed Comm Logs**: Automatically records all `TX` and `RX` serial traffic during burn operations.
- **Millisecond Precision**: Every log entry includes a high-resolution timestamp (`hh:mm:ss:ms`).
- **Persistence**: Logs are saved to `~/.trogdor/burnlog/[timestamp].log` for post-job analysis.

### ✍️ Interactive Text Tab
- **Live Typing**: Type directly into the UI with a real-time preview and blinking cursor.
- **System Font Access**: Select from any installed system font via a scrollable dropdown menu.
- **Vector & Raster Modes**: High-fidelity engraving using `font-kit` for precise outline extraction.

### 🖼️ Image & SVG Processing
- **Advanced SVG Extraction**: Recursive path extraction supports deeply nested SVG groups and complex geometries.
- **Raster Fidelity**: Adjust Low/High fidelity (White/Black) thresholds and lines-per-mm for perfect photo engraving.
- **Toggleable Previews**: Instant eyeball button (`👁️`) toggles high-performance vector previews on the virtual grid.

### 📏 Engineering Controls
- **Content-Aware Tracing**: Use the `[]` button to trace the exact footprint of your work without firing the laser.
- **Virtual Grid Visualizer**: Real-time rendering of machine and virtual head positions with major/minor grid lines.
- **40+ Quick Commands**: A dedicated Manual tab with categorized GRBL commands, jog controls, and manual laser firing.
- **UI Scaling**: Full interface zooming via `Ctrl` + `+/-` that respects layout boundaries.

### 🛡️ Safety & Calibration
- **Automatic Homing**: Every job automatically begins and ends with an `$H` sequence for perfect calibration.
- **Explicit Laser-Off**: Ensures `M5` is sent before every homing operation and safety reset.
- **SafetyGuard**: Automatically sends `!`, `M5`, `0x18` on normal exit or `Ctrl-C` to ensure the machine is left in a safe state.
- **Emergency Stop**: Massive, always-visible E-STOP button that provides dynamic visual feedback.

## Setup

### 1. Install Dependencies
Run the included installation script to set up required system libraries:
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
./target/debug/trogdor
```

### CLI Mode (Command Labels)
Run any command defined in the UI by its label:
```bash
./target/debug/trogdor Status
./target/debug/trogdor Unlock
./target/debug/trogdor Home
```

### CLI Mode (Raw G-Code)
Send raw G-Code directly to the machine:
```bash
./target/debug/trogdor "G91 G0 X20 Y20"
```

### Test Patterns (CLI)
Execute predefined shapes or assets using named parameters:
```bash
./target/debug/trogdor test-pattern [shape] --power [pct]% --speed [pct]% --scale [scale]x --passes [count]
```
