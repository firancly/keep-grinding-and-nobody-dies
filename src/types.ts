// Shared protocol types — these mirror the Rust `DefuserView` serde structs
// in src-tauri/src/view.rs exactly. The frontend receives one of these per
// `state:update` event.

export interface DefuserView {
  phase: "Idle" | "Running" | "Defused" | "Exploded";
  timer_ms: number; // count DOWN — format as mm:ss
  strikes: number;
  max_strikes: number;
  serial: string; // defuser reads this aloud to the expert
  batteries: number; // 0-3
  car: boolean; // "CAR" indicator lit
  frk: boolean; // "FRK" indicator lit
  modules: ModuleState[];
  active_event: EventInfo | null;
  // Index into `modules` of the module currently being played, or null
  // while idle/defused/exploded. Modules before this index are solved;
  // modules after it haven't started yet (placeholder data only).
  active_module_index: number | null;
  // Echoes game_config.toml so the manual can describe them accurately
  // instead of hardcoding numbers that go stale when config changes.
  hold_threshold_ms: number;
  simon_max_stages: number;
  // One-line game feedback ("SECOND CHANCE", "MODULE SOLVED", ...) plus a
  // detail sentence — rendered on both screens so mistakes are never silent.
  status_title: string;
  status_detail: string;
}

export type ModuleState = WiresModule | SimonModule | MemoryModule | HoldModule;

export interface WireSlot {
  index: number; // 0-3, physical wire position
  cut: boolean;
}

export interface WiresModule {
  kind: "Wires";
  wires: [WireSlot, WireSlot, WireSlot, WireSlot];
  armed: boolean; // false until all 4 wires have been reconnected
  alien_phrase: string;
  alien_class: "ALPHA" | "BETA" | "OMEGA";
  alien_grammar: string; // e.g. "ACTION - INDEX - OBJECT"
  alien_action: string;
  alien_noun: string;
  alien_ordinals: [string, string, string, string];
  solved: boolean;
}

export type SimonColor = "red" | "blue" | "green" | "yellow";

export interface SimonModule {
  kind: "Simon";
  flash_sequence: SimonColor[]; // full sequence this stage (for progress pips)
  input_len: number; // how many correct presses so far (progress)
  // "watch" while the engine plays the sequence back (presses are ignored),
  // "input" once it's the defuser's turn to repeat it.
  mode: "watch" | "input";
  // The color the engine is flashing right now — the tablet lights its pads
  // from this, so the visible flashes are exactly the engine's timing.
  lit: SimonColor | null;
  solved: boolean;
}

export interface MemoryModule {
  kind: "Memory";
  stage: number; // 1-5
  display: number; // 1-4, the number shown this stage
  labels: [number, number, number, number]; // label at physical position 0-3
  solved: boolean;
}

export type ButtonColor = "blue" | "white" | "yellow" | "red";
export type ButtonLabel = "abort" | "detonate" | "hold" | "press";
export type StripColor = "blue" | "white" | "yellow" | "red";

export interface HoldModule {
  kind: "Hold";
  color: ButtonColor;
  label: ButtonLabel;
  active_slot: number; // 0-3, which physical button is "the button"
  holding: boolean; // true while physically pressed
  strip_visible: boolean;
  strip_color: StripColor | null;
  solved: boolean;
}

export type EventKind =
  | "Dyslexia"
  | "UpsideDown"
  | "FakeBlueScreen"
  | "TurkAttack"
  | "Jumpscare"
  | "MirrorMode"
  | "StaticGlitch"
  | "SirenLights";

export interface EventInfo {
  kind: EventKind;
  remaining_ms: number;
  // Only set for "Jumpscare", when game_config.toml has at least one video
  // configured - a ready-to-use URL (served by the relay). Null = fall back
  // to the built-in CSS monster-face effect.
  video_url: string | null;
}
