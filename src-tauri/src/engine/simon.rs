use std::time::{Duration, Instant};

use rand::RngExt;

use super::{register_mistake, solve_current_module, schedule_pending, GameState, PendingAction};
use crate::esp_io::{ButtonEdge, EdgeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimonMode {
    #[default]
    Waiting,
    FlashOn,
    FlashOff,
    Input,
}

#[derive(Debug, Clone, Default)]
pub struct SimonState {
    /// Grows via `push` as stages advance - not a fixed [u8;5] anymore, so
    /// `simon_stages` in game_config.toml can be set above 5 without
    /// overflowing a fixed-size array.
    pub sequence: Vec<u8>,
    pub stage_length: u8,
    pub input_index: u8,
    pub mode: SimonMode,
    pub playback_index: u8,
    pub flash_color: i8,
    pub next_event_at: Option<Instant>,
}

const VOWEL_MAP: [[u8; 4]; 3] = [[1, 0, 3, 2], [3, 2, 1, 0], [2, 0, 3, 1]];
const NO_VOWEL_MAP: [[u8; 4]; 3] = [[1, 3, 2, 0], [0, 1, 3, 2], [3, 2, 1, 0]];

pub fn mapped_button(state: &GameState, flashed_color: u8) -> u8 {
    let row = (state.mistakes as usize).min(2);
    if state.serial_has_vowel {
        VOWEL_MAP[row][flashed_color as usize]
    } else {
        NO_VOWEL_MAP[row][flashed_color as usize]
    }
}

pub fn start(state: &mut GameState, now: Instant) {
    let mut rng = rand::rng();
    state.simon.stage_length = 1;
    state.simon.sequence.clear();
    state.simon.sequence.push(rng.random_range(0..4));
    begin_playback(state, now, 700);
}

fn begin_playback(state: &mut GameState, now: Instant, delay_ms: u64) {
    state.simon.playback_index = 0;
    state.simon.input_index = 0;
    state.simon.flash_color = -1;
    state.simon.mode = SimonMode::Waiting;
    state.simon.next_event_at = Some(now + Duration::from_millis(delay_ms));

    state.status_title = "SIMON SAYS".to_string();
    state.status_detail = "Watch the sequence".to_string();
}

pub fn reset_replay(state: &mut GameState, now: Instant) {
    begin_playback(state, now, 350);
}

pub fn on_tick(state: &mut GameState, now: Instant) {
    if state.simon.mode == SimonMode::Input {
        return;
    }

    let Some(next_event_at) = state.simon.next_event_at else {
        return;
    };
    if now < next_event_at {
        return;
    }

    if matches!(state.simon.mode, SimonMode::Waiting | SimonMode::FlashOff) {
        if state.simon.playback_index >= state.simon.stage_length {
            state.simon.flash_color = -1;
            state.simon.mode = SimonMode::Input;
            state.simon.input_index = 0;
            state.status_detail = "Repeat the sequence".to_string();
            return;
        }

        let color = state.simon.sequence[state.simon.playback_index as usize];
        state.simon.flash_color = color as i8;
        state.simon.mode = SimonMode::FlashOn;
        state.simon.next_event_at = Some(now + Duration::from_millis(480));
        return;
    }

    if state.simon.mode == SimonMode::FlashOn {
        state.simon.flash_color = -1;
        state.simon.playback_index += 1;
        state.simon.mode = SimonMode::FlashOff;
        state.simon.next_event_at = Some(now + Duration::from_millis(280));
    }
}

pub fn on_edge(state: &mut GameState, edge: &ButtonEdge, now: Instant) {
    if edge.kind != EdgeKind::Press || state.simon.mode != SimonMode::Input {
        return;
    }

    let expected = mapped_button(state, state.simon.sequence[state.simon.input_index as usize]);

    if edge.button != expected {
        register_mistake(state, "Wrong Simon button", PendingAction::SimonReplay, now);
        return;
    }

    state.simon.input_index += 1;

    if state.simon.input_index >= state.simon.stage_length {
        if state.simon.stage_length >= crate::config::get().game.simon_stages {
            solve_current_module(state, now);
        } else {
            let next_color = rand::rng().random_range(0..4);
            state.simon.sequence.push(next_color);
            state.simon.stage_length += 1;

            schedule_pending(
                state,
                PendingAction::SimonReplay,
                650,
                "SIMON CORRECT",
                "The sequence gets longer".to_string(),
                now,
            );
        }
    }
}
