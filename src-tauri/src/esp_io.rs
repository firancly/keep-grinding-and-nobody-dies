use std::env;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serialport::SerialPort;

// The ESP32 firmware (keep_grinding_ful_game.ino) is a dumb I/O bridge
// connected over USB serial: it debounces buttons/reads wires and reports
// them, plus drives the physical 7-segment display on command. All game
// logic lives here in Rust.
const BAUD_RATE: u32 = 115_200;
const READ_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonEdge {
    pub button: u8,
    pub kind: EdgeKind,
    pub t_ms: u32,
}

#[derive(Debug, Clone)]
pub struct IoSnapshot {
    pub esp_millis: u32,
    pub buttons_stable: [bool; 4],
    pub wires_cut: [bool; 4],
    pub edges: Vec<ButtonEdge>,
}

#[derive(Debug, Deserialize)]
struct RawEdge {
    button: u8,
    kind: String,
    t: u32,
}

#[derive(Debug, Deserialize)]
struct RawSnapshot {
    #[serde(rename = "espMillis")]
    esp_millis: u32,
    #[serde(rename = "buttonsStable")]
    buttons_stable: [bool; 4],
    #[serde(rename = "wiresCut")]
    wires_cut: [bool; 4],
    edges: Vec<RawEdge>,
}

fn parse_snapshot_line(line: &str) -> Result<IoSnapshot, String> {
    let raw: RawSnapshot =
        serde_json::from_str(line).map_err(|error| format!("invalid JSON ({line:?}): {error}"))?;

    let edges = raw
        .edges
        .into_iter()
        .map(|edge| {
            let kind = match edge.kind.as_str() {
                "press" => Ok(EdgeKind::Press),
                "release" => Ok(EdgeKind::Release),
                other => Err(format!("unknown edge kind from ESP32: {other}")),
            }?;

            Ok(ButtonEdge {
                button: edge.button,
                kind,
                t_ms: edge.t,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(IoSnapshot {
        esp_millis: raw.esp_millis,
        buttons_stable: raw.buttons_stable,
        wires_cut: raw.wires_cut,
        edges,
    })
}

/// Substrings (lowercase) of the USB manufacturer/product strings reported
/// by the handful of USB-serial bridge chips ESP32 dev boards commonly use.
/// Used only to break ties when more than one USB serial device is plugged
/// in at once - a common state at a hardware event (a spare Arduino, a
/// USB-to-serial adapter, etc. sharing the laptop with the actual bomb).
const KNOWN_ESP32_BRIDGE_HINTS: &[&str] = &[
    "cp210", "silicon labs", "ch340", "ch9102", "wch.cn", "wch", "ftdi", "esp32", "espressif",
];

fn looks_like_esp32_bridge(port: &serialport::SerialPortInfo) -> bool {
    let serialport::SerialPortType::UsbPort(info) = &port.port_type else {
        return false;
    };

    [&info.manufacturer, &info.product]
        .into_iter()
        .flatten()
        .any(|text| {
            let text = text.to_lowercase();
            KNOWN_ESP32_BRIDGE_HINTS.iter().any(|hint| text.contains(hint))
        })
}

/// Finds the ESP32's COM port. Prefers the `ESP32_SERIAL_PORT` environment
/// variable if set; otherwise auto-selects when exactly one USB serial
/// device is plugged in. If several are plugged in at once, narrows to
/// those whose USB manufacturer/product string matches a known ESP32
/// USB-serial bridge chip (CP210x, CH340/CH9102, FTDI, ...) and auto-selects
/// if that narrows it down to exactly one - only falling back to an error
/// if it's still ambiguous.
fn find_port() -> Result<String, String> {
    if let Ok(explicit) = env::var("ESP32_SERIAL_PORT") {
        return Ok(explicit);
    }

    let ports = serialport::available_ports()
        .map_err(|error| format!("Failed to list serial ports: {error}"))?;

    let usb_ports: Vec<_> = ports
        .into_iter()
        .filter(|port| matches!(port.port_type, serialport::SerialPortType::UsbPort(_)))
        .collect();

    if let [single] = usb_ports.as_slice() {
        return Ok(single.port_name.clone());
    }

    if usb_ports.is_empty() {
        return Err(
            "No USB serial ports found. Plug in the ESP32, or set the ESP32_SERIAL_PORT \
             environment variable to the correct COM port (check Device Manager)."
                .to_string(),
        );
    }

    let likely: Vec<_> = usb_ports.iter().filter(|port| looks_like_esp32_bridge(port)).collect();
    if let [single] = likely.as_slice() {
        return Ok(single.port_name.clone());
    }

    Err(format!(
        "Multiple USB serial ports found ({}). Set the ESP32_SERIAL_PORT environment \
         variable to pick the right one.",
        usb_ports
            .iter()
            .map(|port| port.port_name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

pub struct EspLink {
    port: Box<dyn SerialPort>,
    writer: Box<dyn SerialPort>,
    port_name: String,
    // Accumulates bytes across calls until a '\n' completes a line - a line
    // can easily span more than one read_latest_snapshot() call.
    line_buffer: Vec<u8>,
}

impl EspLink {
    pub fn open() -> Result<Self, String> {
        let port_name = find_port()?;

        let mut port = serialport::new(&port_name, BAUD_RATE)
            .timeout(READ_TIMEOUT)
            .open()
            .map_err(|error| format!("Failed to open serial port {port_name}: {error}"))?;

        // Many ESP32 boards wire DTR/RTS into an auto-reset circuit on the
        // EN pin. If either line is left asserted after opening, the board
        // can be held in permanent reset - the port opens fine and the
        // firmware simply never boots, producing total silence with no
        // error. Explicitly release both so the chip is free to run.
        if let Err(error) = port.write_data_terminal_ready(false) {
            eprintln!("Warning: failed to release DTR on {port_name}: {error}");
        }
        if let Err(error) = port.write_request_to_send(false) {
            eprintln!("Warning: failed to release RTS on {port_name}: {error}");
        }

        let writer = port
            .try_clone()
            .map_err(|error| format!("Failed to clone serial port {port_name}: {error}"))?;

        Ok(EspLink {
            port,
            writer,
            port_name,
            line_buffer: Vec::new(),
        })
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// Reads raw bytes directly (no BufReader) and assembles complete
    /// lines, merging them into a single snapshot (latest levels win, edges
    /// from every line kept in order). Bounded by an explicit software
    /// deadline rather than trusting the port's configured timeout alone -
    /// on some platforms a buffered reader's larger read-ahead requests can
    /// end up blocking far longer than the configured per-call timeout.
    /// Returns `Ok(None)` if nothing new arrived before the deadline -
    /// that's the normal idle case, not an error.
    pub fn read_latest_snapshot(&mut self) -> Result<Option<IoSnapshot>, String> {
        let deadline = Instant::now() + READ_TIMEOUT;
        let mut merged: Option<IoSnapshot> = None;
        let mut chunk = [0u8; 256];

        loop {
            if Instant::now() >= deadline {
                break;
            }

            match self.port.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    for &byte in &chunk[..n] {
                        if byte == b'\n' {
                            if !self.line_buffer.is_empty() {
                                self.process_line(&mut merged);
                                self.line_buffer.clear();
                            }
                        } else if byte != b'\r' {
                            self.line_buffer.push(byte);
                        }
                    }
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::TimedOut
                        || error.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    break;
                }
                Err(error) => return Err(format!("Serial read error: {error}")),
            }
        }

        Ok(merged)
    }

    /// Parses `self.line_buffer` as one snapshot line and folds it into
    /// `merged`. A garbled/undecodable line (e.g. reset-boundary noise) is
    /// logged and skipped rather than treated as a connection failure.
    fn process_line(&self, merged: &mut Option<IoSnapshot>) {
        let text = String::from_utf8_lossy(&self.line_buffer);
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }

        match parse_snapshot_line(trimmed) {
            Ok(mut snapshot) => {
                if let Some(previous) = merged.take() {
                    let mut edges = previous.edges;
                    edges.extend(snapshot.edges);
                    snapshot.edges = edges;
                }
                *merged = Some(snapshot);
            }
            Err(error) => eprintln!("Failed to parse ESP32 line: {error}"),
        }
    }

    pub fn set_display(&mut self, value: u16) -> Result<(), String> {
        let value = value.min(999);
        writeln!(self.writer, "DISPLAY {value}")
            .map_err(|error| format!("Failed to write to serial port: {error}"))
    }
}
