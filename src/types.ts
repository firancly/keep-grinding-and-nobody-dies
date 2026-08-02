// Shared protocol types — these mirror the Rust `DefuserView` serde structs.
// When the backend is ready, the frontend receives one of these per
// `state:update` event; until then we render off `mock.ts`.

export interface DefuserView {
  phase: "Idle" | "Running" | "Defused" | "Exploded";
  timer_ms: number; // count DOWN — format as mm:ss
  strikes: number;
  max_strikes: number;
  serial: string; // defuser reads this aloud to the expert
  bomb_type: "Alpha" | "Beta" | "Omega";
  power: "Battery" | "Electricity";
  temp: { value: number; band: "Low" | "Medium" | "High" };
  modules: ModuleState[];
  active_event: EventInfo | null;
}

export type ModuleState =
  | WiresModule
  | SimonModule
  | PasswordModule
  | HoldModule;

export type WireColor =
  | "red"
  | "blue"
  | "green"
  | "white"
  | "black"
  | "orange"
  | "yellow";

export type SimonColor = "red" | "blue" | "green" | "yellow";

export interface WiresModule {
  kind: "Wires";
  slots: { id: number; color: WireColor; cut: boolean }[];
  solved: boolean;
}

export interface SimonModule {
  kind: "Simon";
  flash_sequence: SimonColor[]; // Rust generates — the UI plays it back
  input_len: number; // how many correct presses so far (progress)
  solved: boolean;
}

export interface PasswordModule {
  kind: "Password";
  current_char: string; // the letter currently shown at the active slot
  position: number; // which slot (0-based) is being edited
  len: number; // total slots, e.g. 5
  solved: boolean;
}

export interface HoldModule {
  kind: "Hold";
  color: SimonColor;
  label: string; // text on the button, e.g. "HOLD" / "ABORT"
  holding: boolean; // true while physically pressed
  solved: boolean;
}

export type EventKind =
  | "Dyslexia"
  | "UpsideDown"
  | "FakeBlueScreen"
  | "TurkAttack"
  | "FnafJumpscare";

export interface EventInfo {
  kind: EventKind;
  remaining_ms: number;
}
