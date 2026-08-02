use tauri::{AppHandle, Manager};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::commands::EngineState;
use crate::engine::GameState;

// Lets any device on the LAN (e.g. a tablet browser, which has no Tauri IPC
// bridge) load the standalone defuser page or fetch the latest game state.
// Restarting additionally requires RestartToken (see below) - everything
// else here is intentionally readable by anyone on the LAN, since it's only
// ever non-sensitive bomb-display data.
const RELAY_ADDR: &str = "0.0.0.0:4000";

/// Random per-run secret gating POST /restart (generated in lib.rs at
/// startup). Without this, binding 0.0.0.0 so the tablet can reach the relay
/// also lets any other device on the same LAN restart the game mid-run.
pub struct RestartToken(pub String);

// The defuser-facing fullscreen page for the player physically at the bomb.
// The main Tauri window is the expert's view; this is served separately so
// a tablet browser (no Tauri IPC bridge) can poll /state on its own.
const TABLET_PAGE: &str = include_str!("tablet.html");

// The operator's "break glass" panel (see engine::admin). Served only to
// whoever holds the run's restart token, and never linked from the
// defuser's page - it exists so a live demo can be steered out of trouble.
const ADMIN_PAGE: &str = include_str!("admin.html");

/// Swapped for the run's real token when the panel is served to the
/// laptop itself, and for an empty string for every other device.
const TOKEN_PLACEHOLDER: &str = "__RESTART_TOKEN__";

pub async fn run_relay_server(handle: AppHandle) {
    let listener = match TcpListener::bind(RELAY_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Failed to bind relay server on {RELAY_ADDR}: {error}");
            return;
        }
    };

    println!("Relay server listening on http://{RELAY_ADDR}");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("Relay server accept error: {error}");
                continue;
            }
        };

        let handle = handle.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, handle, peer).await {
                eprintln!("Relay connection error: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    handle: AppHandle,
    peer: std::net::SocketAddr,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_target = parts.next().unwrap_or("");
    let mut target_parts = raw_target.splitn(2, '?');
    // Strip any query string (e.g. "/state?t=169..." from the tablet page's
    // cache-busting poll) - otherwise it never matches the exact-string
    // routes below and every poll silently 404s.
    let path = target_parts.next().unwrap_or("");
    let query = target_parts.next().unwrap_or("");

    let mut stream = reader.into_inner();

    if let Some(index_str) = path.strip_prefix("/jumpscare-video/") {
        let paths = &crate::config::get().events.jumpscare.video_paths;
        return serve_config_video(&mut stream, index_str, paths).await;
    }

    if let Some(index_str) = path.strip_prefix("/stim-video/") {
        let paths = &crate::config::get().stimulation.video_paths;
        return serve_config_video(&mut stream, index_str, paths).await;
    }

    match (method, path) {
        ("GET", "/" | "/player" | "/defuser") => {
            write_response(&mut stream, 200, "OK", "text/html; charset=utf-8", TABLET_PAGE).await
        }
        ("GET", "/state") => {
            let engine_state = handle.state::<EngineState>();
            let view = engine_state.0.lock().unwrap().last_view.clone();
            let body = serde_json::to_string(&view).unwrap_or_else(|_| "null".to_string());
            write_response(&mut stream, 200, "OK", "application/json", &body).await
        }
        // The page itself carries no authority - every control on it POSTs
        // to /admin/action with the token, which is where access is
        // actually enforced. Serving the HTML unauthenticated keeps the
        // "paste the URL with ?token=... on your phone" flow working.
        //
        // As a convenience, the token is baked into the page ONLY for
        // requests coming from the laptop itself (loopback): that's the
        // machine already printing the token to its own terminal, so it
        // reveals nothing new, and it saves the operator retyping 32 hex
        // characters mid-demo. Any other device on the LAN gets the page
        // with an empty field and still has to supply the token.
        ("GET", "/admin") => {
            let token = if peer.ip().is_loopback() {
                handle.state::<RestartToken>().0.clone()
            } else {
                String::new()
            };
            let page = ADMIN_PAGE.replace(TOKEN_PLACEHOLDER, &token);
            write_response(&mut stream, 200, "OK", "text/html; charset=utf-8", &page).await
        }
        ("POST", "/admin/action") => {
            let expected_token = &handle.state::<RestartToken>().0;
            if query_param(query, "token") != Some(expected_token.as_str()) {
                let body = serde_json::json!({ "error": "missing or invalid token" }).to_string();
                return write_response(&mut stream, 401, "Unauthorized", "application/json", &body)
                    .await;
            }

            let action = query_param(query, "action").unwrap_or("");
            let value = query_param(query, "value").unwrap_or("");
            let engine_state = handle.state::<EngineState>();
            let applied = {
                let mut state = engine_state.0.lock().unwrap();
                apply_admin_action(&mut state, action, value)
            };

            let body = serde_json::json!({ "ok": applied, "action": action }).to_string();
            let status = if applied { 200 } else { 400 };
            let status_text = if applied { "OK" } else { "Bad Request" };
            write_response(&mut stream, status, status_text, "application/json", &body).await
        }
        ("POST", "/restart") => {
            let expected_token = &handle.state::<RestartToken>().0;
            let provided_token = query_param(query, "token");

            if provided_token.as_deref() != Some(expected_token.as_str()) {
                let body = serde_json::json!({ "error": "missing or invalid token" }).to_string();
                return write_response(&mut stream, 401, "Unauthorized", "application/json", &body).await;
            }

            let engine_state = handle.state::<EngineState>();
            *engine_state.0.lock().unwrap() = GameState::new();
            let body = serde_json::json!({ "ok": true }).to_string();
            write_response(&mut stream, 200, "OK", "application/json", &body).await
        }
        _ => {
            let body = serde_json::json!({ "error": "not found" }).to_string();
            write_response(&mut stream, 404, "Not Found", "application/json", &body).await
        }
    }
}

/// Serves the Nth video file from a game_config.toml path list (jumpscare
/// or stimulation clips) as raw bytes. Read from disk on every request
/// (not embedded in the binary) so dropping in new video files doesn't
/// need a rebuild.
async fn serve_config_video(
    stream: &mut TcpStream,
    index_str: &str,
    paths: &[String],
) -> std::io::Result<()> {
    let Ok(index) = index_str.parse::<usize>() else {
        return write_response(stream, 400, "Bad Request", "text/plain", "invalid video index").await;
    };

    let video_path = paths.get(index).cloned();

    let Some(video_path) = video_path else {
        return write_response(stream, 404, "Not Found", "text/plain", "no video configured at this index").await;
    };

    match tokio::fs::read(&video_path).await {
        Ok(bytes) => {
            let content_type = match video_path.rsplit('.').next() {
                Some("webm") => "video/webm",
                Some("ogg") | Some("ogv") => "video/ogg",
                _ => "video/mp4",
            };
            write_binary_response(stream, 200, "OK", content_type, &bytes).await
        }
        Err(error) => {
            eprintln!("Failed to read configured video {video_path}: {error}");
            write_response(stream, 500, "Internal Server Error", "text/plain", "failed to read video file").await
        }
    }
}

/// Maps one admin-panel control to its `engine::admin` call. Returns false
/// for an unknown action or an unparseable value so the panel can show the
/// operator that the click didn't land, rather than silently doing nothing.
fn apply_admin_action(state: &mut GameState, action: &str, value: &str) -> bool {
    use crate::engine::admin;
    let now = std::time::Instant::now();

    match action {
        "start" => admin::force_start(state, now),
        "restart" => *state = GameState::new(),
        "pause" => admin::set_paused(state, true),
        "resume" => admin::set_paused(state, false),
        "add_time" => match value.parse::<i64>() {
            Ok(delta_s) => admin::add_time_ms(state, delta_s * 1000),
            Err(_) => return false,
        },
        "set_time" => match value.parse::<i64>() {
            Ok(seconds) => admin::set_time_ms(state, seconds * 1000),
            Err(_) => return false,
        },
        "set_strikes" => match value.parse::<u8>() {
            Ok(strikes) => admin::set_strikes(state, strikes),
            Err(_) => return false,
        },
        "solve_module" => admin::solve_current(state, now),
        "jump_module" => match value.parse::<usize>() {
            Ok(index) => admin::jump_to_module(state, index, now),
            Err(_) => return false,
        },
        "trigger_event" => return admin::trigger_event(state, value, now),
        "clear_event" => admin::clear_event(state),
        "events_on" => admin::set_events_enabled(state, true),
        "events_off" => admin::set_events_enabled(state, false),
        "end_defused" => admin::end_game(state, true),
        "end_exploded" => admin::end_game(state, false),
        _ => return false,
    }

    true
}

/// Extracts a single query-string parameter's raw value (no percent-decoding
/// - the token is a plain hex string, so none is needed).
fn query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == name {
            kv.next()
        } else {
            None
        }
    })
}

#[cfg(test)]
mod admin_action_tests {
    use super::*;
    use crate::engine::Phase;

    fn running_game() -> GameState {
        let mut state = GameState::new();
        assert!(apply_admin_action(&mut state, "start", ""));
        assert_eq!(state.phase, Phase::Running);
        state
    }

    /// The panel auto-fills its token by string-substituting this
    /// placeholder for loopback requests. If it were ever renamed on one
    /// side only, the substitution would silently no-op and the operator
    /// would be back to copying the token by hand.
    #[test]
    fn the_admin_page_carries_the_token_placeholder() {
        assert!(
            ADMIN_PAGE.contains(TOKEN_PLACEHOLDER),
            "admin.html must contain {TOKEN_PLACEHOLDER} for token auto-fill to work"
        );

        let filled = ADMIN_PAGE.replace(TOKEN_PLACEHOLDER, "deadbeef");
        assert!(filled.contains("deadbeef"));
        assert!(
            !filled.contains(TOKEN_PLACEHOLDER),
            "every occurrence should be substituted"
        );
    }

    #[test]
    fn unknown_action_and_bad_values_are_rejected() {
        let mut state = running_game();
        assert!(!apply_admin_action(&mut state, "self_destruct", ""));
        assert!(!apply_admin_action(&mut state, "add_time", "soon"));
        assert!(!apply_admin_action(&mut state, "jump_module", "-1"));
    }

    #[test]
    fn pause_and_resume_toggle_the_flag() {
        let mut state = running_game();
        assert!(apply_admin_action(&mut state, "pause", ""));
        assert!(state.paused);
        assert!(apply_admin_action(&mut state, "resume", ""));
        assert!(!state.paused);
    }

    #[test]
    fn timer_controls_never_detonate_the_bomb() {
        let mut state = running_game();
        apply_admin_action(&mut state, "set_time", "30");
        assert_eq!(state.remaining_ms, 30_000);

        // Subtracting past zero must clamp, not explode.
        apply_admin_action(&mut state, "add_time", "-600");
        assert!(state.remaining_ms > 0);
        assert_eq!(state.phase, Phase::Running);
    }

    #[test]
    fn strikes_clamp_below_the_fatal_count() {
        let mut state = running_game();
        let attempts = crate::config::get().game.attempts;

        apply_admin_action(&mut state, "set_strikes", "1");
        assert_eq!(state.mistakes, 1);

        apply_admin_action(&mut state, "set_strikes", "255");
        assert_eq!(state.mistakes, attempts - 1);
        assert_eq!(state.phase, Phase::Running);
    }

    #[test]
    fn jumping_marks_earlier_modules_solved_and_starts_the_target() {
        let mut state = running_game();
        assert!(apply_admin_action(&mut state, "jump_module", "2"));
        assert_eq!(state.module_index, 2);
        assert!(state.pending.is_none());
    }

    #[test]
    fn event_controls_drive_the_master_switch() {
        let mut state = running_game();

        assert!(apply_admin_action(&mut state, "trigger_event", "Jumpscare"));
        assert!(state.events.active.is_some());

        assert!(apply_admin_action(&mut state, "clear_event", ""));
        assert!(state.events.active.is_none());

        assert!(!apply_admin_action(&mut state, "trigger_event", "NotAnEvent"));

        // The master switch stops the *random* scheduler; a hand-picked
        // event from the panel still fires.
        assert!(apply_admin_action(&mut state, "events_off", ""));
        assert!(!state.events_enabled);
        assert!(apply_admin_action(&mut state, "trigger_event", "SirenLights"));
        assert!(state.events.active.is_some());
    }

    #[test]
    fn force_endings_set_the_terminal_phase() {
        let mut defused = running_game();
        apply_admin_action(&mut defused, "end_defused", "");
        assert_eq!(defused.phase, Phase::Defused);

        let mut exploded = running_game();
        apply_admin_action(&mut exploded, "end_exploded", "");
        assert_eq!(exploded.phase, Phase::Exploded);
    }
}

async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    status_text: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    // No Access-Control-Allow-Origin header: every page that calls this
    // relay (tablet.html) is served BY this same relay, so it's always a
    // same-origin request. A wildcard here would let any website a browser
    // on the LAN happens to have open make cross-origin requests into the
    // relay too - unnecessary and worth not having.
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         Cache-Control: no-store\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {body}",
        body.len()
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await
}

/// Same as `write_response`, but for binary bodies (video files aren't
/// valid UTF-8 text so can't go through the `&str`-based one).
async fn write_binary_response(
    stream: &mut TcpStream,
    status: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         Cache-Control: no-store\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );

    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}
