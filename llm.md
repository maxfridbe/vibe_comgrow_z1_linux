# Trogdor Laser Runner - Technical Overview

Trogdor is a high-fidelity GRBL runner and engineering tool designed specifically for the Comgrow Z1 Laser engraver. It provides a robust GUI and a safety-first CLI for managing laser operations, including real-time control, pattern generation, image processing, and advanced text engraving.

## Tech Stack
- **Language**: Rust (Edition 2024)
- **GUI Framework**: Raylib (via `raylib` crate)
- **Layout Engine**: Clay (via `clay-layout` crate)
- **Communication**: Serial (via `serialport` crate)
- **Image/SVG Processing**: `image`, `usvg`, `lyon_geom`
- **Typography**: `font-kit` for system font access and outline extraction.

## Project Structure

### Core Logic
- `src/main.rs`: Entry point. Manages the Raylib window, main render loop, input handling (including theme switching via Alt-T), and coordinates between UI modules and the state.
- `src/comm.rs`: Handles serial communication. Spawns a dedicated thread to manage the G-code command queue and status polling. Includes a `Logger` for session-based burn logging to `~/.trogdor/burnlog/`.
- `src/state.rs`: Defines `AppState`, the central source of truth. Manages tab selection, machine position, command queue, preview paths, and persistence (loading/saving states to `~/.config/trogdor/saved_states.json`).
- `src/theme.rs`: Defines the `Theme` struct and 20 selectable themes (10 dark, 10 light).
- `src/styles.rs`: Contains fixed color constants and UI style parameters.
- `src/gcode.rs`: Helper functions for generating standardized G-code commands (jogging, burning, homing, etc.).
- `src/cli_and_helpers.rs`: Contains logic for CLI mode, pattern generation, image-to-G-code conversion, and bounding box calculations.
- `src/virtual_device.rs`: A virtual GRBL machine emulator for testing without physical hardware.
- `src/svg_helper.rs`: Specialized logic for parsing SVG files and converting them to laser-compatible G-code paths.

### UI Modules
- `src/ui.rs`: Common UI components (buttons, sliders, checkboxes, toasts). Implements the floating toast notification system.
- `src/ui_manual.rs`: The "Manual" tab. Provides 40+ quick commands, jog controls, and manual laser firing.
- `src/ui_test.rs`: The "Pattern" tab. Handles built-in test patterns and custom SVG loading with toggleable previews.
- `src/ui_image.rs`: The "Image" tab. Manages image loading, fidelity/scale adjustment, and raster processing.
- `src/ui_text.rs`: The "Text" tab. Features an interactive text input, font selector, and advanced vector/raster engraving controls.
- `src/ui_svg.rs`: (Placeholder/Auxiliary) Specialized SVG path management.
- `src/icons.rs`: FontAwesome icon constants.

## Key Architectures

### State Management
The application uses an `Arc<Mutex<AppState>>` to share state safely between the main render thread and the serial communication thread. UI updates are immediate in the state, and the `comm` thread polls the state/queue for work.

### G-Code Generation & Preview
Operations like Text, Image, and SVG generate G-code strings. These strings are passed to `process_command_for_preview`, which parses them into `PathSegment` vectors for the real-time grid visualizer. The `preview_version` counter triggers texture refreshes in the main loop for high-performance rendering of complex paths.

### Safety & Calibration
- **Homing**: Every burn operation is automatically wrapped in a homing sequence (`$H`).
- **Laser Safety**: Explicit `M5` (Laser Off) commands are issued before homing and on any safety-related reset.
- **SafetyGuard**: On exit or interrupt, the software attempts to send a hard stop sequence to the machine.

## Operational Modes
- **GUI**: Run with no arguments.
- **CLI**: Pass a command label (e.g., `Home`), a raw G-code string, or use the `test-pattern` sub-command for parameter-driven pattern generation.

## Themes
Users can cycle through 20 UI themes using `Alt + T`. Themes adapt both background and text colors (`cl_text_main`, `cl_text_sub`) to ensure legibility in both light and dark modes.
