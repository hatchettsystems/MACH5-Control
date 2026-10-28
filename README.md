# MACH5 Control

Front-end prototype of the MACH5 Management Interface, built from the
*MACH5 Ecosystem: Spec & Design Plan*. Native Rust desktop app (`egui`/`eframe`).
All data is mock data generated in `src/data.rs`; nothing talks to a machine.

```
cargo run --release              # opens on the Dashboard
cargo run --release -- 06 1      # page key (or title), then optional tab indexes
```

Linux needs the usual windowing libraries (X11 or Wayland, `libxkbcommon`, OpenGL).
`Ctrl+B` toggles the left navigation.

## Layout

| Path | Contents |
| --- | --- |
| `src/theme.rs` | Black/grey/white palette; colour is used only for status |
| `src/ui.rs` | Flat panel, tabs, bars, line plots, heat map |
| `src/data.rs` | Mock machine model and simulated telemetry |
| `src/pages/` | One function per page, grouped like the nav |

## Pages

Machine: Dashboard, Control, Print, Thermal. Tuning: Motion, Calibration, Eddy Probe, Drives.
Materials: Filament (MMU, spools, profiles), Camera. Hardware: CAN FD Bus, Nodes, Firmware.
System: Fleet, Config, Rules, API & Security, Simulator, Event Log, Spec & Roadmap.
