use std::sync::OnceLock;

use serde::Deserialize;

// Loaded once from `game_config.toml` (project root), lazily on first access.
// Restart the app after editing the file for changes to take effect.
static CONFIG: OnceLock<GameConfig> = OnceLock::new();

pub fn get() -> &'static GameConfig {
    CONFIG.get_or_init(load)
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct GameConfig {
    pub game: GameSettings,
    pub events: EventSettings,
    pub stimulation: StimulationSettings,
}

/// What a mistake costs *inside* a staged module (Memory, Simon Says).
/// The strike and the time penalty always apply - this only decides what
/// happens to the progress already made in that module.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MistakeBehavior {
    /// The failed stage still counts; play continues at the next stage
    /// (and the module is solved if that was the last one). Nothing is
    /// ever re-done, so a module can't trap a group in a failure loop.
    #[default]
    Advance,
    /// Keep every earlier stage, but replay the one that was failed.
    RetryStage,
    /// Wipe the module's progress and start it over (the strictest,
    /// original behaviour).
    RestartModule,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    pub total_time_ms: i64,
    /// How many mistakes the defuser can make before the bomb explodes.
    /// Every mistake before the last one costs `first_mistake_penalty_ms`;
    /// the attempts-th one ends the game immediately.
    pub attempts: u8,
    pub first_mistake_penalty_ms: i64,
    pub hold_threshold_ms: u32,
    pub simon_stages: u8,
    pub mistake_behavior: MistakeBehavior,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            total_time_ms: 180_000,
            attempts: 2,
            first_mistake_penalty_ms: 20_000,
            hold_threshold_ms: 700,
            simon_stages: 5,
            mistake_behavior: MistakeBehavior::Advance,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EventSettings {
    /// Quiet gap between sabotage events, measured from the moment the
    /// previous one *ends* (so an event's own duration never eats into it).
    pub interval_ms: u64,
    /// The gap is randomised by +/- this much, so events don't arrive on a
    /// predictable metronome. 0 disables the jitter.
    pub interval_jitter_ms: u64,
    pub duration_ms: EventDurations,
    pub jumpscare: JumpscareSettings,
}

impl Default for EventSettings {
    fn default() -> Self {
        EventSettings {
            interval_ms: 45_000,
            interval_jitter_ms: 15_000,
            duration_ms: EventDurations::default(),
            jumpscare: JumpscareSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EventDurations {
    pub dyslexia: u64,
    pub upside_down: u64,
    pub blue_screen: u64,
    pub turk_attack: u64,
    pub jumpscare: u64,
    pub mirror_mode: u64,
    pub static_glitch: u64,
    pub siren_lights: u64,
}

impl Default for EventDurations {
    fn default() -> Self {
        EventDurations {
            dyslexia: 9_000,
            upside_down: 7_000,
            blue_screen: 9_000,
            turk_attack: 11_000,
            jumpscare: 3_500,
            mirror_mode: 10_000,
            static_glitch: 5_000,
            siren_lights: 10_000,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct JumpscareSettings {
    /// Up to 3 file paths to jumpscare video files. One is picked at random
    /// each time the Jumpscare event fires. Empty = fall back to the
    /// built-in CSS monster-face effect.
    pub video_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct StimulationSettings {
    /// File paths to short gameplay/feed clips for the tablet's
    /// "stimulation corner" (played muted, on loop, auto-advancing).
    /// Empty = the corner never appears.
    pub video_paths: Vec<String>,
}

/// Tries `game_config.toml` in the current working directory first (matches
/// running the built .exe directly), then `../game_config.toml` (matches
/// `cargo run`'s CWD of `src-tauri/` during `pnpm tauri dev`, with the file
/// sitting at the project root). Falls back to defaults if neither is found
/// or the file fails to parse - the app should never fail to start over a
/// bad config file.
fn load() -> GameConfig {
    for candidate in ["game_config.toml", "../game_config.toml"] {
        match std::fs::read_to_string(candidate) {
            Ok(text) => match toml::from_str::<GameConfig>(&text) {
                Ok(config) => {
                    println!("game_config.toml yuklendi: {candidate}");
                    return config;
                }
                Err(error) => {
                    eprintln!("{candidate} okunamadi (format hatasi), varsayilan degerler kullaniliyor: {error}");
                    return GameConfig::default();
                }
            },
            Err(_) => continue,
        }
    }

    println!("game_config.toml bulunamadi, varsayilan degerler kullaniliyor");
    GameConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_root_game_config_parses() {
        let text = std::fs::read_to_string("../game_config.toml")
            .expect("game_config.toml should exist at the project root");
        let config: GameConfig =
            toml::from_str(&text).expect("game_config.toml should be valid TOML matching GameConfig");

        assert_eq!(config.game.total_time_ms, 240_000);
        assert_eq!(config.events.jumpscare.video_paths.len(), 0);
    }
}
