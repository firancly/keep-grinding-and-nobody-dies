import type { ModuleState } from "../types";
import { Wires } from "./modules/Wires";
import { Simon } from "./modules/Simon";
import { Password } from "./modules/Password";
import { Hold } from "./modules/Hold";

const TITLES: Record<ModuleState["kind"], string> = {
  Wires: "Wires",
  Simon: "Simon Says",
  Password: "Password Memory",
  Hold: "Button Hold",
};

// Renders one puzzle. The `switch` on `kind` narrows the tagged union so each
// module component gets exactly its own fields.
export function ModuleCard({ mod }: { mod: ModuleState }) {
  return (
    <section className={`module-card ${mod.solved ? "is-solved" : ""}`}>
      <header className="module-head">
        <span className="module-title">{TITLES[mod.kind]}</span>
        <span className="module-status">{mod.solved ? "✓ SOLVED" : "ARMED"}</span>
      </header>
      <div className="module-body">{renderBody(mod)}</div>
    </section>
  );
}

function renderBody(mod: ModuleState) {
  switch (mod.kind) {
    case "Wires":
      return <Wires mod={mod} />;
    case "Simon":
      return <Simon mod={mod} />;
    case "Password":
      return <Password mod={mod} />;
    case "Hold":
      return <Hold mod={mod} />;
  }
}
