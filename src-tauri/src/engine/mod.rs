use std::time::Instant;

use rand::{Rng, RngExt};

use crate::esp_io::{EdgeKind, IoSnapshot};
use crate::view::{self, DefuserView};

pub mod button_module;
pub mod events;
pub mod memory;
pub mod simon;
pub mod wires;

// Tunable game-balance numbers live in game_config.toml (project root) now,
// not here - see crate::config.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Phase {
    Idle,
    Running,
    Defused,
    Exploded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    Memory,
    Simon,
    Button,
    Wires,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    NextModule,
    MemoryReset,
    SimonReplay,
    ButtonRetry,
    WiresReset,
}

pub struct GameState {
    pub phase: Phase,
    pub module_order: [ModuleKind; 4],
    pub module_index: usize,
    pub mistakes: u8,
    pub remaining_ms: i64,
    pub last_tick_at: Option<Instant>,
    pub pending: Option<(PendingAction, Instant)>,
    pub status_title: String,
    pub status_detail: String,

    pub serial: String,
    pub serial_has_vowel: bool,
    pub batteries: u8,
    pub car: bool,
    pub frk: bool,

    pub memory: memory::MemoryState,
    pub simon: simon::SimonState,
    pub button: button_module::ButtonState,
    pub wires: wires::WiresState,
    pub events: events::EventState,

    pub last_view: Option<DefuserView>,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            phase: Phase::Idle,
            module_order: [
                ModuleKind::Memory,
                ModuleKind::Simon,
                ModuleKind::Button,
                ModuleKind::Wires,
            ],
            module_index: 0,
            mistakes: 0,
            remaining_ms: crate::config::get().game.total_time_ms,
            last_tick_at: None,
            pending: None,
            status_title: "READY".to_string(),
            status_detail: "Press Button 1 to start".to_string(),
            serial: String::new(),
            serial_has_vowel: false,
            batteries: 0,
            car: false,
            frk: false,
            memory: memory::MemoryState::default(),
            simon: simon::SimonState::default(),
            button: button_module::ButtonState::default(),
            wires: wires::WiresState::default(),
            events: events::EventState::default(),
            last_view: None,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        GameState::new()
    }
}

/// Advances the game one step given the latest raw ESP32 I/O snapshot.
/// Returns a fresh `DefuserView` only when something observable changed.
pub fn tick(state: &mut GameState, io: &IoSnapshot, now: Instant) -> Option<DefuserView> {
    if state.phase == Phase::Idle {
        if press_occurred(io, 0) {
            start_game(state, now);
        }
        state.last_tick_at = Some(now);
        return finalize(state, io, now);
    }

    if matches!(state.phase, Phase::Defused | Phase::Exploded) {
        return finalize(state, io, now);
    }

    let elapsed_ms = state
        .last_tick_at
        .map(|prev| now.saturating_duration_since(prev).as_millis() as i64)
        .unwrap_or(0);
    state.last_tick_at = Some(now);
    state.remaining_ms -= elapsed_ms;

    if state.remaining_ms <= 0 {
        state.remaining_ms = 0;
        lose_game(state, "Time expired");
        return finalize(state, io, now);
    }

    events::update(state, now);

    if let Some((action, fires_at)) = state.pending {
        if now >= fires_at {
            state.pending = None;
            resolve_pending(state, action, now);
        }
        return finalize(state, io, now);
    }

    if events::blocks_input(&state.events) {
        return finalize(state, io, now);
    }

    let current_module = state.module_order[state.module_index];

    // Discrete input edges, in order, aborting early once a same-tick
    // transition has been scheduled - mirrors the original firmware only
    // ever acting on one triggering condition per loop() iteration.
    for edge in &io.edges {
        if state.pending.is_some() {
            break;
        }

        match current_module {
            ModuleKind::Memory => memory::on_edge(state, edge, now),
            ModuleKind::Simon => simon::on_edge(state, edge, now),
            ModuleKind::Button => button_module::on_edge(state, edge, now),
            ModuleKind::Wires => {}
        }
    }

    // Continuous, time-based checks that must run even without a new edge.
    if state.pending.is_none() {
        match current_module {
            ModuleKind::Memory => {}
            ModuleKind::Simon => simon::on_tick(state, now),
            ModuleKind::Button => button_module::on_tick(state, io),
            ModuleKind::Wires => wires::on_tick(state, io, now),
        }
    }

    finalize(state, io, now)
}

fn finalize(state: &mut GameState, io: &IoSnapshot, now: Instant) -> Option<DefuserView> {
    let view = view::build_view(state, io, now);
    if Some(&view) == state.last_view.as_ref() {
        None
    } else {
        state.last_view = Some(view.clone());
        Some(view)
    }
}

fn press_occurred(io: &IoSnapshot, button: u8) -> bool {
    io.edges
        .iter()
        .any(|edge| edge.button == button && edge.kind == EdgeKind::Press)
}

pub(crate) fn module_name(kind: ModuleKind) -> &'static str {
    match kind {
        ModuleKind::Memory => "MEMORY",
        ModuleKind::Simon => "SIMON SAYS",
        ModuleKind::Button => "THE BUTTON",
        ModuleKind::Wires => "ALIEN WIRES",
    }
}

pub(crate) fn schedule_pending(
    state: &mut GameState,
    action: PendingAction,
    delay_ms: u64,
    title: &str,
    detail: String,
    now: Instant,
) {
    state.pending = Some((action, now + std::time::Duration::from_millis(delay_ms)));
    state.status_title = title.to_string();
    state.status_detail = detail;
}

pub(crate) fn solve_current_module(state: &mut GameState, now: Instant) {
    let title = module_name(state.module_order[state.module_index]).to_string();
    schedule_pending(state, PendingAction::NextModule, 1000, "MODULE SOLVED", title, now);
}

pub(crate) fn register_mistake(
    state: &mut GameState,
    reason: &str,
    retry_action: PendingAction,
    now: Instant,
) {
    state.mistakes += 1;

    if state.mistakes >= 2 {
        lose_game(state, &format!("Second mistake: {reason}"));
        return;
    }

    state.remaining_ms -= crate::config::get().game.first_mistake_penalty_ms;

    if state.remaining_ms <= 0 {
        state.remaining_ms = 0;
        lose_game(state, "Penalty exhausted the timer");
        return;
    }

    schedule_pending(
        state,
        retry_action,
        1500,
        "SECOND CHANCE",
        format!("{reason} | 20 seconds removed"),
        now,
    );
}

fn lose_game(state: &mut GameState, reason: &str) {
    state.phase = Phase::Exploded;
    state.pending = None;
    state.events.active = None;
    state.events.queued = None;
    state.remaining_ms = 0;
    state.status_title = "BOOM".to_string();
    state.status_detail = reason.to_string();
}

fn win_game(state: &mut GameState) {
    state.phase = Phase::Defused;
    state.pending = None;
    state.events.active = None;
    state.events.queued = None;
    state.status_title = "DEFUSED".to_string();
    state.status_detail = "All four modules solved".to_string();
}

fn resolve_pending(state: &mut GameState, action: PendingAction, now: Instant) {
    match action {
        PendingAction::NextModule => {
            state.module_index += 1;
            if state.module_index >= 4 {
                win_game(state);
            } else {
                start_current_module(state, now);
            }
        }
        PendingAction::MemoryReset => memory::reset(state),
        PendingAction::SimonReplay => simon::reset_replay(state, now),
        PendingAction::ButtonRetry => button_module::retry(state),
        PendingAction::WiresReset => wires::reset(state),
    }
}

fn start_current_module(state: &mut GameState, now: Instant) {
    match state.module_order[state.module_index] {
        ModuleKind::Memory => memory::start(state),
        ModuleKind::Simon => simon::start(state, now),
        ModuleKind::Button => button_module::start(state),
        ModuleKind::Wires => wires::start(state),
    }
}

fn start_game(state: &mut GameState, now: Instant) {
    state.mistakes = 0;
    state.module_index = 0;
    state.remaining_ms = crate::config::get().game.total_time_ms;
    state.last_tick_at = Some(now);

    let (serial, has_vowel) = generate_serial();
    state.serial = serial;
    state.serial_has_vowel = has_vowel;

    let mut rng = rand::rng();
    state.batteries = rng.random_range(0..4);
    state.car = rng.random_bool(0.5);
    state.frk = rng.random_bool(0.5);

    state.module_order = shuffle_games();

    state.phase = Phase::Running;
    state.pending = None;
    state.events = events::EventState::new_for_game_start(now);

    start_current_module(state, now);
}

fn generate_serial() -> (String, bool) {
    const VOWELS: &[u8] = b"AEIOU";
    const CONSONANTS: &[u8] = b"BCDFGHJKLMNPQRSTVWXYZ";

    let mut rng = rand::rng();
    let has_vowel = rng.random_bool(0.5);
    let mut chars = [0u8; 6];

    if has_vowel {
        chars[0] = VOWELS[rng.random_range(0..VOWELS.len())];
        chars[1] = CONSONANTS[rng.random_range(0..CONSONANTS.len())];
    } else {
        chars[0] = CONSONANTS[rng.random_range(0..CONSONANTS.len())];
        chars[1] = CONSONANTS[rng.random_range(0..CONSONANTS.len())];
    }

    for slot in chars.iter_mut().skip(2) {
        *slot = b'0' + rng.random_range(0..10);
    }

    (String::from_utf8_lossy(&chars).to_string(), has_vowel)
}

pub(crate) fn shuffle4(rng: &mut impl Rng) -> [u8; 4] {
    let mut values = [0u8, 1, 2, 3];
    for i in (1..4).rev() {
        let j = rng.random_range(0..=i);
        values.swap(i, j);
    }
    values
}

fn shuffle_games() -> [ModuleKind; 4] {
    let mut rng = rand::rng();
    shuffle4(&mut rng).map(|value| match value {
        0 => ModuleKind::Memory,
        1 => ModuleKind::Simon,
        2 => ModuleKind::Button,
        _ => ModuleKind::Wires,
    })
}
