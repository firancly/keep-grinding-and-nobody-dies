import type { ModuleState } from "../types";

const TITLES: Record<ModuleState["kind"], string> = {
  Wires: "Wires",
  Simon: "Simon",
  Memory: "Memory",
  Hold: "Button",
};

// A compact 4-step progress row. Shows WHICH module is active/solved/
// pending (so the expert knows which manual page applies) without
// revealing any of that module's actual puzzle content.
export function ModuleStepper({
  modules,
  activeIndex,
}: {
  modules: ModuleState[];
  activeIndex: number | null;
}) {
  return (
    <div className="module-stepper">
      {modules.map((mod, i) => {
        const state = mod.solved ? "done" : i === activeIndex ? "current" : "pending";
        return (
          <div key={i} className={`step step-${state}`}>
            <span className="step-dot">{mod.solved ? "✓" : i + 1}</span>
            <span className="step-label">{TITLES[mod.kind]}</span>
          </div>
        );
      })}
    </div>
  );
}
