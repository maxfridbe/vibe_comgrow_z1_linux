# CLI & Safety Features TODO

These features were implemented in the `feature/tabs-and-svg` branch and should be integrated back into the main UI codebase:

## 1. CLI Enhancements
- [ ] **Named Parameters**: Support `--power`, `--speed`, `--scale`, and `--passes` in the `test-pattern` command using `pico-args`.
- [ ] **SVG Support**:
    - [ ] Ability to load `.svg` files from the `assets/` folder via CLI.
    - [ ] Robust path flattening (converting Beziers/Arcs to smooth `G1` segments).
    - [ ] Machine boundary checking (calculating max X/Y before sending G-code).
- [ ] **Interpreted Logs**: Show G-code descriptions in the `SEND` logs (e.g., `M5` -> `Laser Off`).

## 2. Safety & Reliability
- [ ] **SafetyGuard**: Implement a `Drop` guard that always sends `!`, `M5`, `0x18` when the program terminates (handles crashes/normal exits).
- [ ] **Signal Handling**: Catch Ctrl-C (`SIGINT`) to safely power down the laser before exiting.
- [ ] **Auto-Polling**: Send `?` every 500ms automatically to keep position and status updated.
- [ ] **Log Filtering**: Filter out noisy periodic status reports from the serial log unless they contain an Alarm or Hold.

## 3. UI Organization (Requested)
- [ ] Separate UI into Test, Manual, SVG, and Image tabs.
- [ ] Ensure original rounded styles and rich button counts are preserved.
- [ ] Move E-STOP next to the serial log for global access.
- [ ] Implement secondary preview grid for SVG mode.
