//! What a mistake costs inside a staged module.
//!
//! The behaviour these lock down is the one players actually complained
//! about: losing a module's whole progress to a single wrong press, which
//! made long modules feel unfinishable. `mistake_behavior = "advance"`
//! (the default) must never send anyone backwards.
//!
//! The policy is read from the process-wide config `OnceLock`, so these
//! exercise the shipped default; the stricter policies are single match
//! arms in `resolve_mistake` reached by changing game_config.toml.

use std::time::{Duration, Instant};

use super::admin;
use super::{memory, simon, GameState, ModuleKind, PendingAction, Phase};
use crate::config::MistakeBehavior;
use crate::esp_io::{ButtonEdge, EdgeKind};

/// A running game parked on `kind`, with the timer topped up so a time
/// penalty can't end the run underneath the assertions.
fn game_on(kind: ModuleKind, now: Instant) -> GameState {
    let mut state = GameState::new();
    super::start_game(&mut state, now);
    state.remaining_ms = 600_000;

    let index = state
        .module_order
        .iter()
        .position(|&k| k == kind)
        .expect("every run contains all four modules");
    admin::jump_to_module(&mut state, index, now);

    assert_eq!(state.phase, Phase::Running);
    state
}

/// Fires whatever mistake beat is pending, as the engine would once its
/// delay elapsed.
fn resolve_pending_now(state: &mut GameState, now: Instant) {
    let (action, _) = state.pending.expect("a mistake should have scheduled a beat");
    state.pending = None;
    super::resolve_pending(state, action, now);
}

fn press(button: u8) -> ButtonEdge {
    ButtonEdge {
        button,
        kind: EdgeKind::Press,
        t_ms: 0,
    }
}

#[test]
fn configured_default_is_the_forgiving_one() {
    assert_eq!(
        crate::config::get().game.mistake_behavior,
        MistakeBehavior::Advance,
        "game_config.toml should ship with the non-punishing default"
    );
}

#[test]
fn advance_keeps_memory_progress_and_moves_to_the_next_stage() {
    let now = Instant::now();
    let mut state = game_on(ModuleKind::Memory, now);
    state.memory.stage = 3; // deep into the module - the painful case

    let correct = memory::correct_position(&state.memory);
    let wrong = (0..4).find(|&b| b != correct).unwrap();

    memory::on_edge(&mut state, &press(wrong), now);
    resolve_pending_now(&mut state, now + Duration::from_millis(1600));

    assert_eq!(state.mistakes, 1, "the strike still lands");
    assert_eq!(
        state.memory.stage, 4,
        "the failed stage counts - progress must never go backwards"
    );
    assert_eq!(
        state.memory.pressed_position[3], wrong,
        "later stages reference the press that actually happened"
    );
}

#[test]
fn advance_solves_memory_when_the_final_stage_is_failed() {
    let now = Instant::now();
    let mut state = game_on(ModuleKind::Memory, now);
    state.memory.stage = 4; // last stage

    let correct = memory::correct_position(&state.memory);
    let wrong = (0..4).find(|&b| b != correct).unwrap();

    memory::on_edge(&mut state, &press(wrong), now);
    resolve_pending_now(&mut state, now + Duration::from_millis(1600));

    assert_eq!(state.memory.stage, 5);
    assert!(
        matches!(state.pending, Some((PendingAction::NextModule, _))),
        "failing the last stage under `advance` still completes the module"
    );
}

/// Colour Memory is a single fixed-length sequence, so there are no stages
/// to bank - a wrong press replays the SAME colours rather than dealing a
/// new puzzle, which is what keeps it learnable instead of punishing.
#[test]
fn a_wrong_colour_replays_the_same_sequence() {
    let now = Instant::now();
    let mut state = game_on(ModuleKind::Simon, now);
    state.simon.mode = simon::SimonMode::Input;
    state.simon.input_index = 1; // already got one right

    let sequence_before = state.simon.sequence.clone();
    let expected = simon::mapped_button(&state, state.simon.sequence[1]);
    let wrong = (0..4).find(|&b| b != expected).unwrap();

    simon::on_edge(&mut state, &press(wrong), now);
    resolve_pending_now(&mut state, now + Duration::from_millis(1600));

    assert_eq!(state.mistakes, 1, "the strike still lands");
    assert_eq!(
        state.simon.sequence, sequence_before,
        "the same colours are replayed - not a fresh sequence to memorise"
    );
    assert_eq!(state.simon.input_index, 0, "input starts over");
    assert_eq!(state.simon.mode, simon::SimonMode::Waiting, "playback resumes");
}

/// The mapping is a straight colour -> button lookup, so a strike must not
/// silently change which button answers a colour.
#[test]
fn the_colour_mapping_never_shifts_with_strikes() {
    let now = Instant::now();
    let mut state = game_on(ModuleKind::Simon, now);

    let before: Vec<u8> = (0..4).map(|c| simon::mapped_button(&state, c)).collect();
    state.mistakes = 2;
    let after: Vec<u8> = (0..4).map(|c| simon::mapped_button(&state, c)).collect();

    assert_eq!(before, after, "colour -> button stays fixed across strikes");
}

#[test]
fn the_strike_message_tells_players_they_kept_their_progress() {
    let now = Instant::now();
    let mut state = game_on(ModuleKind::Memory, now);

    let correct = memory::correct_position(&state.memory);
    let wrong = (0..4).find(|&b| b != correct).unwrap();
    memory::on_edge(&mut state, &press(wrong), now);

    assert_eq!(state.status_title, "SECOND CHANCE");
    assert!(
        state.status_detail.contains("stage counted, moving on"),
        "both screens must say progress was kept, got: {}",
        state.status_detail
    );
}
