use std::time::Instant;

use rand::RngExt;

use super::{register_mistake, solve_current_module, GameState, PendingAction};
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
        register_mistake(state, "Wrong Memory position", PendingAction::MemoryReset, now);
        return;
    }

    let stage = state.memory.stage as usize;
    state.memory.pressed_position[stage] = i;
    state.memory.pressed_label[stage] = state.memory.labels[i as usize];
    state.memory.stage += 1;

    if state.memory.stage >= 5 {
        solve_current_module(state, now);
    } else {
        prepare_stage(state);
    }
}

pub fn reset(state: &mut GameState) {
    state.memory.stage = 0;
    state.memory.pressed_position = [0; 5];
    state.memory.pressed_label = [0; 5];
    prepare_stage(state);
}
