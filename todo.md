# Comgrow Z1 Laser GRBL Runner TODO

## 1. CLI Enhancements
- [x] **Named Parameters**: Support `--power`, `--speed`, `--scale`, and `--passes` in the `test-pattern` command using `pico-args`.
- [x] **SVG Support**:
    - [x] Ability to load `.svg` files from the `assets/` folder via CLI.
    - [x] Robust path flattening (converting Beziers/Arcs to smooth `G1` segments).
    - [x] Machine boundary checking (calculating max X/Y before sending G-code).
- [x] **Interpreted Logs**: Show G-code descriptions in the CLI logs.

## 2. Safety & Reliability
- [x] **SafetyGuard**: Implement a `Drop` guard that always sends `!`, `M5`, `0x18` when the program terminates.
- [x] **Signal Handling**: Catch Ctrl-C (`SIGINT`) to safely power down the laser before exiting.
- [ ] **Auto-Polling**: Send `?` every 500ms automatically to keep position and status updated (Partially implemented in serial thread).
- [ ] **Log Filtering**: Filter out noisy periodic status reports from the serial log.

## 3. UI Organization
- [x] Separate UI into Manual, Test, and SVG tabs.
- [x] **UI Styling**: Preserve original rounded styles (16px radii) and rich button sets.
- [x] **Docked Layout**: Fixed bottom bar for Serial Log and E-STOP that persists across tabs.
- [x] **Persistent Center**: Keep Virtual Grid and Position labels visible in all tabs.
- [x] **UI Scaling**: Smooth `Ctrl` + `+/-` zooming that respects docked boundaries.
- [ ] **Image Tab**: Implement raster image processing and burning tab.
- [ ] **SVG Tools**: Implement full file selection dialog and interactive preview.
