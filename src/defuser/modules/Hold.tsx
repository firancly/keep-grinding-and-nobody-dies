import type { HoldModule } from "../../types";

export function Hold({ mod }: { mod: HoldModule }) {
  return (
    <div className="hold">
      <button
        type="button"
        className={`hold-btn ${mod.holding ? "is-holding" : ""}`}
        data-color={mod.color}
        disabled
      >
        {mod.label}
      </button>
      <div className="hold-status">
        {mod.holding ? "HOLDING…" : "release to arm"}
      </div>
    </div>
  );
}
