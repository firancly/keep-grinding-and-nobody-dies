use std::time::{Duration, Instant};

use rand::RngExt;

use super::simon::SimonMode;
use super::{GameState, ModuleKind, Phase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SabotageEvent {
    Dyslexia,
    UpsideDown,
    BlueScreen,
    TurkAttack,
    Jumpscare,
    MirrorMode,
    StaticGlitch,
    SirenLights,
}

impl SabotageEvent {
    pub fn duration(self) -> Duration {
        let durations = &crate::config::get().events.duration_ms;
        Duration::from_millis(match self {
            SabotageEvent::Dyslexia => durations.dyslexia,
            SabotageEvent::UpsideDown => durations.upside_down,
            SabotageEvent::BlueScreen => durations.blue_screen,
            SabotageEvent::TurkAttack => durations.turk_attack,
            SabotageEvent::Jumpscare => durations.jumpscare,
            SabotageEvent::MirrorMode => durations.mirror_mode,
            SabotageEvent::StaticGlitch => durations.static_glitch,
            SabotageEvent::SirenLights => durations.siren_lights,
        })
    }

    fn random_excluding(last: Option<SabotageEvent>) -> SabotageEvent {
        use SabotageEvent::*;
        const ALL: [SabotageEvent; 8] = [
            Dyslexia, UpsideDown, BlueScreen, TurkAttack, Jumpscare, MirrorMode, StaticGlitch,
            SirenLights,
        ];
        let mut rng = rand::rng();
        loop {
            let candidate = ALL[rng.random_range(0..8)];
            if Some(candidate) != last {
                return candidate;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventState {
    pub active: Option<SabotageEvent>,
    pub queued: Option<SabotageEvent>,
    pub last: Option<SabotageEvent>,
    pub active_until: Option<Instant>,
    pub next_random_at: Option<Instant>,
    /// Which of game_config.toml's `events.jumpscare.video_paths` was
    /// randomly picked for the current Jumpscare event, if any.
    pub jumpscare_video_index: Option<usize>,
}

impl EventState {
    pub fn new_for_game_start(now: Instant) -> Self {
        EventState {
            active: None,
            queued: None,
            last: None,
            active_until: None,
            next_random_at: Some(now + Duration::from_millis(crate::config::get().events.interval_ms)),
            jumpscare_video_index: None,
        }
    }
}

pub fn blocks_input(events: &EventState) -> bool {
    matches!(events.active, Some(SabotageEvent::BlueScreen) | Some(SabotageEvent::Jumpscare))
}

fn safe_to_start(state: &GameState) -> bool {
    if state.pending.is_some() {
        return false;
    }

    if state.phase == Phase::Running {
        let current_module = state.module_order[state.module_index];
        if current_module == ModuleKind::Simon && state.simon.mode != SimonMode::Input {
            return false;
        }
        if current_module == ModuleKind::Button && state.button.is_down {
            return false;
        }
    }

    true
}

pub fn update(state: &mut GameState, now: Instant) {
    if state.phase != Phase::Running {
        state.events.active = None;
        state.events.queued = None;
        return;
    }

    if let Some(until) = state.events.active_until {
        if state.events.active.is_some() && now >= until {
            state.events.active = None;
        }
    }

    if let Some(next_at) = state.events.next_random_at {
        if now >= next_at {
            state.events.next_random_at =
                Some(next_at + Duration::from_millis(crate::config::get().events.interval_ms));
            let next = SabotageEvent::random_excluding(state.events.last);

            if state.events.active.is_none() && safe_to_start(state) {
                start_event(state, next, now);
            } else {
                state.events.queued = Some(next);
            }
        }
    }

    if state.events.active.is_none() {
        if let Some(queued) = state.events.queued {
            if safe_to_start(state) {
                start_event(state, queued, now);
            }
        }
    }
}

fn start_event(state: &mut GameState, event: SabotageEvent, now: Instant) {
    state.events.active = Some(event);
    state.events.last = Some(event);
    state.events.active_until = Some(now + event.duration());
    state.events.queued = None;

    state.events.jumpscare_video_index = if event == SabotageEvent::Jumpscare {
        let video_paths = &crate::config::get().events.jumpscare.video_paths;
        if video_paths.is_empty() {
            None
        } else {
            Some(rand::rng().random_range(0..video_paths.len()))
        }
    } else {
        None
    };
}
