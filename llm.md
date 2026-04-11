# Trogdor Laser Runner - Technical Overview

Trogdor is a high-fidelity GRBL runner and engineering tool designed specifically for the Comgrow Z1 Laser engraver. It provides a robust GUI and a safety-first CLI for managing laser operations, including real-time control, pattern generation, image processing, and advanced text engraving.

## Tech Stack
- **Language**: Rust (Edition 2024)
- **GUI Framework**: Raylib (via `raylib` crate)
- **Layout Engine**: Clay (via `clay-layout` crate)
- **Communication**: Serial (via `serialport` crate)
- **Image/SVG Processing**: `image`, `usvg`, `lyon_geom`
- **Typography**: `font-kit` for system font access and outline extraction.

## UI Architecture (Clay + Raylib)

### The Clay Layout Model
Trogdor uses the **Clay** layout engine, which follows a declarative, high-performance model inspired by Flexbox. 
- **Frame-based Rebuild**: The entire UI hierarchy is re-declared every frame within `src/main.rs` and the `ui_*.rs` modules. This allows the UI to be perfectly reactive to the `AppState` without complex change detection.
- **Declarative Syntax**: Elements are defined using builders (`Declaration::new()`) that specify layout properties (width, height, direction, padding, gap) and styling (background color, corner radius).
- **Floating Elements**: Toasts and modal overlays use Clay's `floating()` feature, which allows elements to be positioned relative to the root or other elements without affecting the standard flow. Trogdor specifically uses `attach_to(Root)` and high Z-indexes for these notifications.

### The Renderer
Clay handles the layout math but is platform-agnostic. `src/main.rs` implements the renderer by iterating over Clay's generated `render_commands`:
- **Rectangle**: Mapped to `d.draw_rectangle_rounded` for backgrounds and buttons.
- **Text**: Mapped to `d.draw_text_ex`. Custom logic handles FontAwesome icons and specialized animations like the `ICON_SPINNER`.
- **Image**: Mapped to `d.draw_texture_pro`. This is used both for UI icons and for the complex real-time grid visualizer.
- **Scissor**: Mapped to Raylib's `BeginScissorMode` and `EndScissorMode`, enabling efficient scrollable areas.
- **Custom Canvas**: The central grid visualizer is triggered by a specific element ID ("canvas"). When the renderer encounters this ID, it injects custom Raylib drawing code to render the machine state, real-time paths, and the cached G-code preview texture.

## Project Structure

### Core Logic
- `src/main.rs`: Entry point. Manages the Raylib window, main render loop, and the Clay-to-Raylib render pass.
- `src/comm.rs`: Handles serial communication. Spawns a dedicated thread to manage the G-code command queue and status polling. Includes a `Logger` for session-based burn logging to `~/.trogdor/burnlog/`.
- `src/state.rs`: Defines `AppState`, the central source of truth. Manages tab selection, machine position, command queue, preview paths, and persistence.
- `src/theme.rs`: Defines the `Theme` struct and 20 selectable themes (10 dark, 10 light).
- `src/styles.rs`: Contains fixed color constants and UI style parameters.
- `src/gcode.rs`: Centralized source of truth for G-code strings, constants, and generator functions.
- `src/cli_and_helpers.rs`: Logic for CLI mode, pattern generation, and image-to-G-code rasterization.
- `src/virtual_device.rs`: A virtual GRBL machine emulator for hardware-free testing.

### UI Modules
- `src/ui.rs`: Common UI components (buttons, sliders, checkboxes, toasts).
- `src/ui_manual.rs`: The "Manual" tab. Provides quick commands and jog controls.
- `src/ui_test.rs`: The "Pattern" tab. Handles built-in patterns and custom SVGs.
- `src/ui_image.rs`: The "Image" tab. Raster processing with fidelity and scale controls.
- `src/ui_text.rs`: The "Text" tab. Interactive text engraving with system font support.

## Key Architectures

### State Management
Safe shared state between the render thread and the serial thread using `Arc<Mutex<AppState>>`.

### G-Code Generation & Preview
Generates G-code on the fly, which is then parsed into `PathSegment` vectors. A `preview_version` counter in `AppState` notifies the main loop to re-render the vector data into a 2000x2000 `RenderTexture2D` for performant display.

### Safety & Calibration
- **Homing**: Every burn operation ends with an `$H` sequence.
- **Laser Safety**: Explicit `M5` commands ensure the laser is off during transitions and resets.
- **SafetyGuard**: Automatic emergency sequences sent on application exit or interrupt.

## Operational Modes
- **GUI**: Standard interactive mode.
- **CLI**: Supports command execution via labels, raw G-code, or parameterized test patterns.

## Themes
20 UI themes selectable via `Alt + T`. Themes adapt background, primary accents, and text colors (`cl_text_main`, `cl_text_sub`) for full light/dark mode support.
