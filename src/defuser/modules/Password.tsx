import type { PasswordModule } from "../../types";

export function Password({ mod }: { mod: PasswordModule }) {
  const slots = Array.from({ length: mod.len });
  return (
    <div className="password">
      <div className="pw-slots">
        {slots.map((_, i) => {
          const active = i === mod.position;
          return (
            <div key={i} className={`pw-slot ${active ? "is-active" : ""}`}>
              {active && <span className="pw-arrow">▲</span>}
              <span className="pw-char">{active ? mod.current_char : "•"}</span>
              {active && <span className="pw-arrow">▼</span>}
            </div>
          );
        })}
      </div>
    </div>
  );
}
