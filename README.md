# Comgrow Z1 Laser GRBL Runner

## Usage
1.  Connect your Comgrow Z1 via USB.
2.  Ensure you have permissions to access `/dev/ttyUSB0` (usually `sudo usermod -a -G dialout $USER`).
3.  Run the application: `./run.sh` or download the binary from the [Releases](https://github.com/maxfridbe/vibe_comgrow_z1_linux/releases) page.
4.  Use **Ctrl+** and **Ctrl-** to zoom the UI.

## Architecture

### UI (User Interface)
Built using the **Clay Layout** engine with a custom **Raylib** renderer.
- **`src/ui.rs`**: Contains high-level UI components and helper functions for rendering sliders and buttons.
- **`src/main.rs`**: The main application loop and layout definition.
- **`src/icons.rs`**: Centralized Nerd Font icon definitions.
- **`src/state.rs`**: Manages the application state, including movement history (`paths`), virtual position (`v_pos`), and serial logs.

### Communication (Comm)
Direct serial communication with the GRBL firmware.
- **`src/comm.rs`**: Implements a "Ping-Pong" protocol in a background thread. It queues outgoing G-code commands and only sends the next one after receiving an `ok` or `error` response from the machine. This ensures the machine's buffer is never overwhelmed.
- **`src/gcode.rs`**: Contains the decoding logic for interpreting G-code commands and machine responses into human-readable text for the serial log.

## Deployment
Automated releases are handled by a GitHub Action. To trigger a release:
1.  Run `./increment_and_publishrelease.sh`.
2.  The script will increment the version in `version.txt` (format: `1.YYMMDD.Minor`), tag the commit, and push to GitHub.
3.  The GitHub Action will then build the Linux binary and publish it as a new release.
