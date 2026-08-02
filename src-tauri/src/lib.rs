mod commands;
mod config;
mod engine;
mod esp_io;
mod relay;
mod view;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand::RngExt;
use tauri::{AppHandle, Emitter, Manager};

use commands::EngineState;
use engine::GameState;
use relay::RestartToken;

/// Generates a random 128-bit hex token gating the relay's POST /restart
/// route - without this, anyone on the same LAN/Wi-Fi as the laptop could
/// restart the game mid-run (the relay has to bind 0.0.0.0 so the tablet
/// can reach it, which also makes it reachable by every other device on
/// the network). Printed to the terminal on startup for the person running
/// the relay to use, e.g. `curl -X POST "http://<ip>:4000/restart?token=..."`.
fn generate_restart_token() -> String {
    let mut rng = rand::rng();
    (0..32)
        .map(|_| {
            let nibble = rng.random_range(0..16u8);
            std::char::from_digit(nibble as u32, 16).unwrap()
        })
        .collect()
}

const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);
const DISPLAY_RESEND_INTERVAL: Duration = Duration::from_millis(1000);
const FAILURE_RECONNECT_THRESHOLD: u32 = 5;
// Matches esp_io::READ_TIMEOUT - used only to turn an idle-tick count into
// an approximate elapsed-seconds figure for the "still no data" nudge.
const READ_TIMEOUT_MS: u64 = 50;
// ESP32 boards can land in a bad state (e.g. bootloader/download mode
// instead of actually running the sketch) after the auto-reset pulse
// triggered by opening the port - this can be timing-sensitive rather than
// deterministic. If we go this long after connecting without ever seeing
// a single valid line, reopening the port gives the reset circuit another
// chance to land in a working state instead of waiting forever.
const NEVER_CONNECTED_RECONNECT_TICKS: u32 = 100; // ~5s at the 50ms read timeout

fn connect_with_retry() -> esp_io::EspLink {
    loop {
        match esp_io::EspLink::open() {
            Ok(link) => {
                println!("Connected to ESP32 on {}", link.port_name());
                return link;
            }
            Err(error) => {
                eprintln!("Waiting for ESP32: {error}");
                std::thread::sleep(RECONNECT_INTERVAL);
            }
        }
    }
}

fn run_engine_loop(handle: AppHandle) {
    let mut link = connect_with_retry();

    let mut last_shown_display: i32 = -1;
    let mut last_display_sent_at: Option<Instant> = None;
    let mut consecutive_failures: u32 = 0;
    let mut received_any_data = false;
    let mut idle_ticks_since_connect: u32 = 0;

    loop {
        match link.read_latest_snapshot() {
            Ok(Some(snapshot)) => {
                if !received_any_data {
                    received_any_data = true;
                    println!("First valid data received from ESP32");
                }
                idle_ticks_since_connect = 0;

                if consecutive_failures > 0 {
                    println!("ESP32 connection restored after {consecutive_failures} failed read(s)");
                    consecutive_failures = 0;
                }

                let now = Instant::now();
                let engine_state = handle.state::<EngineState>();
                let view = {
                    let mut state = engine_state.0.lock().unwrap();
                    engine::tick(&mut state, &snapshot, now)
                };

                if let Some(view) = view {
                    let _ = handle.emit("state:update", &view);

                    // M:SS packed into 3 digits (4:00 -> "400"), matching
                    // the tablet's readout and The Button's digit rule -
                    // NOT raw total seconds.
                    let display_value = engine::timer_display_value(view.timer_ms);
                    let should_resend = display_value as i32 != last_shown_display
                        || last_display_sent_at
                            .map(|at| now.duration_since(at) >= DISPLAY_RESEND_INTERVAL)
                            .unwrap_or(true);

                    if should_resend {
                        last_shown_display = display_value as i32;
                        last_display_sent_at = Some(now);
                        if let Err(error) = link.set_display(display_value) {
                            eprintln!("Failed to update ESP32 display: {error}");
                        }
                    }
                }
            }
            Ok(None) => {
                // Nothing new within the read timeout - normal idle case,
                // unless we've NEVER received a single valid line since
                // connecting, in which case it's worth a periodic nudge and
                // eventually a fresh reconnect attempt.
                if !received_any_data {
                    idle_ticks_since_connect += 1;

                    if idle_ticks_since_connect % 60 == 0 {
                        let seconds = idle_ticks_since_connect as u64 * READ_TIMEOUT_MS / 1000;
                        eprintln!(
                            "Still no data received from ESP32 after {seconds}s - check it's \
                             powered, running the latest firmware, and that COM port is really it"
                        );
                    }

                    if idle_ticks_since_connect >= NEVER_CONNECTED_RECONNECT_TICKS {
                        eprintln!(
                            "No data after {}s - reopening the connection to retry the reset sequence...",
                            NEVER_CONNECTED_RECONNECT_TICKS as u64 * READ_TIMEOUT_MS / 1000
                        );
                        link = connect_with_retry();
                        idle_ticks_since_connect = 0;
                        consecutive_failures = 0;
                    }
                }
            }
            Err(error) => {
                consecutive_failures += 1;
                if consecutive_failures == 1 || consecutive_failures % 20 == 0 {
                    eprintln!(
                        "ESP32 serial error ({consecutive_failures} consecutive failure(s)): {error}"
                    );
                }

                if consecutive_failures >= FAILURE_RECONNECT_THRESHOLD {
                    eprintln!("Reconnecting to ESP32...");
                    link = connect_with_retry();
                    consecutive_failures = 0;
                }
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(EngineState(Mutex::new(GameState::new())));

            let restart_token = generate_restart_token();
            println!(
                "Operator panel:  http://localhost:4000/admin\n  \
                 (opened on this laptop the token fills itself in - no copying needed)\n\
                 From a phone:    http://<laptop-ip>:4000/admin?token={restart_token}\n\
                 Restart token:   {restart_token}\n  \
                 e.g. curl -X POST \"http://<laptop-ip>:4000/restart?token={restart_token}\""
            );
            app.manage(RestartToken(restart_token));

            let engine_handle = app.handle().clone();
            std::thread::spawn(move || run_engine_loop(engine_handle));

            tauri::async_runtime::spawn(relay::run_relay_server(app.handle().clone()));

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_game_state,
            commands::restart_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
