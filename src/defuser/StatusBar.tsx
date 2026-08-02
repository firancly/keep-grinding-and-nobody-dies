import type { DefuserView } from "../types";

function fmt(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = String(Math.floor(total / 60)).padStart(2, "0");
  const s = String(total % 60).padStart(2, "0");
  return `${m}:${s}`;
}

export function StatusBar({ view }: { view: DefuserView }) {
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

      <div className="badges">
        <span className="badge">SERIAL · {view.serial}</span>
        <span className="badge">TYPE · {view.bomb_type}</span>
        <span className="badge">PWR · {view.power}</span>
        <span className="badge">
          TEMP · {view.temp.band} ({view.temp.value}°)
        </span>
      </div>
    </header>
  );
}
