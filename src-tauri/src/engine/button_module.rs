use std::time::Instant;

use rand::RngExt;

use super::{register_mistake, solve_current_module, GameState, PendingAction};
use crate::esp_io::{ButtonEdge, EdgeKind, IoSnapshot};

#[derive(Debug, Clone, Default)]
pub struct ButtonState {
    pub active_slot: u8,
    pub virtual_color: u8,
    pub virtual_label: u8,
    pub rule_says_hold: bool,
    pub is_down: bool,
    pub pressed_at_esp_ms: Option<u32>,
    pub strip_visible: bool,
    pub strip_color: u8,
}

fn calculate_should_hold(color: u8, label: u8, batteries: u8, car: bool, frk: bool) -> bool {
    if color == 0 && label == 0 {
        return true;
    }
    if batteries > 1 && label == 1 {
        return false;
    }
    if color == 1 && car {
        return true;
    }
    if batteries > 2 && frk {
        return false;
    }
    if color == 2 {
        return true;
    }
    if color == 3 && label == 2 {
        return false;
    }
    true
}

fn release_digit_for_strip(color: u8) -> u8 {
    if color == 0 {
        4
    } else if color == 2 {
        5
    } else {
        1
    }
}

/// Checks the release rule against the same M:SS digits both displays
/// actually show (see `super::timer_display_value`). Note the target digit
/// must never be 0: the tablet zero-pads minutes ("2:53" renders as
/// "02:53"), so a 0 is visible on one display but not the other.
fn timer_contains_digit(remaining_ms: i64, digit: u8) -> bool {
    let value = super::timer_display_value(remaining_ms);
    [value / 100, (value / 10) % 10, value % 10]
        .iter()
        .any(|&d| d as u8 == digit)
}

pub fn start(state: &mut GameState) {
    let mut rng = rand::rng();
    state.button.active_slot = rng.random_range(0..4);
    state.button.virtual_color = rng.random_range(0..4);
    state.button.virtual_label = rng.random_range(0..4);
    state.button.rule_says_hold = calculate_should_hold(
        state.button.virtual_color,
        state.button.virtual_label,
        state.batteries,
        state.car,
        state.frk,
    );
    state.button.is_down = false;
    state.button.pressed_at_esp_ms = None;
    state.button.strip_visible = false;

    state.status_title = "THE BUTTON".to_string();
    state.status_detail = "Ask the expert: tap or hold?".to_string();
}

pub fn retry(state: &mut GameState) {
    state.button.is_down = false;
    state.button.pressed_at_esp_ms = None;
    state.button.strip_visible = false;

    state.status_title = "THE BUTTON".to_string();
    state.status_detail = "Second chance: try the same button".to_string();
}

pub fn on_edge(state: &mut GameState, edge: &ButtonEdge, now: Instant) {
    let active_slot = state.button.active_slot;

    if edge.button != active_slot {
        if edge.kind == EdgeKind::Press {
            register_mistake(state, "Wrong physical button", PendingAction::ButtonRetry, now);
        }
        return;
    }

    match edge.kind {
        EdgeKind::Press => {
            state.button.is_down = true;
            state.button.pressed_at_esp_ms = Some(edge.t_ms);
            state.button.strip_visible = false;
            state.status_detail = "Button is being held".to_string();
        }
        EdgeKind::Release => {
            if !state.button.is_down {
                return;
            }

            let pressed_at = state.button.pressed_at_esp_ms.unwrap_or(edge.t_ms);
            let held_for = edge.t_ms.wrapping_sub(pressed_at);
            state.button.is_down = false;

            if !state.button.rule_says_hold {
                if held_for < crate::config::get().game.hold_threshold_ms {
                    solve_current_module(state, now);
                } else {
                    register_mistake(
                        state,
                        "This button should have been tapped",
                        PendingAction::ButtonRetry,
                        now,
                    );
                }
                return;
            }

            if !state.button.strip_visible {
                // The poll cadence can miss the exact instant the hold
                // threshold was crossed if the release lands in the same
                // batch of edges. If the hold was genuinely long enough,
                // the player did everything right but never got shown a
                // strip - judging their release digit against a strip they
                // couldn't see would be a coin-flip mistake, so count it
                // as a solve instead.
                if held_for >= crate::config::get().game.hold_threshold_ms {
                    solve_current_module(state, now);
                } else {
                    register_mistake(
                        state,
                        "Released before the strip appeared",
                        PendingAction::ButtonRetry,
                        now,
                    );
                }
                return;
            }

            let target_digit = release_digit_for_strip(state.button.strip_color);

            if timer_contains_digit(state.remaining_ms, target_digit) {
                solve_current_module(state, now);
            } else {
                register_mistake(
                    state,
                    "Released on the wrong timer digit",
                    PendingAction::ButtonRetry,
                    now,
                );
            }
        }
    }
}

pub fn on_tick(state: &mut GameState, io: &IoSnapshot) {
    let active_slot = state.button.active_slot as usize;

    if !state.button.is_down || state.button.strip_visible {
        return;
    }

    if !io.buttons_stable.get(active_slot).copied().unwrap_or(false) {
        return;
    }

    if let Some(pressed_at) = state.button.pressed_at_esp_ms {
        if io.esp_millis.wrapping_sub(pressed_at) >= crate::config::get().game.hold_threshold_ms {
            state.button.strip_visible = true;
            state.button.strip_color = rand::rng().random_range(0..4);
            state.status_detail = "Strip revealed".to_string();
        }
    }
}
