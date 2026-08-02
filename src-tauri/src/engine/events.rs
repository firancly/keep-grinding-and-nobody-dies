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
            last: None,
            active_until: None,
            // The run opens with a full quiet gap, so players get oriented
            // before anything starts sabotaging them.
            next_random_at: Some(now + next_gap()),
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

/// How long the screen stays calm between sabotage events, measured from
/// the moment the previous one *ends* - not from when it started. Counting
/// from the start meant a long event ate most of its own interval: an 11s
/// TurkAttack on a 15s interval left only 4 seconds of quiet, which is
/// what made events feel relentless.
///
/// Jittered by +/- `interval_jitter_ms` so the rhythm isn't metronomic.
fn next_gap() -> Duration {
    let events = &crate::config::get().events;
    let jitter = events.interval_jitter_ms as i64;

    let offset = if jitter > 0 {
        rand::rng().random_range(-jitter..=jitter)
    } else {
        0
    };

    Duration::from_millis((events.interval_ms as i64 + offset).max(1_000) as u64)
}

pub fn update(state: &mut GameState, now: Instant) {
    // Operator master switch (see engine::admin) - off means no random
    // events at all, which is the sane setting for a showcase.
    if !state.events_enabled || state.phase != Phase::Running {
        state.events.active = None;
        return;
    }

    // An event that has run its course opens the quiet period.
    if let Some(until) = state.events.active_until {
        if state.events.active.is_some() && now >= until {
            state.events.active = None;
            state.events.next_random_at = Some(now + next_gap());
        }
    }

    // Never stack a second event on top of one that's still on screen.
    if state.events.active.is_some() {
        return;
    }

    // No gap scheduled (first event of the run, or an operator cleared the
    // active one) - open one now so the scheduler can never stall.
    let next_at = match state.events.next_random_at {
        Some(at) => at,
        None => {
            let at = now + next_gap();
            state.events.next_random_at = Some(at);
            at
        }
    };

    if now < next_at {
        return;
    }

    // Hold fire during a critical interaction (Simon mid-playback, the
    // button held down, a module transition) and retry on a later tick
    // rather than firing over the top of it.
    if !safe_to_start(state) {
        return;
    }

    let event = SabotageEvent::random_excluding(state.events.last);
    start_event(state, event, now);
    // The next gap is opened when this event ends.
    state.events.next_random_at = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ModuleKind, PendingAction};

    /// A running game with nothing in flight, so `safe_to_start` passes.
    fn quiet_running_game(now: Instant) -> GameState {
        let mut state = GameState::new();
        crate::engine::start_game(&mut state, now);
        state.remaining_ms = 600_000;
        state.pending = None;
        // Park on a module with no "critical moment" guard of its own.
        let index = state
            .module_order
            .iter()
            .position(|&k| k == ModuleKind::Memory)
            .unwrap();
        state.module_index = index;
        state.events = EventState::new_for_game_start(now);
        state
    }

    fn max_gap() -> Duration {
        let events = &crate::config::get().events;
        Duration::from_millis(events.interval_ms + events.interval_jitter_ms)
    }

    #[test]
    fn the_quiet_gap_is_measured_from_the_end_of_the_previous_event() {
        let start = Instant::now();
        let mut state = quiet_running_game(start);

        // Run the clock forward until the first event fires.
        let mut now = start;
        while state.events.active.is_none() {
            now += Duration::from_millis(50);
            assert!(
                now - start <= max_gap() + Duration::from_millis(100),
                "an event should have fired within one full gap"
            );
            update(&mut state, now);
        }

        let event = state.events.active.expect("an event is running");
        let ends_at = state.events.active_until.expect("it has an end time");

        // Advance to just past the moment it ends.
        now = ends_at + Duration::from_millis(50);
        update(&mut state, now);
        assert!(state.events.active.is_none(), "the event has expired");

        // The next one must not be due until a full gap AFTER that end -
        // this is the regression: it used to be scheduled from the event's
        // START, leaving only a few seconds of calm.
        let next_at = state.events.next_random_at.expect("a new gap opened");
        let gap = next_at - now;
        let floor = Duration::from_millis(
            crate::config::get().events.interval_ms - crate::config::get().events.interval_jitter_ms,
        );
        assert!(
            gap >= floor - Duration::from_millis(100),
            "expected at least {floor:?} of calm after {event:?}, got {gap:?}"
        );

        // And nothing fires during that calm.
        update(&mut state, now + floor / 2);
        assert!(state.events.active.is_none(), "the screen stays calm");
    }

    #[test]
    fn events_never_stack_on_top_of_each_other() {
        let start = Instant::now();
        let mut state = quiet_running_game(start);
        state.events.next_random_at = Some(start);

        update(&mut state, start);
        let first = state.events.active.expect("first event started");

        // Hammer the scheduler while the event is still on screen.
        for step in 1..20 {
            update(&mut state, start + Duration::from_millis(step * 50));
            assert_eq!(
                state.events.active,
                Some(first),
                "a second event must not replace one already on screen"
            );
        }
    }

    #[test]
    fn the_scheduler_recovers_if_the_gap_is_lost() {
        let start = Instant::now();
        let mut state = quiet_running_game(start);

        // Simulate an operator clearing an active event, which leaves no
        // scheduled gap behind.
        state.events.next_random_at = None;
        update(&mut state, start);

        assert!(
            state.events.next_random_at.is_some() || state.events.active.is_some(),
            "the scheduler must re-open a gap rather than stalling forever"
        );
    }

    #[test]
    fn critical_moments_hold_events_off() {
        let start = Instant::now();
        let mut state = quiet_running_game(start);
        state.events.next_random_at = Some(start);

        // A module transition is in flight.
        state.pending = Some((PendingAction::NextModule, start));
        update(&mut state, start);
        assert!(
            state.events.active.is_none(),
            "no event should fire over a module transition"
        );

        // Once it clears, the pending event fires at the next opportunity.
        state.pending = None;
        update(&mut state, start + Duration::from_millis(50));
        assert!(state.events.active.is_some(), "it fires once it's safe again");
    }
}

pub(crate) fn start_event(state: &mut GameState, event: SabotageEvent, now: Instant) {
    state.events.active = Some(event);
    state.events.last = Some(event);
    state.events.active_until = Some(now + event.duration());

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
