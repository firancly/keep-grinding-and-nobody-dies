import type { DefuserView } from "../types";

function fmt(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = String(Math.floor(total / 60)).padStart(2, "0");
  const s = String(total % 60).padStart(2, "0");
  return `${m}:${s}`;
}

// Deliberately does NOT show serial/batteries/CAR/FRK/wire colors/alien
// phrase/etc - all instance-specific bomb data the defuser must read off
// their own screen and describe out loud. This header only shows what an
// expert bystander could plausibly already know (elapsed time, strikes).
export function ExpertHeader({ view }: { view: DefuserView }) {
  const low = view.timer_ms <= 30000;
  return (
    <header className="status-bar">
      <div className={`timer ${low ? "is-low" : ""}`}>{fmt(view.timer_ms)}</div>

      <div className="strikes" title={`${view.strikes}/${view.max_strikes}`}>
        {Array.from({ length: view.max_strikes }).map((_, i) => (
          <span key={i} className={`strike ${i < view.strikes ? "is-hit" : ""}`}>
            ✕
          </span>
        ))}
      </div>
    </header>
  );
}
