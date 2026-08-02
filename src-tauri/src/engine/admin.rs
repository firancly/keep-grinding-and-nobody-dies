//! Operator ("break glass") controls for running the game during a live
//! demo: pause, retime, restrike, skip to a module, force events.
//!
//! Nothing here is reachable by the players - the relay only exposes these
//! behind the same random per-run token that gates POST /restart, and the
//! defuser's page never calls them. They deliberately reuse the normal
//! engine entry points (`start_current_module`, `solve_current_module`,
//! `start_event`) so an admin-driven game stays in exactly the same shapes
//! a naturally-played one does.

use std::time::{Duration, Instant};

use super::events::{self, SabotageEvent};
use super::{GameState, Phase};

/// While paused, wall-clock keeps advancing but the game must not - so
/// every stored deadline gets pushed forward by the same amount, otherwise
/// everything that was mid-flight (a pending module transition, an active
/// sabotage event, Simon's next flash) would fire the instant we resume.
///
/// The Button's hold timing is deliberately NOT shifted: it's measured in
/// the ESP32's own millis(), and a hold spanning a pause is ambiguous
/// enough that letting it resolve normally is the least surprising choice.
pub fn shift_deadlines(state: &mut GameState, by: Duration) {
    if let Some((_, fires_at)) = state.pending.as_mut() {
        *fires_at += by;
    }
    if let Some(until) = state.events.active_until.as_mut() {
        *until += by;
    }
    if let Some(next) = state.events.next_random_at.as_mut() {
        *next += by;
    }
    if let Some(next) = state.simon.next_event_at.as_mut() {
        *next += by;
    }
    if let Some(since) = state.wires.all_connected_since.as_mut() {
        *since += by;
    }
    for slot in state.wires.wire_high_since.iter_mut().flatten() {
        *slot += by;
    }
}

pub fn set_paused(state: &mut GameState, paused: bool) {
    if state.paused == paused {
        return;
    }
    state.paused = paused;

    if paused {
        state.status_title = "PAUSED".to_string();
        state.status_detail = "Operator paused the game".to_string();
    } else {
        state.status_title = "RESUMED".to_string();
        state.status_detail = "Back to it".to_string();
    }
}

/// Starts a game immediately without waiting for the physical Button 1 -
/// the single most useful demo control when the bomb isn't cooperating.
pub fn force_start(state: &mut GameState, now: Instant) {
    if state.phase != Phase::Running {
        super::start_game(state, now);
    }
}

pub fn add_time_ms(state: &mut GameState, delta_ms: i64) {
    if state.phase != Phase::Running {
        return;
    }
    // Never let an operator nudge push the timer to 0 and detonate the
    // bomb as a side effect - clamp at one second.
    state.remaining_ms = (state.remaining_ms + delta_ms).max(1_000);
}

pub fn set_time_ms(state: &mut GameState, value_ms: i64) {
    if state.phase != Phase::Running {
        return;
    }
    state.remaining_ms = value_ms.max(1_000);
}

pub fn set_strikes(state: &mut GameState, strikes: u8) {
    let attempts = crate::config::get().game.attempts;
    // Clamped one below `attempts`: setting the fatal count from the panel
    // would explode the bomb on the next tick, which is never what an
    // operator reaching for this control wants.
    state.mistakes = strikes.min(attempts.saturating_sub(1));
}

/// Marks the current module solved through the normal transition (1s
/// "MODULE SOLVED" beat, then the next module starts / the game is won).
pub fn solve_current(state: &mut GameState, now: Instant) {
    if state.phase != Phase::Running {
        return;
    }
    state.pending = None;
    super::solve_current_module(state, now);
}

/// Jumps straight to the Nth module in the current run's order. Modules
/// before it read as solved; the target one is started fresh.
pub fn jump_to_module(state: &mut GameState, index: usize, now: Instant) {
    if state.phase != Phase::Running || index >= state.module_order.len() {
        return;
    }
    state.pending = None;
    state.module_index = index;
    super::start_current_module(state, now);
}

/// Fires one named event immediately. Returns false if the name isn't a
/// real event or there's no running game, so a mistyped control reports
/// failure instead of silently doing nothing.
///
/// Deliberately ignores `events_enabled`: that switch governs the *random*
/// event scheduler, and an operator clicking a specific event by hand is
/// asking for exactly that event.
pub fn trigger_event(state: &mut GameState, name: &str, now: Instant) -> bool {
    if state.phase != Phase::Running {
        return false;
    }

    let event = match name {
        "Dyslexia" => SabotageEvent::Dyslexia,
        "UpsideDown" => SabotageEvent::UpsideDown,
        "BlueScreen" | "FakeBlueScreen" => SabotageEvent::BlueScreen,
        "TurkAttack" => SabotageEvent::TurkAttack,
        "Jumpscare" => SabotageEvent::Jumpscare,
        "MirrorMode" => SabotageEvent::MirrorMode,
        "StaticGlitch" => SabotageEvent::StaticGlitch,
        "SirenLights" => SabotageEvent::SirenLights,
        _ => return false,
    };

    events::start_event(state, event, now);
    true
}

pub fn clear_event(state: &mut GameState) {
    state.events.active = None;
    state.events.active_until = None;
}

/// Master switch for random sabotage events. Off is the safe setting for a
/// showcase where the bomb needs to stay legible to an audience.
pub fn set_events_enabled(state: &mut GameState, enabled: bool) {
    state.events_enabled = enabled;
    if !enabled {
        clear_event(state);
    }
}

pub fn end_game(state: &mut GameState, defused: bool) {
    if defused {
        super::win_game(state);
    } else {
        super::lose_game(state, "Ended by operator");
    }
}
