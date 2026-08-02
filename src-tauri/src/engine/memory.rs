use std::time::Instant;

use rand::RngExt;

use super::{register_mistake, solve_current_module, GameState, PendingAction};
use crate::config::MistakeBehavior;
use crate::esp_io::{ButtonEdge, EdgeKind};

#[derive(Debug, Clone, Default)]
pub struct MemoryState {
    /// 0-4 while playing, 5 once solved.
    pub stage: u8,
    pub display: u8,
    pub labels: [u8; 4],
    pub pressed_position: [u8; 5],
    pub pressed_label: [u8; 5],
}

pub fn start(state: &mut GameState) {
    state.memory.stage = 0;
    state.memory.pressed_position = [0; 5];
    state.memory.pressed_label = [0; 5];
    prepare_stage(state);
}

fn prepare_stage(state: &mut GameState) {
    let mut rng = rand::rng();
    state.memory.display = rng.random_range(1..=4);
    state.memory.labels = super::shuffle4(&mut rng).map(|v| v + 1);

    state.status_title = "MEMORY".to_string();
    state.status_detail = "Choose position 1, 2, 3, or 4".to_string();
}

fn position_with_label(memory: &MemoryState, label: u8) -> u8 {
    memory
        .labels
        .iter()
        .position(|&l| l == label)
        .map(|i| i as u8)
        .unwrap_or(0)
}

pub fn correct_position(memory: &MemoryState) -> u8 {
    match memory.stage {
        0 => {
            if memory.display <= 2 {
                1
            } else if memory.display == 3 {
                2
            } else {
                3
            }
        }
        1 => {
            if memory.display == 1 {
                position_with_label(memory, 4)
            } else if memory.display == 2 || memory.display == 4 {
                memory.pressed_position[0]
            } else {
                0
            }
        }
        2 => {
            if memory.display == 1 {
                position_with_label(memory, memory.pressed_label[1])
            } else if memory.display == 2 {
                position_with_label(memory, memory.pressed_label[0])
            } else if memory.display == 3 {
                2
            } else {
                position_with_label(memory, 4)
            }
        }
        3 => {
            if memory.display == 1 {
                memory.pressed_position[0]
            } else if memory.display == 2 {
                0
            } else {
                memory.pressed_position[1]
            }
        }
        4 => {
            if memory.display == 1 {
                position_with_label(memory, memory.pressed_label[0])
            } else if memory.display == 2 {
                position_with_label(memory, memory.pressed_label[1])
            } else if memory.display == 3 {
                position_with_label(memory, memory.pressed_label[3])
            } else {
                position_with_label(memory, memory.pressed_label[2])
            }
        }
        _ => 0,
    }
}

pub fn on_edge(state: &mut GameState, edge: &ButtonEdge, now: Instant) {
    if edge.kind != EdgeKind::Press || edge.button > 3 {
        return;
    }

    let i = edge.button;
    let correct = correct_position(&state.memory);

    if i != correct {
        // Under `advance`, the stage is credited using the button they
        // ACTUALLY pressed, not the one they should have. Later stages'
        // rules refer to "the position/label you pressed in stage N", and
        // both the defuser and the expert watched the real press - so
        // recording it is what keeps the manual truthful from here on.
        if crate::config::get().game.mistake_behavior == MistakeBehavior::Advance {
            record_press(state, i);
        }

        register_mistake(
            state,
            &format!("Wrong Memory position | {}", super::mistake_note()),
            PendingAction::MemoryMistake,
            now,
        );
        return;
    }

    record_press(state, i);

    if state.memory.stage >= 5 {
        solve_current_module(state, now);
    } else {
        prepare_stage(state);
    }
}

/// Banks the press for the current stage and moves the stage counter on.
/// Guarded against running past the 5 recorded slots.
fn record_press(state: &mut GameState, i: u8) {
    let stage = state.memory.stage as usize;
    if stage >= 5 {
        return;
    }

    state.memory.pressed_position[stage] = i;
    state.memory.pressed_label[stage] = state.memory.labels[i as usize];
    state.memory.stage += 1;
}

/// Runs once the "SECOND CHANCE" beat after a wrong press has elapsed.
pub fn resolve_mistake(state: &mut GameState, now: Instant) {
    match crate::config::get().game.mistake_behavior {
        // The press was already banked in `on_edge`; carry on (or finish
        // the module if that was the fifth stage).
        MistakeBehavior::Advance => {
            if state.memory.stage >= 5 {
                solve_current_module(state, now);
            } else {
                prepare_stage(state);
            }
        }
        // Earlier stages survive - only this stage is dealt again.
        MistakeBehavior::RetryStage => prepare_stage(state),
        MistakeBehavior::RestartModule => reset(state),
    }
}

pub fn reset(state: &mut GameState) {
    state.memory.stage = 0;
    state.memory.pressed_position = [0; 5];
    state.memory.pressed_label = [0; 5];
    prepare_stage(state);
}
