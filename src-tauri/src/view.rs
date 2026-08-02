use std::time::Instant;

use serde::Serialize;

use crate::engine::events::SabotageEvent;
use crate::engine::{wires, GameState, ModuleKind, PendingAction, Phase};
use crate::esp_io::IoSnapshot;

// Mirrors src/types.ts's `DefuserView`. Field names are already snake_case
// matching the TypeScript interface verbatim, so no serde renaming is
// needed anywhere in this file.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DefuserView {
    pub phase: Phase,
    pub timer_ms: i64,
    pub strikes: u8,
    pub max_strikes: u8,
    pub serial: String,
    pub batteries: u8,
    pub car: bool,
    pub frk: bool,
    pub modules: Vec<ModuleState>,
    pub active_event: Option<EventInfo>,
    /// Index into `modules` of the module currently being played, or `None`
    /// while idle/defused/exploded. Modules before this index are solved;
    /// modules after it haven't been started yet (their fields still hold
    /// placeholder defaults, not a real puzzle instance).
    pub active_module_index: Option<usize>,
    /// Echoes game_config.toml's `hold_threshold_ms`/`simon_stages` so the
    /// expert-facing manual can describe them accurately instead of
    /// hardcoding numbers that silently go stale whenever config changes.
    pub hold_threshold_ms: u32,
    pub simon_max_stages: u8,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct WireSlot {
    pub index: u8,
    pub cut: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind")]
pub enum ModuleState {
    Wires {
        wires: [WireSlot; 4],
        armed: bool,
        alien_phrase: String,
        alien_class: &'static str,
        alien_grammar: &'static str,
        alien_action: &'static str,
        alien_noun: &'static str,
        alien_ordinals: [&'static str; 4],
        solved: bool,
    },
    Simon {
        flash_sequence: Vec<&'static str>,
        input_len: u8,
        solved: bool,
    },
    Memory {
        stage: u8,
        display: u8,
        labels: [u8; 4],
        solved: bool,
    },
    Hold {
        color: &'static str,
        label: &'static str,
        active_slot: u8,
        holding: bool,
        strip_visible: bool,
        strip_color: Option<&'static str>,
        solved: bool,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EventInfo {
    pub kind: &'static str,
    pub remaining_ms: u32,
    /// Set only for the Jumpscare event, when game_config.toml has at least
    /// one video configured - a full URL the frontend can load directly
    /// (served by the relay's /jumpscare-video/<n> route).
    pub video_url: Option<String>,
}

fn simon_color_str(index: u8) -> &'static str {
    match index {
        0 => "red",
        1 => "blue",
        2 => "green",
        _ => "yellow",
    }
}

fn button_color_str(index: u8) -> &'static str {
    match index {
        0 => "blue",
        1 => "white",
        2 => "yellow",
        _ => "red",
    }
}

fn button_label_str(index: u8) -> &'static str {
    match index {
        0 => "abort",
        1 => "detonate",
        2 => "hold",
        _ => "press",
    }
}

fn event_kind_str(event: SabotageEvent) -> &'static str {
    match event {
        SabotageEvent::Dyslexia => "Dyslexia",
        SabotageEvent::UpsideDown => "UpsideDown",
        SabotageEvent::BlueScreen => "FakeBlueScreen",
        SabotageEvent::TurkAttack => "TurkAttack",
        SabotageEvent::Jumpscare => "Jumpscare",
        SabotageEvent::MirrorMode => "MirrorMode",
        SabotageEvent::StaticGlitch => "StaticGlitch",
        SabotageEvent::SirenLights => "SirenLights",
    }
}

pub fn build_view(state: &GameState, io: &IoSnapshot, now: Instant) -> DefuserView {
    let modules = state
        .module_order
        .iter()
        .enumerate()
        .map(|(order_index, &kind)| build_module(state, io, kind, order_index))
        .collect();

    let active_event = state.events.active.map(|event| EventInfo {
        kind: event_kind_str(event),
        remaining_ms: state
            .events
            .active_until
            .map(|until| until.saturating_duration_since(now).as_millis() as u32)
            .unwrap_or(0),
        // Relative, not absolute - the tablet loads this page from
        // http://<laptop-ip>:4000/, and a hardcoded "localhost" here would
        // resolve to the tablet itself instead of the laptop, breaking
        // video playback on any device but the laptop.
        video_url: state
            .events
            .jumpscare_video_index
            .map(|index| format!("/jumpscare-video/{index}")),
    });

    let active_module_index =
        if state.phase == Phase::Running && state.module_index < state.module_order.len() {
            Some(state.module_index)
        } else {
            None
        };

    DefuserView {
        phase: state.phase,
        timer_ms: state.remaining_ms.max(0),
        strikes: state.mistakes,
        max_strikes: crate::config::get().game.attempts,
        serial: state.serial.clone(),
        batteries: state.batteries,
        car: state.car,
        frk: state.frk,
        modules,
        active_event,
        active_module_index,
        hold_threshold_ms: crate::config::get().game.hold_threshold_ms,
        simon_max_stages: crate::config::get().game.simon_stages,
    }
}

fn build_module(state: &GameState, io: &IoSnapshot, kind: ModuleKind, order_index: usize) -> ModuleState {
    let solved = if order_index < state.module_index {
        true
    } else if order_index == state.module_index {
        matches!(state.pending, Some((PendingAction::NextModule, _)))
    } else {
        false
    };

    match kind {
        ModuleKind::Memory => ModuleState::Memory {
            stage: state.memory.stage + 1,
            display: state.memory.display,
            labels: state.memory.labels,
            solved,
        },
        ModuleKind::Simon => {
            let flash_sequence = state.simon.sequence[..state.simon.stage_length as usize]
                .iter()
                .map(|&color| simon_color_str(color))
                .collect();

            ModuleState::Simon {
                flash_sequence,
                input_len: state.simon.input_index,
                solved,
            }
        }
        ModuleKind::Button => ModuleState::Hold {
            color: button_color_str(state.button.virtual_color),
            label: button_label_str(state.button.virtual_label),
            active_slot: state.button.active_slot,
            holding: state.button.is_down,
            strip_visible: state.button.strip_visible,
            strip_color: if state.button.strip_visible {
                Some(button_color_str(state.button.strip_color))
            } else {
                None
            },
            solved,
        },
        ModuleKind::Wires => {
            let mut wire_slots = [WireSlot { index: 0, cut: false }; 4];
            for (i, slot) in wire_slots.iter_mut().enumerate() {
                *slot = WireSlot {
                    index: i as u8,
                    cut: io.wires_cut[i],
                };
            }

            ModuleState::Wires {
                wires: wire_slots,
                armed: state.wires.armed,
                alien_phrase: wires::phrase(&state.wires),
                alien_class: wires::CLASS_NAME[state.wires.class as usize],
                alien_grammar: wires::grammar_name(state.wires.grammar),
                alien_action: wires::action_word(state.wires.action_set),
                alien_noun: wires::noun_word(state.wires.noun_set),
                alien_ordinals: wires::ordinal_words(state.wires.ordinal_set),
                solved,
            }
        }
    }
}
