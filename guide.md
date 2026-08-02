# Keep Grinding and Nobody Dies — Run Guide

This is the plain "how do I actually run this thing" guide. Not a technical deep-dive — just the steps.

## 1. One-time computer setup

You need these installed once:

- **Node.js** (includes npm)
- **pnpm** — after Node is installed: `npm install -g pnpm`
- **Rust** (rustup + cargo)
- **Windows C++ Build Tools** — Visual Studio Build Tools with the "Desktop development with C++" workload (Rust needs this to link on Windows)
- **Arduino IDE** + ESP32 board support — only needed if you're re-flashing the firmware

After installing anything new, **close and reopen your terminal** — an already-open terminal window won't pick up new PATH entries.

Install project dependencies (once, and again any time `package.json` changes):

```
pnpm install
```

## 2. Running the app day-to-day

```
pnpm tauri dev
```

This compiles the Rust backend, starts the dev server, and opens the desktop app window automatically. Leave the terminal running — closing it stops the app.

## 3. The ESP32 (the physical bomb)

The firmware lives in `keep_grinding_ful_game.ino`. It doesn't run the game itself anymore — it just reads the 4 buttons and 4 wires and drives the display, over a **USB cable straight into the laptop**. All actual game logic runs on your laptop in Rust.

**Wiring** (for reference):
- Buttons 1–4 → GPIO 14, 27, 26, 33
- Wires 1–4 → GPIO 25, 13, 16, 17
- Display shift register → DIN 32, CLK 18, LOAD 19

**Flashing it:** open `keep_grinding_ful_game.ino` in Arduino IDE, select your ESP32 board/port, hit Upload.

**Connecting to it:** just plug the ESP32 into the laptop with a USB cable. That's it — no WiFi network to join, and your laptop stays on its normal Wi-Fi/internet the entire time.

The Rust backend automatically finds the right COM port as long as it's the only USB-serial device plugged in. If you ever have more than one plugged in at once (or auto-detect fails for some reason), set the port explicitly before running:

- PowerShell: `$env:ESP32_SERIAL_PORT = "COM5"` (check Device Manager → Ports (COM & LPT) for the real number)
- cmd.exe: `set ESP32_SERIAL_PORT=COM5`

then run `pnpm tauri dev` in that same terminal.

## 4. Starting a game

Once the laptop app is running and the ESP32 is plugged in over USB, press **physical Button 1** on the bomb to start. The countdown, mistakes, and all 4 modules (Memory, Simon, Button, Wires) are handled by the Rust engine.

## 5. Two screens: expert (laptop) and defuser (tablet)

There are two roles, two screens:

- **The desktop app window** (`pnpm tauri dev`'s native window) is the **expert's** screen.
- **The tablet**, held by the player physically at the bomb (the **defuser**), gets its own fullscreen page served straight from the laptop — no app install needed, just a browser.

The app runs a small extra server on **port 4000** on your laptop that any device on the same network can load — since the ESP32 connection is over USB (not WiFi), your laptop stays on its normal Wi-Fi the whole time this is happening, no network switching needed.

1. Find your laptop's IP address:
   ```
   ipconfig
   ```
   Look for "IPv4 Address" under your active network adapter (Wi-Fi or Ethernet). It'll look like `192.168.x.x`.

2. Make sure the tablet is on the **same Wi-Fi network** as the laptop.

3. On the tablet's browser, go to:
   ```
   http://<laptop-ip>:4000/
   ```
   This loads the defuser's fullscreen bomb view (timer, strikes, badges, all 4 modules, sabotage-event effects) — the same live game the expert sees on the laptop, styled for the player at the bomb. Tap **"TAP TO BEGIN"** once to enter fullscreen (browsers require a tap before allowing fullscreen, so this can't happen fully automatically) and it starts polling immediately.

4. Raw JSON is still available directly at `http://<laptop-ip>:4000/state`, if you ever need to poll it from something other than a browser.

5. To trigger a restart from any device, send a POST request to:
   ```
   http://<laptop-ip>:4000/restart
   ```
   (from a terminal: `curl -X POST http://<laptop-ip>:4000/restart`)

### One-time firewall step for port 4000

Windows Firewall blocks incoming connections by default. Run this **once**, in an **Administrator** PowerShell window:

```powershell
New-NetFirewallRule -DisplayName "Keep Grinding Relay 4000" -Direction Inbound -LocalPort 4000 -Protocol TCP -Action Allow
```

## 6. (Rarely needed) Loading the expert's React app itself on a second screen

Section 5's port-4000 page is what the tablet/defuser should normally use. This section is only for the unusual case where you want to load the **expert's own React app** (the same thing running in the desktop window) on a second screen during development:

1. Before running dev, set an environment variable to your laptop's IP:
   - PowerShell: `$env:TAURI_DEV_HOST = "<laptop-ip>"`
   - cmd.exe: `set TAURI_DEV_HOST=<laptop-ip>`
2. Then run `pnpm tauri dev` as normal in that same terminal.
3. On the second screen, open `http://<laptop-ip>:1420`.

**Heads up:** this only works for *display* — a browser has no way to run Tauri commands directly (that only works inside the native desktop window), so anything relying on `invoke(...)` won't work there. For the tablet/defuser, use the port-4000 page from section 5 instead — it's built for exactly this and doesn't have that limitation.

One-time firewall rule for this (same idea as above, different port):

```powershell
New-NetFirewallRule -DisplayName "Vite Dev 1420" -Direction Inbound -LocalPort 1420 -Protocol TCP -Action Allow
```

## 7. Tuning the game (`game_config.toml`)

All the "how hard is this game" numbers live in **`game_config.toml`** at the project root — no code editing needed. Open it in any text editor, change a value, save, and **restart `pnpm tauri dev`** (config is only read once at startup, not live-reloaded).

- `[game]` — `total_time_ms` (bomb timer), `first_mistake_penalty_ms` (time lost on the 1st mistake), `hold_threshold_ms` (tap vs. hold cutoff for The Button), `simon_stages` (how long Simon Says grows).
- `[events]` — `interval_ms` controls how often a random sabotage event is attempted (lower = more chaotic). `[events.duration_ms]` has a per-event duration in milliseconds (Dyslexia, UpsideDown, BlueScreen, TurkAttack, Jumpscare, MirrorMode, StaticGlitch, SirenLights).
- `[events.jumpscare]` → `video_paths` — up to 3 file paths to video files. When the Jumpscare event fires, one is picked at random and played fullscreen with sound on both screens. Leave the list empty (`video_paths = []`) to keep the built-in CSS monster-face + synthesized scream instead — nothing breaks either way.

Every setting in the file has a comment above it explaining what it does. If the file is missing, deleted, or has a typo that breaks TOML parsing, the app doesn't crash — it just falls back to built-in defaults and prints a note to the terminal.

## 8. Troubleshooting

- **"pnpm"/"npm"/"cargo" not recognized"** — you just installed something and this terminal is stale. Close it, open a brand new terminal window, try again.
- **`pnpm tauri dev` hangs on "Waiting for your frontend dev server..."** — usually means `TAURI_DEV_HOST` is set but Vite isn't reachable at `localhost` anymore. This is already handled in `vite.config.ts` (it binds to all network interfaces whenever `TAURI_DEV_HOST` is set), so this shouldn't happen — but if it does, double-check the IP you set is correct and try unsetting `TAURI_DEV_HOST` and restarting normally.
- **`ERR_SSL_PROTOCOL_ERROR` on the tablet** — the browser tried `https://` instead of `http://`. Type `http://` explicitly in the address bar, and turn off "Always use secure connections" in the browser's security settings if it keeps happening.
- **Tablet can't reach the laptop at all** — check both devices are on the same Wi-Fi network, re-check the laptop's IP with `ipconfig` (it can change between sessions), and confirm the firewall rules from sections 5/6 are in place.
- **Nothing happens when pressing Button 1** — confirm the ESP32 is plugged in over USB and `pnpm tauri dev` is actually running (check the terminal for "Connected to ESP32 on COM...").
- **Terminal repeats `Waiting for ESP32: ...` and never connects** — usually means either the USB cable isn't plugged in / the board isn't powered, the firmware isn't flashed, or auto-detect picked the wrong COM port (or found none/multiple). Check Device Manager → Ports (COM & LPT) for the ESP32's port name, and set it explicitly with `ESP32_SERIAL_PORT` (see section 3) if needed.
- **Terminal repeats `ESP32 serial error (...): ...`, then "Reconnecting to ESP32..."** — the cable got unplugged, the board reset, or Windows momentarily dropped the COM port (common after a USB power-saving suspend). It auto-reconnects on its own every ~2 seconds once the port is available again — no restart needed. If it keeps happening, check Device Manager → the ESP32's USB-serial device → Properties → Power Management → uncheck "Allow the computer to turn off this device to save power".
- **"Multiple USB serial ports found" error on startup** — you have more than one USB-serial device plugged in (another Arduino, a USB-to-serial adapter, etc). Set `ESP32_SERIAL_PORT` explicitly to the right one (section 3).
