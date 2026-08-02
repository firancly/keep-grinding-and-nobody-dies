# Keep Grinding and Nobody Dies (I Guess)

A physical bomb-defusal party game, in the spirit of *Keep Talking and Nobody Explodes*: one player (the **defuser**) is alone at a real hardware "bomb" — buttons, wires, a countdown display — and can only see what's in front of them. Everyone else (the **expert**) has a rulebook and has to talk the defuser through defusing it before the timer runs out, without ever seeing the bomb themselves.

- **Hardware**: an ESP32 reads the 4 buttons and 4 wires and drives the physical countdown display, over a USB cable into a laptop.
- **Game logic**: all of it — the countdown, mistakes, and all 4 puzzle modules (Memory, Simon Says, The Button, Alien Wires) — runs in a Rust/Tauri backend on the laptop.
- **Two screens**: the laptop's own window is the expert's rulebook view; a separate fullscreen page, served over the LAN from the laptop, is the defuser's view (meant for a tablet held at the bomb).
- Sabotage events (screen glitches, jumpscares, etc.) fire randomly during a run to keep things chaotic.

## Setup and running

See [guide.md](guide.md) for the full step-by-step: one-time computer setup, flashing the ESP32, running the app, connecting the tablet, and tuning the game's difficulty via `game_config.toml`.

## Project layout

- `src/` — the expert's React app (Tauri window).
- `src-tauri/src/` — the Rust backend: game engine (`engine/`), ESP32 serial I/O (`esp_io.rs`), the LAN relay serving the defuser's page (`relay.rs`, `tablet.html`), and config loading (`config.rs`).
- `keep_grinding_ful_game.ino` — ESP32 firmware. It's a dumb I/O bridge only (debounces buttons, reads wires, drives the display) — no game logic lives on the device.
- `game_config.toml` — all game-balance numbers (timer length, mistake penalty, event frequency, etc.), documented inline.

## Tech stack

[Tauri](https://tauri.app/) (Rust backend + native window) with a [React](https://react.dev/) + [Vite](https://vite.dev/) frontend.
