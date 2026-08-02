import type { WiresModule } from "../../types";

export function Wires({ mod }: { mod: WiresModule }) {
  return (
    <div className="wire-list">
      {mod.slots.map((w) => (
        <div key={w.id} className={`wire-row ${w.cut ? "is-cut" : ""}`}>
          <span className="wire-index">{w.id + 1}</span>
          <span className="wire" data-color={w.color}>
            <span className="wire-line" />
            {w.cut && <span className="wire-gap" />}
          </span>
          <span className="wire-label">{w.color}</span>
        </div>
      ))}
    </div>
  );
}
