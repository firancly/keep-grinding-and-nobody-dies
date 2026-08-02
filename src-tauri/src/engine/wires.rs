use std::time::{Duration, Instant};

use rand::RngExt;

use super::{register_mistake, solve_current_module, GameState, PendingAction};
use crate::esp_io::IoSnapshot;

#[derive(Debug, Clone, Default)]
pub struct WiresState {
    pub correct_wire: u8,
    pub clue_wire: u8,
    /// 0 = Alpha, 1 = Beta, 2 = Omega
    pub class: u8,
    pub grammar: u8,
    pub action_set: u8,
    pub noun_set: u8,
    pub ordinal_set: u8,
    pub armed: bool,
    // Timing here uses Rust's own wall clock rather than the ESP32's: the
    // arm-delay (800ms) and cut-confirm debounce (70ms) are coarse
    // reconnect/anti-bounce margins, not gameplay-critical timing like the
    // Button module's hold duration, so poll-cycle-level precision is fine.
    pub all_connected_since: Option<Instant>,
    pub wire_high_since: [Option<Instant>; 4],
    pub wire_triggered: [bool; 4],
}

const ACTION_WORDS: [&str; 4] = ["VRAK", "SHOL", "KREX", "NAAK"];
const NOUN_WORDS: [&str; 4] = ["ZORP", "MIV", "TREL", "BAK"];
const ORDINAL_WORDS: [[&str; 4]; 3] = [
    ["KEL", "DRA", "VON", "SEK"],
    ["PLO", "RIM", "TAK", "WU"],
    ["ESH", "GRA", "NOL", "FEX"],
];
pub const CLASS_NAME: [&str; 3] = ["ALPHA", "BETA", "OMEGA"];

/// Physical wire colors, in the same order as `WIRE_PINS` in
/// keep_grinding_ful_game.ino (wire 1 = GPIO25 = blue, ... wire 4 =
/// GPIO17 = yellow) - i.e. index N here is the same wire as `io.wires_cut[N]`.
pub const WIRE_COLORS: [&str; 4] = ["blue", "white", "black", "yellow"];

pub fn wire_color(index: u8) -> &'static str {
    WIRE_COLORS[index as usize]
}

pub fn action_word(set: u8) -> &'static str {
    ACTION_WORDS[set as usize]
}

pub fn noun_word(set: u8) -> &'static str {
    NOUN_WORDS[set as usize]
}

pub fn ordinal_words(set: u8) -> [&'static str; 4] {
    ORDINAL_WORDS[set as usize]
}

pub fn phrase(wires: &WiresState) -> String {
    let action = ACTION_WORDS[wires.action_set as usize];
    let noun = NOUN_WORDS[wires.noun_set as usize];
    let ordinal = ORDINAL_WORDS[wires.ordinal_set as usize][wires.clue_wire as usize];

    match wires.grammar {
        0 => format!("{action} {ordinal} {noun}"),
        1 => format!("{noun} {ordinal} {action}"),
        _ => format!("{ordinal} {noun} {action}"),
    }
}

pub fn grammar_name(grammar: u8) -> &'static str {
    match grammar {
        0 => "ACTION - INDEX - OBJECT",
        1 => "OBJECT - INDEX - ACTION",
        _ => "INDEX - OBJECT - ACTION",
    }
}

fn generate(wires: &mut WiresState) {
    let mut rng = rand::rng();
    wires.correct_wire = rng.random_range(0..4);
    wires.class = rng.random_range(0..3);

    wires.clue_wire = match wires.class {
        0 => wires.correct_wire,
        1 => (wires.correct_wire + 3) % 4,
        _ => (wires.correct_wire + 2) % 4,
    };

    wires.grammar = rng.random_range(0..3);
    wires.action_set = rng.random_range(0..4);
    wires.noun_set = rng.random_range(0..4);
    wires.ordinal_set = rng.random_range(0..3);
}

pub fn start(state: &mut GameState) {
    generate(&mut state.wires);
    state.wires.armed = false;
    state.wires.all_connected_since = None;
    state.wires.wire_high_since = [None; 4];
    state.wires.wire_triggered = [false; 4];

    state.status_title = "ALIEN WIRES".to_string();
    state.status_detail = "Reconnect every wire to arm".to_string();
}

pub fn reset(state: &mut GameState) {
    start(state);
}

pub fn on_tick(state: &mut GameState, io: &IoSnapshot, now: Instant) {
    if !state.wires.armed {
        let all_connected = io.wires_cut.iter().all(|&cut| !cut);

        if all_connected {
            let since = *state.wires.all_connected_since.get_or_insert(now);
            if now.duration_since(since) >= Duration::from_millis(800) {
                state.wires.armed = true;
                state.status_detail = "Decode the transmission and cut one wire".to_string();
                state.wires.wire_high_since = [None; 4];
                state.wires.wire_triggered = [false; 4];
            }
        } else {
            state.wires.all_connected_since = None;
            state.status_detail = "Reconnect every wire to arm".to_string();
        }

        return;
    }

    let mut newly_cut: Option<u8> = None;
    let mut cut_count = 0u8;

    for i in 0..4usize {
        let cut = io.wires_cut[i];
        let triggered = state.wires.wire_triggered[i];

        if cut && !triggered {
            let since = *state.wires.wire_high_since[i].get_or_insert(now);
            if now.duration_since(since) >= Duration::from_millis(70) {
                state.wires.wire_triggered[i] = true;
                newly_cut = Some(i as u8);
                cut_count += 1;
            }
        } else if !cut && !triggered {
            state.wires.wire_high_since[i] = None;
        }
    }

    if cut_count > 1 {
        register_mistake(state, "Multiple wires were cut", PendingAction::WiresReset, now);
        return;
    }

    if cut_count == 1 {
        if newly_cut == Some(state.wires.correct_wire) {
            solve_current_module(state, now);
        } else {
            register_mistake(state, "Wrong alien wire", PendingAction::WiresReset, now);
        }
    }
}
