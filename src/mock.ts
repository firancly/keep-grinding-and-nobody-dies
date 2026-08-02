import type { DefuserView } from "./types";

// Standalone fixture so the whole Defuser UI can be built and styled with
// zero backend. Swap this for `listen<DefuserView>("state:update", ...)`
// once Rust is emitting.
export const mockDefuser: DefuserView = {
  phase: "Running",
  timer_ms: 183000,
  strikes: 1,
  max_strikes: 3,
  serial: "KT4X9B",
  bomb_type: "Beta",
  power: "Battery",
  temp: { value: 72, band: "High" },
  modules: [
    {
      kind: "Wires",
      solved: false,
      slots: [
        { id: 0, color: "red", cut: false },
        { id: 1, color: "blue", cut: true },
        { id: 2, color: "yellow", cut: false },
        { id: 3, color: "white", cut: false },
        { id: 4, color: "green", cut: false },
      ],
    },
    {
      kind: "Simon",
      solved: false,
      flash_sequence: ["red", "green", "green", "blue"],
      input_len: 2,
    },
    { kind: "Password", solved: false, current_char: "K", position: 1, len: 5 },
    {
      kind: "Hold",
      solved: true,
      color: "green",
      label: "HOLD",
      holding: false,
    },
  ],
  active_event: { kind: "Dyslexia", remaining_ms: 4200 },
};
