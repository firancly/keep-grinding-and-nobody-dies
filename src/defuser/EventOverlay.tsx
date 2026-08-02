import type { EventInfo } from "../types";

const LABELS: Record<EventInfo["kind"], string> = {
  Dyslexia: "D Y S L E X I A",
  UpsideDown: "UPSIDE DOWN",
  FakeBlueScreen: ":(",
  TurkAttack: "⚠ INTRUSION DETECTED",
  FnafJumpscare: "!!!",
};

// Whole-screen effects. The class on the root wrapper (see Defuser.tsx) does
// the heavy lifting (rotation, scramble); this renders any overlay content.
export function EventOverlay({ event }: { event: EventInfo }) {
  if (event.kind === "FakeBlueScreen") {
    return (
      <div className="event-overlay bsod">
        <div className="bsod-face">:(</div>
        <p>Your bomb ran into a problem and needs to restart.</p>
        <p className="bsod-pct">
          {Math.min(99, 100 - Math.floor(event.remaining_ms / 60))}% complete
        </p>
      </div>
    );
  }
  return (
    <div className={`event-overlay banner event-${event.kind.toLowerCase()}`}>
      <span>{LABELS[event.kind]}</span>
    </div>
  );
}
