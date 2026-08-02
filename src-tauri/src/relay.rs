use tauri::{AppHandle, Manager};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::commands::EngineState;
use crate::engine::GameState;

// Lets any device on the LAN (e.g. a tablet browser, which has no Tauri IPC
// bridge) load the standalone defuser page, fetch the latest game state, or
// trigger a restart directly.
const RELAY_ADDR: &str = "0.0.0.0:4000";

// The defuser-facing fullscreen page for the player physically at the bomb.
// The main Tauri window is the expert's view; this is served separately so
// a tablet browser (no Tauri IPC bridge) can poll /state on its own.
const TABLET_PAGE: &str = include_str!("tablet.html");

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
        let (stream, _peer) = match listener.accept().await {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("Relay server accept error: {error}");
                continue;
            }
        };

        let handle = handle.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, handle).await {
                eprintln!("Relay connection error: {error}");
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, handle: AppHandle) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    // Strip any query string (e.g. "/state?t=169..." from the tablet page's
    // cache-busting poll) - otherwise it never matches the exact-string
    // routes below and every poll silently 404s.
    let path = parts.next().unwrap_or("").split('?').next().unwrap_or("");

    let mut stream = reader.into_inner();

    if let Some(index_str) = path.strip_prefix("/jumpscare-video/") {
        return serve_jumpscare_video(&mut stream, index_str).await;
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
        ("POST", "/restart") => {
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

/// Serves the Nth configured jumpscare video file
/// (game_config.toml -> events.jumpscare.video_paths) as raw bytes. Read
/// from disk on every request (not embedded in the binary) so dropping in
/// new video files doesn't need a rebuild.
async fn serve_jumpscare_video(stream: &mut TcpStream, index_str: &str) -> std::io::Result<()> {
    let Ok(index) = index_str.parse::<usize>() else {
        return write_response(stream, 400, "Bad Request", "text/plain", "invalid video index").await;
    };

    let video_path = crate::config::get().events.jumpscare.video_paths.get(index).cloned();

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
            eprintln!("Failed to read jumpscare video {video_path}: {error}");
            write_response(stream, 500, "Internal Server Error", "text/plain", "failed to read video file").await
        }
    }
}

async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    status_text: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         Access-Control-Allow-Origin: *\r\n\
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
         Access-Control-Allow-Origin: *\r\n\
         Cache-Control: no-store\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );

    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}
